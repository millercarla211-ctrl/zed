use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use anyhow::Result;
use harper_core::linting::{LintGroup, Linter, Suggestion};
use harper_core::parsers::PlainEnglish;
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct GrammarIssue {
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub priority: u8,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarperGrammarChecker {
    dialect: Dialect,
}

impl HarperGrammarChecker {
    pub fn new() -> Self {
        Self {
            dialect: Dialect::American,
        }
    }

    pub fn analyze(&self, text: &str) -> Result<Vec<GrammarIssue>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let parser = PlainEnglish;
        let document = Document::new_curated(text, &parser);
        let dictionary = FstDictionary::curated();
        let mut linter = LintGroup::new_curated(dictionary, self.dialect);

        let mut lints = linter.lint(&document);
        harper_core::remove_overlaps(&mut lints);
        lints.sort_by_key(|lint| (lint.span.start, lint.priority));

        Ok(lints
            .into_iter()
            .map(|lint| GrammarIssue {
                start: lint.span.start,
                end: lint.span.end,
                message: lint.message,
                priority: lint.priority,
                replacement: lint.suggestions.first().and_then(stringify_suggestion),
            })
            .collect())
    }

    pub fn correct(&self, text: &str) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(text.to_string());
        }

        let parser = PlainEnglish;
        let document = Document::new_curated(text, &parser);
        let dictionary = FstDictionary::curated();
        let mut linter = LintGroup::new_curated(dictionary, self.dialect);
        let mut lints = linter.lint(&document);
        harper_core::remove_overlaps(&mut lints);

        let mut chars: Vec<char> = text.chars().collect();
        lints.sort_by(|left, right| right.span.start.cmp(&left.span.start));

        for lint in lints {
            if let Some(suggestion) = lint.suggestions.first() {
                suggestion.apply(lint.span, &mut chars);
            }
        }

        Ok(chars.into_iter().collect())
    }
}

impl Default for HarperGrammarChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn stringify_suggestion(suggestion: &Suggestion) -> Option<String> {
    if let Some(replacement) = suggestion.as_replace_with() {
        return Some(replacement.iter().collect());
    }

    suggestion
        .as_insert_after()
        .map(|replacement| replacement.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_checker_applies_basic_fix() {
        let checker = HarperGrammarChecker::new();
        let corrected = checker.correct("This is an test.").unwrap();
        assert!(corrected.contains("a test"));
    }
}
