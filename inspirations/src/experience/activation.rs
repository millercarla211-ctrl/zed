use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationSource {
    WakeWord,
    KeyboardShortcut,
    ManualOpen,
    Programmatic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationMode {
    Toggle,
    PushToTalk,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardShortcut {
    pub modifiers: Vec<&'static str>,
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeAlias {
    pub phrase: &'static str,
    pub confidence_floor: f32,
    pub allow_partial_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeModelSource {
    pub label: &'static str,
    pub relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowActivationProfile {
    pub mode: ActivationMode,
    pub wake_aliases: Vec<WakeAlias>,
    pub model_sources: Vec<WakeModelSource>,
    pub primary_toggle: KeyboardShortcut,
    pub hold_to_dictate: KeyboardShortcut,
    pub overlay_toggle: KeyboardShortcut,
    pub debounce_ms: u64,
    pub idle_timeout_secs: u64,
    pub keep_microphone_hot: bool,
    pub allow_background_detection: bool,
}

impl FlowActivationProfile {
    pub fn low_end_default() -> Self {
        Self {
            mode: ActivationMode::Hybrid,
            wake_aliases: vec![
                WakeAlias {
                    phrase: "dx",
                    confidence_floor: 0.68,
                    allow_partial_match: false,
                },
                WakeAlias {
                    phrase: "friday",
                    confidence_floor: 0.68,
                    allow_partial_match: true,
                },
                WakeAlias {
                    phrase: "hello",
                    confidence_floor: 0.68,
                    allow_partial_match: true,
                },
                WakeAlias {
                    phrase: "aladdin",
                    confidence_floor: 0.68,
                    allow_partial_match: true,
                },
                WakeAlias {
                    phrase: "arise",
                    confidence_floor: 0.68,
                    allow_partial_match: true,
                },
            ],
            model_sources: vec![WakeModelSource {
                label: "wakeword-models",
                relative_path: PathBuf::from("models/wake_words"),
            }],
            primary_toggle: KeyboardShortcut {
                modifiers: vec!["Ctrl", "Alt"],
                key: "Space",
                label: "Toggle Flow",
            },
            hold_to_dictate: KeyboardShortcut {
                modifiers: vec!["Ctrl", "Shift"],
                key: "Space",
                label: "Hold To Dictate",
            },
            overlay_toggle: KeyboardShortcut {
                modifiers: vec!["Alt"],
                key: "`",
                label: "Quick Overlay",
            },
            debounce_ms: 1_500,
            idle_timeout_secs: 18,
            keep_microphone_hot: true,
            allow_background_detection: true,
        }
    }

    pub fn desktop_power_user() -> Self {
        let mut profile = Self::low_end_default();
        profile.idle_timeout_secs = 30;
        profile.primary_toggle = KeyboardShortcut {
            modifiers: vec!["Ctrl", "Alt", "Shift"],
            key: "Space",
            label: "Toggle Flow Pro",
        };
        profile
    }

    pub fn wake_phrases(&self) -> Vec<&'static str> {
        self.wake_aliases.iter().map(|alias| alias.phrase).collect()
    }
}
