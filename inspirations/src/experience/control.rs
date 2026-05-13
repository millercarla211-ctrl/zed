#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlCapability {
    ReadClipboard,
    WriteClipboard,
    ReadSelection,
    ReplaceSelection,
    SimulateShortcut,
    OpenUrl,
    OpenApplication,
    OpenFile,
    RevealFile,
    CreateDraftFile,
    FocusWindow,
    MediaPlayback,
    VolumeControl,
    BrightnessControl,
    SystemSearch,
    Notification,
    ShellCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlSurface {
    Desktop,
    Mobile,
    Browser,
    EditorEmbed,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyRequirement {
    Silent,
    Confirm,
    ExplicitConsent,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRule {
    pub capability: ControlCapability,
    pub safety: SafetyRequirement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlActionPlan {
    pub capability: ControlCapability,
    pub description: String,
    pub requires_user_confirmation: bool,
    pub suggested_payload: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowControlPolicy {
    pub surface: ControlSurface,
    pub rules: Vec<CapabilityRule>,
    pub allow_background_automation: bool,
    pub audit_every_action: bool,
}

impl FlowControlPolicy {
    pub fn desktop_default() -> Self {
        Self {
            surface: ControlSurface::Desktop,
            rules: vec![
                CapabilityRule {
                    capability: ControlCapability::ReadClipboard,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::WriteClipboard,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::ReadSelection,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::ReplaceSelection,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::SimulateShortcut,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::OpenUrl,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::OpenApplication,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::OpenFile,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::RevealFile,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::CreateDraftFile,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::FocusWindow,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::MediaPlayback,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::VolumeControl,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::BrightnessControl,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::SystemSearch,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::Notification,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::ShellCommand,
                    safety: SafetyRequirement::ExplicitConsent,
                },
            ],
            allow_background_automation: false,
            audit_every_action: true,
        }
    }

    pub fn mobile_default() -> Self {
        let mut policy = Self::desktop_default();
        policy.surface = ControlSurface::Mobile;
        policy.allow_background_automation = false;
        policy
    }

    pub fn editor_embedded() -> Self {
        Self {
            surface: ControlSurface::EditorEmbed,
            rules: vec![
                CapabilityRule {
                    capability: ControlCapability::ReadSelection,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::ReplaceSelection,
                    safety: SafetyRequirement::Silent,
                },
                CapabilityRule {
                    capability: ControlCapability::OpenFile,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::RevealFile,
                    safety: SafetyRequirement::Confirm,
                },
                CapabilityRule {
                    capability: ControlCapability::SystemSearch,
                    safety: SafetyRequirement::Silent,
                },
            ],
            allow_background_automation: false,
            audit_every_action: true,
        }
    }

    pub fn plan_text_insert(&self, rewritten_text: impl Into<String>) -> ControlActionPlan {
        let rewritten_text = rewritten_text.into();
        let safety = self.safety_for(&ControlCapability::ReplaceSelection);
        ControlActionPlan {
            capability: ControlCapability::ReplaceSelection,
            description: "Replace the current selection with Flow output.".to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(rewritten_text),
        }
    }

    pub fn plan_open_url(&self, url: impl Into<String>) -> ControlActionPlan {
        let url = url.into();
        let safety = self.safety_for(&ControlCapability::OpenUrl);
        ControlActionPlan {
            capability: ControlCapability::OpenUrl,
            description: "Open a URL in the active system browser.".to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(url),
        }
    }

    pub fn plan_launch_app(&self, app_id: impl Into<String>) -> ControlActionPlan {
        let app_id = app_id.into();
        let safety = self.safety_for(&ControlCapability::OpenApplication);
        ControlActionPlan {
            capability: ControlCapability::OpenApplication,
            description: "Launch or focus an application for the current task.".to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(app_id),
        }
    }

    pub fn plan_system_search(&self, query: impl Into<String>) -> ControlActionPlan {
        let query = query.into();
        let safety = self.safety_for(&ControlCapability::SystemSearch);
        ControlActionPlan {
            capability: ControlCapability::SystemSearch,
            description: "Open system search with the requested query.".to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(query),
        }
    }

    pub fn plan_shortcut(&self, shortcut: impl Into<String>) -> ControlActionPlan {
        let shortcut = shortcut.into();
        let safety = self.safety_for(&ControlCapability::SimulateShortcut);
        ControlActionPlan {
            capability: ControlCapability::SimulateShortcut,
            description: "Send a shortcut to the active application.".to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(shortcut),
        }
    }

    pub fn plan_shell_command(&self, command: impl Into<String>) -> ControlActionPlan {
        let command = command.into();
        let safety = self.safety_for(&ControlCapability::ShellCommand);
        ControlActionPlan {
            capability: ControlCapability::ShellCommand,
            description: "Execute a shell command only when the host grants explicit consent."
                .to_string(),
            requires_user_confirmation: requires_confirmation(&safety),
            suggested_payload: Some(command),
        }
    }

    fn safety_for(&self, capability: &ControlCapability) -> SafetyRequirement {
        self.rules
            .iter()
            .find(|rule| &rule.capability == capability)
            .map(|rule| rule.safety.clone())
            .unwrap_or(SafetyRequirement::Deny)
    }
}

fn requires_confirmation(safety: &SafetyRequirement) -> bool {
    !matches!(safety, SafetyRequirement::Silent)
}
