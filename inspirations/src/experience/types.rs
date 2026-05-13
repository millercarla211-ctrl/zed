use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::writing::GrammarIssue;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum WritingDomain {
    General,
    Email,
    Chat,
    Docs,
    Support,
    Code,
    Academic,
    Marketing,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum ToneStyle {
    Natural,
    Professional,
    Casual,
    Concise,
    Friendly,
    Enthusiastic,
    Technical,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum StyleRule {
    FormalityUp,
    FormalityDown,
    Shorten,
    AddWarmth,
    PreserveSyntax,
    PreferBullets,
    StrongPunctuation,
}

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
pub struct AppContext {
    pub app_name: String,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub language: Option<String>,
    pub domain: WritingDomain,
    pub workspace_files: Vec<String>,
    pub team_terms: Vec<String>,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            app_name: "Flow".to_string(),
            window_title: None,
            url: None,
            language: Some("en".to_string()),
            domain: WritingDomain::General,
            workspace_files: Vec::new(),
            team_terms: Vec::new(),
        }
    }
}

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
pub struct DictionaryEntry {
    pub surface: String,
    pub canonical: String,
    pub case_sensitive: bool,
    pub shared: bool,
}

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
pub struct SnippetEntry {
    pub trigger: String,
    pub expansion: String,
    pub shared: bool,
    pub description: Option<String>,
}

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
pub struct StylePreset {
    pub name: String,
    pub domain: WritingDomain,
    pub tone: ToneStyle,
    pub rules: Vec<StyleRule>,
}

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
pub struct ExpandedSnippet {
    pub trigger: String,
    pub expansion: String,
}

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
pub struct TypingAssistRequest {
    pub text: String,
    pub app_context: AppContext,
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<SnippetEntry>,
    pub styles: Vec<StylePreset>,
    pub auto_correct: bool,
    pub expand_snippets: bool,
}

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
pub struct TypingAssistResult {
    pub original_text: String,
    pub final_text: String,
    pub issues: Vec<GrammarIssue>,
    pub expanded_snippets: Vec<ExpandedSnippet>,
    pub normalized_terms: Vec<String>,
    pub notes: Vec<String>,
}

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
pub struct DictationAssistRequest {
    pub transcript: String,
    pub app_context: AppContext,
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<SnippetEntry>,
    pub styles: Vec<StylePreset>,
    pub remove_fillers: bool,
    pub auto_punctuate: bool,
    pub format_lists: bool,
    pub tag_workspace_files: bool,
}

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
pub struct DictationAssistResult {
    pub raw_text: String,
    pub cleaned_text: String,
    pub file_tags: Vec<String>,
    pub normalized_terms: Vec<String>,
    pub expanded_snippets: Vec<ExpandedSnippet>,
    pub notes: Vec<String>,
}

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
pub struct TextCommandRequest {
    pub selected_text: String,
    pub command: String,
    pub app_context: AppContext,
    pub styles: Vec<StylePreset>,
}

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
pub struct TextCommandResult {
    pub original_text: String,
    pub transformed_text: String,
    pub applied_command: String,
    pub notes: Vec<String>,
}

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
pub struct AppUsageStat {
    pub app_name: String,
    pub events: u64,
}

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
pub struct UsageDashboardSnapshot {
    pub total_typing_sessions: u64,
    pub total_dictation_sessions: u64,
    pub total_snippet_expansions: u64,
    pub total_dictionary_normalizations: u64,
    pub per_app: Vec<AppUsageStat>,
}

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
pub struct FlowWorkspaceProfile {
    pub name: String,
    pub personal_dictionary: Vec<DictionaryEntry>,
    pub shared_dictionary: Vec<DictionaryEntry>,
    pub personal_snippets: Vec<SnippetEntry>,
    pub shared_snippets: Vec<SnippetEntry>,
    pub styles: Vec<StylePreset>,
    pub preferred_languages: Vec<String>,
    pub whisper_mode: bool,
    pub usage: UsageDashboardSnapshot,
}
