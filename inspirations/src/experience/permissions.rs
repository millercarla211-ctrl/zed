use super::modules::OperatingSystemFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionKind {
    Microphone,
    Accessibility,
    GlobalHotkeys,
    Overlay,
    Notifications,
    Clipboard,
    Automation,
    KeyboardExtension,
    BrowserMedia,
    BrowserClipboard,
    FileAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequirement {
    pub kind: PermissionKind,
    pub required: bool,
    pub title: &'static str,
    pub rationale: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowPermissionBundle {
    pub os: OperatingSystemFamily,
    pub required: Vec<PermissionRequirement>,
    pub optional: Vec<PermissionRequirement>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowPermissionPlanner;

impl FlowPermissionPlanner {
    pub fn build(os: OperatingSystemFamily) -> FlowPermissionBundle {
        let mut required = vec![PermissionRequirement {
            kind: PermissionKind::Microphone,
            required: true,
            title: "Microphone Access",
            rationale: "Flow needs microphone access for wake words and local dictation.",
        }];
        let mut optional = vec![
            PermissionRequirement {
                kind: PermissionKind::Notifications,
                required: false,
                title: "Notifications",
                rationale: "Allows Flow to show quick results, reminders, and recovery prompts.",
            },
            PermissionRequirement {
                kind: PermissionKind::Clipboard,
                required: false,
                title: "Clipboard Access",
                rationale: "Improves rewrite, copy, and paste flows across applications.",
            },
        ];

        match os {
            OperatingSystemFamily::Windows
            | OperatingSystemFamily::Macos
            | OperatingSystemFamily::Linux => {
                required.push(PermissionRequirement {
                    kind: PermissionKind::GlobalHotkeys,
                    required: true,
                    title: "Global Shortcuts",
                    rationale: "Needed for toggle and hold-to-dictate shortcuts across apps.",
                });
                required.push(PermissionRequirement {
                    kind: PermissionKind::Overlay,
                    required: true,
                    title: "Overlay Access",
                    rationale: "Needed to show the compact Flow overlay on top of active apps.",
                });
                required.push(PermissionRequirement {
                    kind: PermissionKind::Accessibility,
                    required: true,
                    title: "Accessibility Access",
                    rationale: "Needed for selection reading, replacement, and host control.",
                });
            }
            OperatingSystemFamily::Android | OperatingSystemFamily::Ios => {
                required.push(PermissionRequirement {
                    kind: PermissionKind::KeyboardExtension,
                    required: true,
                    title: "Keyboard Extension",
                    rationale: "Needed for system text entry and rewrite behavior on mobile.",
                });
                optional.push(PermissionRequirement {
                    kind: PermissionKind::Automation,
                    required: false,
                    title: "Automation Shortcuts",
                    rationale: "Lets Flow open actions through mobile shortcuts or intents.",
                });
            }
            OperatingSystemFamily::BrowserWasm => {
                required.push(PermissionRequirement {
                    kind: PermissionKind::BrowserMedia,
                    required: true,
                    title: "Browser Media Permissions",
                    rationale: "Needed for microphone use in browser-hosted Flow.",
                });
                optional.push(PermissionRequirement {
                    kind: PermissionKind::BrowserClipboard,
                    required: false,
                    title: "Browser Clipboard Access",
                    rationale: "Lets Flow copy and paste rewritten text in the browser.",
                });
            }
            OperatingSystemFamily::Server => {
                required.push(PermissionRequirement {
                    kind: PermissionKind::FileAccess,
                    required: true,
                    title: "State Storage Access",
                    rationale: "Needed for audit logs, module state, and benchmark history.",
                });
            }
        }

        FlowPermissionBundle {
            os,
            required,
            optional,
        }
    }
}
