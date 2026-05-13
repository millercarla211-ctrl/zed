use anyhow::Result;

use crate::writing::HarperGrammarChecker;

use super::types::{
    AppContext, DictionaryEntry, ExpandedSnippet, SnippetEntry, StylePreset, StyleRule,
    TextCommandRequest, TextCommandResult, ToneStyle, TypingAssistRequest, TypingAssistResult,
    WritingDomain,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowTypingAssistant {
    grammar: HarperGrammarChecker,
}

impl FlowTypingAssistant {
    pub fn new() -> Self {
        Self {
            grammar: HarperGrammarChecker::new(),
        }
    }

    pub fn process(&self, request: TypingAssistRequest) -> Result<TypingAssistResult> {
        let original_text = request.text.clone();
        let mut notes = Vec::new();

        let mut working = normalize_whitespace(&request.text);

        let expanded_snippets = if request.expand_snippets {
            let (expanded, expansions) = expand_snippets(&working, &request.snippets);
            working = expanded;
            if !expansions.is_empty() {
                notes.push("Expanded personal or shared snippets.".to_string());
            }
            expansions
        } else {
            Vec::new()
        };

        let (normalized, terms) = normalize_dictionary_terms(&working, &request.dictionary);
        working = normalized;
        let normalized_terms = terms;
        if !normalized_terms.is_empty() {
            notes.push("Normalized dictionary and team terminology.".to_string());
        }

        if request.auto_correct && request.app_context.domain != WritingDomain::Code {
            working = self.grammar.correct(&working)?;
            notes.push("Applied local grammar correction.".to_string());
        }

        working = apply_styles(working, &request.app_context, &request.styles, &mut notes);
        working = apply_domain_touches(working, &request.app_context, &mut notes);

        let issues = if request.app_context.domain == WritingDomain::Code {
            Vec::new()
        } else {
            self.grammar.analyze(&working)?
        };

        Ok(TypingAssistResult {
            original_text,
            final_text: working,
            issues,
            expanded_snippets,
            normalized_terms,
            notes,
        })
    }

    pub fn execute_command(&self, request: TextCommandRequest) -> Result<TextCommandResult> {
        let command = request.command.trim().to_ascii_lowercase();
        let original_text = request.selected_text.clone();
        let mut notes = Vec::new();

        let transformed_text = if command.contains("professional") {
            notes.push("Applied professional rewrite heuristics.".to_string());
            professionalize(&request.selected_text)
        } else if command.contains("casual") || command.contains("friendlier") {
            notes.push("Applied casual rewrite heuristics.".to_string());
            make_casual(&request.selected_text)
        } else if command.contains("concise") || command.contains("shorter") {
            notes.push("Applied concise rewrite heuristics.".to_string());
            make_concise(&request.selected_text)
        } else if command.contains("bullet") || command.contains("list") {
            notes.push("Converted text into a bullet-style layout.".to_string());
            bullets_from_sentences(&request.selected_text)
        } else if command.contains("grammar") || command.contains("fix") {
            notes.push("Applied grammar correction.".to_string());
            self.grammar.correct(&request.selected_text)?
        } else {
            let fallback = apply_styles(
                request.selected_text.clone(),
                &request.app_context,
                &request.styles,
                &mut notes,
            );
            notes.push("Used style presets as the fallback command handler.".to_string());
            fallback
        };

        Ok(TextCommandResult {
            original_text,
            transformed_text,
            applied_command: request.command,
            notes,
        })
    }
}

impl Default for FlowTypingAssistant {
    fn default() -> Self {
        Self::new()
    }
}

fn expand_snippets(text: &str, snippets: &[SnippetEntry]) -> (String, Vec<ExpandedSnippet>) {
    let mut working = text.to_string();
    let mut applied = Vec::new();
    let mut ordered = snippets.to_vec();
    ordered.sort_by_key(|snippet| usize::MAX - snippet.trigger.len());

    for snippet in ordered {
        if snippet.trigger.trim().is_empty() {
            continue;
        }

        if working.contains(&snippet.trigger) {
            working = working.replace(&snippet.trigger, &snippet.expansion);
            applied.push(ExpandedSnippet {
                trigger: snippet.trigger,
                expansion: snippet.expansion,
            });
        }
    }

    (working, applied)
}

fn normalize_dictionary_terms(text: &str, dictionary: &[DictionaryEntry]) -> (String, Vec<String>) {
    let mut working = text.to_string();
    let mut normalized_terms = Vec::new();

    for entry in dictionary {
        if entry.surface.is_empty()
            || entry.canonical.is_empty()
            || entry.surface == entry.canonical
        {
            continue;
        }

        let changed = if entry.case_sensitive {
            let replaced = working.replace(&entry.surface, &entry.canonical);
            let changed = replaced != working;
            working = replaced;
            changed
        } else {
            replace_case_insensitive(&mut working, &entry.surface, &entry.canonical)
        };

        if changed {
            normalized_terms.push(entry.canonical.clone());
        }
    }

    (working, normalized_terms)
}

fn replace_case_insensitive(target: &mut String, needle: &str, replacement: &str) -> bool {
    let lower_target = target.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    if lower_needle.is_empty() || !lower_target.contains(&lower_needle) {
        return false;
    }

    let mut rebuilt = String::with_capacity(target.len());
    let mut index = 0;
    let mut changed = false;

    while let Some(found) = lower_target[index..].find(&lower_needle) {
        let absolute = index + found;
        rebuilt.push_str(&target[index..absolute]);
        rebuilt.push_str(replacement);
        index = absolute + needle.len();
        changed = true;
    }

    rebuilt.push_str(&target[index..]);
    *target = rebuilt;
    changed
}

fn apply_styles(
    mut text: String,
    app_context: &AppContext,
    styles: &[StylePreset],
    notes: &mut Vec<String>,
) -> String {
    for style in styles
        .iter()
        .filter(|style| style.domain == app_context.domain)
    {
        text = apply_tone(text, style.tone);
        for rule in &style.rules {
            text = apply_style_rule(text, *rule);
        }
        notes.push(format!("Applied style preset '{}'.", style.name));
    }
    text
}

fn apply_tone(text: String, tone: ToneStyle) -> String {
    match tone {
        ToneStyle::Natural => text,
        ToneStyle::Professional => professionalize(&text),
        ToneStyle::Casual => make_casual(&text),
        ToneStyle::Concise => make_concise(&text),
        ToneStyle::Friendly => add_warmth(&text),
        ToneStyle::Enthusiastic => make_enthusiastic(&text),
        ToneStyle::Technical => preserve_technical_text(&text),
    }
}

fn apply_style_rule(text: String, rule: StyleRule) -> String {
    match rule {
        StyleRule::FormalityUp => professionalize(&text),
        StyleRule::FormalityDown => make_casual(&text),
        StyleRule::Shorten => make_concise(&text),
        StyleRule::AddWarmth => add_warmth(&text),
        StyleRule::PreserveSyntax => preserve_technical_text(&text),
        StyleRule::PreferBullets => bullets_from_sentences(&text),
        StyleRule::StrongPunctuation => ensure_terminal_punctuation(&text),
    }
}

fn apply_domain_touches(
    mut text: String,
    app_context: &AppContext,
    notes: &mut Vec<String>,
) -> String {
    match app_context.domain {
        WritingDomain::Email => {
            text = capitalize_first(&text);
            text = ensure_terminal_punctuation(&text);
            notes.push("Applied email-friendly punctuation and capitalization.".to_string());
        }
        WritingDomain::Support => {
            text = add_warmth(&text);
            notes.push("Applied support-friendly warmth adjustments.".to_string());
        }
        WritingDomain::Code => {
            text = preserve_technical_text(&text);
            notes.push("Preserved syntax-sensitive code tokens.".to_string());
        }
        _ => {}
    }
    text
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn professionalize(text: &str) -> String {
    let mut result = text.to_string();
    for (from, to) in [
        (" can't ", " cannot "),
        (" won't ", " will not "),
        (" don't ", " do not "),
        (" i'm ", " I am "),
        (" i've ", " I have "),
        (" it's ", " it is "),
    ] {
        result = result.replace(from, to);
    }
    capitalize_first(&ensure_terminal_punctuation(&result))
}

fn make_casual(text: &str) -> String {
    let mut result = text.to_string();
    for (from, to) in [
        (" cannot ", " can't "),
        (" will not ", " won't "),
        (" do not ", " don't "),
        ("I am ", "I'm "),
        ("it is ", "it's "),
    ] {
        result = result.replace(from, to);
    }
    result
}

fn make_concise(text: &str) -> String {
    let mut result = text.to_string();
    for phrase in [
        "in order to ",
        "basically ",
        "actually ",
        "just ",
        "really ",
        "very ",
    ] {
        result = result.replace(phrase, "");
    }
    ensure_terminal_punctuation(result.trim())
}

fn add_warmth(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("Thanks") || trimmed.starts_with("Thank you") {
        return ensure_terminal_punctuation(trimmed);
    }
    format!("Thanks, {}", ensure_terminal_punctuation(trimmed))
}

fn make_enthusiastic(text: &str) -> String {
    let trimmed = capitalize_first(text.trim());
    if trimmed.ends_with('!') {
        trimmed
    } else {
        format!("{}!", trimmed.trim_end_matches('.'))
    }
}

fn preserve_technical_text(text: &str) -> String {
    text.replace(" dot ", ".")
        .replace(" slash ", "/")
        .replace(" underscore ", "_")
        .replace(" dash ", "-")
}

fn bullets_from_sentences(text: &str) -> String {
    let items = text
        .split(['.', ';', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {}", capitalize_first(item)))
        .collect::<Vec<_>>();

    if items.is_empty() {
        text.to_string()
    } else {
        items.join("\n")
    }
}

fn ensure_terminal_punctuation(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with(['.', '!', '?']) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(domain: WritingDomain) -> AppContext {
        AppContext {
            app_name: "Test".to_string(),
            window_title: None,
            url: None,
            language: Some("en".to_string()),
            domain,
            workspace_files: Vec::new(),
            team_terms: Vec::new(),
        }
    }

    #[test]
    fn typing_assistant_expands_snippets_and_dictionary() {
        let assistant = FlowTypingAssistant::new();
        let result = assistant
            .process(TypingAssistRequest {
                text: "addr and supabase".to_string(),
                app_context: context(WritingDomain::General),
                dictionary: vec![DictionaryEntry {
                    surface: "supabase".to_string(),
                    canonical: "Supabase".to_string(),
                    case_sensitive: false,
                    shared: true,
                }],
                snippets: vec![SnippetEntry {
                    trigger: "addr".to_string(),
                    expansion: "221B Baker Street".to_string(),
                    shared: false,
                    description: None,
                }],
                styles: Vec::new(),
                auto_correct: false,
                expand_snippets: true,
            })
            .unwrap();

        assert!(result.final_text.contains("221B Baker Street"));
        assert!(result.final_text.contains("Supabase"));
    }
}
