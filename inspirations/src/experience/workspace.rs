use super::types::{
    AppContext, AppUsageStat, DictionaryEntry, ExpandedSnippet, FlowWorkspaceProfile, SnippetEntry,
    StylePreset, TypingAssistResult, UsageDashboardSnapshot, WritingDomain,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowExperienceHub {
    profile: FlowWorkspaceProfile,
}

impl FlowExperienceHub {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            profile: FlowWorkspaceProfile {
                name: name.into(),
                personal_dictionary: Vec::new(),
                shared_dictionary: Vec::new(),
                personal_snippets: Vec::new(),
                shared_snippets: Vec::new(),
                styles: Vec::new(),
                preferred_languages: vec!["en".to_string()],
                whisper_mode: false,
                usage: UsageDashboardSnapshot {
                    total_typing_sessions: 0,
                    total_dictation_sessions: 0,
                    total_snippet_expansions: 0,
                    total_dictionary_normalizations: 0,
                    per_app: Vec::new(),
                },
            },
        }
    }

    pub fn profile(&self) -> &FlowWorkspaceProfile {
        &self.profile
    }

    pub fn into_profile(self) -> FlowWorkspaceProfile {
        self.profile
    }

    pub fn add_personal_dictionary_entry(&mut self, surface: &str, canonical: &str) {
        self.profile.personal_dictionary.push(DictionaryEntry {
            surface: surface.to_string(),
            canonical: canonical.to_string(),
            case_sensitive: false,
            shared: false,
        });
    }

    pub fn add_shared_dictionary_entry(&mut self, surface: &str, canonical: &str) {
        self.profile.shared_dictionary.push(DictionaryEntry {
            surface: surface.to_string(),
            canonical: canonical.to_string(),
            case_sensitive: false,
            shared: true,
        });
    }

    pub fn add_personal_snippet(&mut self, trigger: &str, expansion: &str) {
        self.profile.personal_snippets.push(SnippetEntry {
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
            shared: false,
            description: None,
        });
    }

    pub fn add_shared_snippet(&mut self, trigger: &str, expansion: &str) {
        self.profile.shared_snippets.push(SnippetEntry {
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
            shared: true,
            description: None,
        });
    }

    pub fn add_style(&mut self, style: StylePreset) {
        self.profile.styles.push(style);
    }

    pub fn set_whisper_mode(&mut self, enabled: bool) {
        self.profile.whisper_mode = enabled;
    }

    pub fn dictionary_for_context(&self) -> Vec<DictionaryEntry> {
        let mut dictionary = self.profile.shared_dictionary.clone();
        dictionary.extend(self.profile.personal_dictionary.clone());
        dictionary
    }

    pub fn snippets_for_context(&self) -> Vec<SnippetEntry> {
        let mut snippets = self.profile.shared_snippets.clone();
        snippets.extend(self.profile.personal_snippets.clone());
        snippets
    }

    pub fn styles_for_context(&self, app_context: &AppContext) -> Vec<StylePreset> {
        self.profile
            .styles
            .iter()
            .filter(|style| {
                style.domain == app_context.domain || style.domain == WritingDomain::General
            })
            .cloned()
            .collect()
    }

    pub fn record_typing_session(&mut self, app_name: &str, result: &TypingAssistResult) {
        self.profile.usage.total_typing_sessions += 1;
        self.profile.usage.total_snippet_expansions += result.expanded_snippets.len() as u64;
        self.profile.usage.total_dictionary_normalizations += result.normalized_terms.len() as u64;
        bump_app(&mut self.profile.usage.per_app, app_name);
    }

    pub fn record_dictation_session(
        &mut self,
        app_name: &str,
        snippets: &[ExpandedSnippet],
        normalized_terms: &[String],
    ) {
        self.profile.usage.total_dictation_sessions += 1;
        self.profile.usage.total_snippet_expansions += snippets.len() as u64;
        self.profile.usage.total_dictionary_normalizations += normalized_terms.len() as u64;
        bump_app(&mut self.profile.usage.per_app, app_name);
    }
}

impl Default for FlowExperienceHub {
    fn default() -> Self {
        Self::new("default")
    }
}

fn bump_app(per_app: &mut Vec<AppUsageStat>, app_name: &str) {
    if let Some(existing) = per_app.iter_mut().find(|item| item.app_name == app_name) {
        existing.events += 1;
        return;
    }

    per_app.push(AppUsageStat {
        app_name: app_name.to_string(),
        events: 1,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::{FlowTypingAssistant, ToneStyle, TypingAssistRequest};

    #[test]
    fn experience_hub_combines_personal_and_shared_assets() {
        let mut hub = FlowExperienceHub::new("test");
        hub.add_personal_dictionary_entry("supabase", "Supabase");
        hub.add_shared_snippet("addr", "221B Baker Street");
        hub.add_style(StylePreset {
            name: "general".to_string(),
            domain: WritingDomain::General,
            tone: ToneStyle::Professional,
            rules: Vec::new(),
        });

        assert_eq!(hub.dictionary_for_context().len(), 1);
        assert_eq!(hub.snippets_for_context().len(), 1);
        assert_eq!(
            hub.styles_for_context(&AppContext {
                app_name: "Mail".to_string(),
                window_title: None,
                url: None,
                language: Some("en".to_string()),
                domain: WritingDomain::General,
                workspace_files: Vec::new(),
                team_terms: Vec::new(),
            })
            .len(),
            1
        );
    }

    #[test]
    fn experience_hub_tracks_usage() {
        let assistant = FlowTypingAssistant::new();
        let mut hub = FlowExperienceHub::new("test");
        hub.add_shared_snippet("addr", "221B Baker Street");

        let result = assistant
            .process(TypingAssistRequest {
                text: "addr".to_string(),
                app_context: AppContext {
                    app_name: "Mail".to_string(),
                    window_title: None,
                    url: None,
                    language: Some("en".to_string()),
                    domain: WritingDomain::General,
                    workspace_files: Vec::new(),
                    team_terms: Vec::new(),
                },
                dictionary: Vec::new(),
                snippets: hub.snippets_for_context(),
                styles: Vec::new(),
                auto_correct: false,
                expand_snippets: true,
            })
            .unwrap();

        hub.record_typing_session("Mail", &result);
        assert_eq!(hub.profile().usage.total_typing_sessions, 1);
        assert_eq!(hub.profile().usage.total_snippet_expansions, 1);
    }
}
