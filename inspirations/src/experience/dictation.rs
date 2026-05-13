use anyhow::Result;

use super::types::{
    AppContext, DictationAssistRequest, DictationAssistResult, DictionaryEntry, ExpandedSnippet,
    SnippetEntry, StylePreset, StyleRule, ToneStyle, WritingDomain,
};

pub struct FlowDictationEngine;

impl FlowDictationEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, request: DictationAssistRequest) -> Result<DictationAssistResult> {
        let raw_text = request.transcript.clone();
        let mut notes = Vec::new();

        let mut working = normalize_spaces(&request.transcript);

        if request.remove_fillers {
            working = remove_fillers(&working);
            working = remove_repeated_words(&working);
            notes.push("Removed filler words and repeated tokens.".to_string());
        }

        working = apply_backtracking(&working, &mut notes);

        let (expanded, expanded_snippets) = expand_snippets(&working, &request.snippets);
        working = expanded;

        let (normalized, normalized_terms) =
            normalize_dictionary_terms(&working, &request.dictionary);
        working = normalized;
        if !normalized_terms.is_empty() {
            notes.push("Applied personal and shared dictionary terms.".to_string());
        }

        working = apply_style_presets(working, &request.app_context, &request.styles, &mut notes);

        if request.format_lists {
            let formatted = format_spoken_numbered_list(&working);
            if formatted != working {
                notes.push("Formatted spoken numbered items into a list.".to_string());
                working = formatted;
            }
        }

        if request.auto_punctuate && request.app_context.domain != WritingDomain::Code {
            working = auto_punctuate(&working);
            notes.push("Applied punctuation heuristics.".to_string());
        }

        let file_tags = if request.tag_workspace_files {
            tag_workspace_files(&working, &request.app_context)
        } else {
            Vec::new()
        };
        if !file_tags.is_empty() {
            notes.push("Detected workspace file references in dictation.".to_string());
        }

        Ok(DictationAssistResult {
            raw_text,
            cleaned_text: working,
            file_tags,
            normalized_terms,
            expanded_snippets,
            notes,
        })
    }
}

impl Default for FlowDictationEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_fillers(text: &str) -> String {
    let fillers = [
        "um",
        "uh",
        "you know",
        "kind of",
        "sort of",
        "basically",
        "actually",
        "literally",
        "like",
    ];

    let mut working = format!(" {} ", text);
    for filler in fillers {
        let needle = format!(" {} ", filler);
        working = working.replace(&needle, " ");
    }
    normalize_spaces(working.trim())
}

fn remove_repeated_words(text: &str) -> String {
    let mut output = Vec::new();
    let mut last_lower = String::new();

    for token in text.split_whitespace() {
        let lower = token
            .trim_matches(|ch: char| !ch.is_alphanumeric())
            .to_ascii_lowercase();
        if !lower.is_empty() && lower == last_lower {
            continue;
        }
        last_lower = lower;
        output.push(token);
    }

    output.join(" ")
}

fn apply_backtracking(text: &str, notes: &mut Vec<String>) -> String {
    let markers = [" scratch that ", " actually ", " no, ", " no ", " rather "];
    let lower = text.to_ascii_lowercase();

    for marker in markers {
        if let Some(index) = lower.find(marker) {
            let prefix = text[..index].trim();
            let replacement = text[index + marker.len()..].trim();
            if replacement.is_empty() {
                continue;
            }

            let base = sentence_prefix(prefix);
            notes.push(format!(
                "Resolved a spoken backtracking correction using marker '{}'.",
                marker.trim()
            ));
            return if base.is_empty() {
                replacement.to_string()
            } else {
                format!("{} {}", base, replacement).trim().to_string()
            };
        }
    }

    text.to_string()
}

fn sentence_prefix(prefix: &str) -> String {
    if let Some(index) = prefix.rfind(['.', '!', '?', '\n']) {
        prefix[..=index].trim().to_string()
    } else {
        String::new()
    }
}

fn expand_snippets(text: &str, snippets: &[SnippetEntry]) -> (String, Vec<ExpandedSnippet>) {
    let mut working = text.to_string();
    let mut expanded = Vec::new();

    for snippet in snippets {
        if snippet.trigger.is_empty() {
            continue;
        }
        if working.contains(&snippet.trigger) {
            working = working.replace(&snippet.trigger, &snippet.expansion);
            expanded.push(ExpandedSnippet {
                trigger: snippet.trigger.clone(),
                expansion: snippet.expansion.clone(),
            });
        }
    }

    (working, expanded)
}

fn normalize_dictionary_terms(text: &str, dictionary: &[DictionaryEntry]) -> (String, Vec<String>) {
    let mut working = text.to_string();
    let mut normalized = Vec::new();

    for entry in dictionary {
        let changed = if entry.case_sensitive {
            let replaced = working.replace(&entry.surface, &entry.canonical);
            let changed = replaced != working;
            working = replaced;
            changed
        } else {
            replace_case_insensitive(&mut working, &entry.surface, &entry.canonical)
        };

        if changed {
            normalized.push(entry.canonical.clone());
        }
    }

    (working, normalized)
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

fn apply_style_presets(
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
            text = apply_rule(text, *rule);
        }
        notes.push(format!("Applied dictation style preset '{}'.", style.name));
    }
    text
}

fn apply_tone(text: String, tone: ToneStyle) -> String {
    match tone {
        ToneStyle::Natural => text,
        ToneStyle::Professional => capitalize_first(&ensure_punctuation(&text)),
        ToneStyle::Casual => text,
        ToneStyle::Concise => make_concise(&text),
        ToneStyle::Friendly => format!("{} {}", "Thanks,", ensure_punctuation(text.trim()))
            .trim()
            .to_string(),
        ToneStyle::Enthusiastic => {
            let cleaned = ensure_punctuation(&text).trim_end_matches('.').to_string();
            format!("{cleaned}!")
        }
        ToneStyle::Technical => preserve_technical_tokens(&text),
    }
}

fn apply_rule(text: String, rule: StyleRule) -> String {
    match rule {
        StyleRule::FormalityUp => capitalize_first(&ensure_punctuation(&text)),
        StyleRule::FormalityDown => text,
        StyleRule::Shorten => make_concise(&text),
        StyleRule::AddWarmth => format!("Thanks, {}", ensure_punctuation(text.trim())),
        StyleRule::PreserveSyntax => preserve_technical_tokens(&text),
        StyleRule::PreferBullets => format_spoken_numbered_list(&text),
        StyleRule::StrongPunctuation => ensure_punctuation(&text),
    }
}

fn make_concise(text: &str) -> String {
    let mut working = text.to_string();
    for filler in ["really ", "very ", "basically ", "just "] {
        working = working.replace(filler, "");
    }
    working.trim().to_string()
}

fn preserve_technical_tokens(text: &str) -> String {
    text.replace(" slash ", "/")
        .replace(" underscore ", "_")
        .replace(" colon colon ", "::")
        .replace(" dot ", ".")
}

fn auto_punctuate(text: &str) -> String {
    let working = capitalize_first(text.trim());
    ensure_punctuation(&working)
}

fn ensure_punctuation(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    if text.trim().ends_with(['.', '!', '?']) {
        text.trim().to_string()
    } else {
        format!("{}.", text.trim())
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn format_spoken_numbered_list(text: &str) -> String {
    let mut output = String::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        let prev = if index == 0 {
            None
        } else {
            Some(chars[index - 1])
        };
        let next = chars.get(index + 1).copied();

        if current.is_ascii_digit()
            && next == Some('.')
            && prev.is_some_and(|value| value.is_whitespace())
        {
            output.push('\n');
            output.push(current);
            output.push('.');
            index += 2;
            continue;
        }

        output.push(current);
        index += 1;
    }

    output.trim().to_string()
}

fn tag_workspace_files(text: &str, app_context: &AppContext) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut matches = Vec::new();

    for file in &app_context.workspace_files {
        let file_name = file
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(file)
            .to_ascii_lowercase();

        if !file_name.is_empty() && lower.contains(&file_name) {
            matches.push(file.clone());
        }
    }

    matches.sort();
    matches.dedup();
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> AppContext {
        AppContext {
            app_name: "Cursor".to_string(),
            window_title: None,
            url: None,
            language: Some("en".to_string()),
            domain: WritingDomain::Code,
            workspace_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
            team_terms: Vec::new(),
        }
    }

    #[test]
    fn dictation_engine_tags_workspace_files() {
        let engine = FlowDictationEngine::new();
        let result = engine
            .process(DictationAssistRequest {
                transcript: "please update main.rs and readme.md".to_string(),
                app_context: context(),
                dictionary: Vec::new(),
                snippets: Vec::new(),
                styles: Vec::new(),
                remove_fillers: true,
                auto_punctuate: false,
                format_lists: false,
                tag_workspace_files: true,
            })
            .unwrap();

        assert_eq!(result.file_tags.len(), 2);
    }
}
