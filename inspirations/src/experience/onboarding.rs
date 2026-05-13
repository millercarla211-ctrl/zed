use super::modules::OperatingSystemFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnboardingStepKind {
    Permissions,
    Audio,
    Keyboard,
    Overlay,
    Accessibility,
    Privacy,
    ModelCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingStep {
    pub kind: OnboardingStepKind,
    pub title: &'static str,
    pub description: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowOnboardingPlan {
    pub os: OperatingSystemFamily,
    pub steps: Vec<OnboardingStep>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowOnboardingBuilder;

impl FlowOnboardingBuilder {
    pub fn build(os: OperatingSystemFamily) -> FlowOnboardingPlan {
        FlowOnboardingPlan {
            steps: base_steps(&os),
            os,
        }
    }
}

fn base_steps(os: &OperatingSystemFamily) -> Vec<OnboardingStep> {
    let mut steps = vec![
        OnboardingStep {
            kind: OnboardingStepKind::Audio,
            title: "Enable Microphone",
            description: "Allow Flow to listen for wake words and run local dictation.",
            required: true,
        },
        OnboardingStep {
            kind: OnboardingStepKind::Keyboard,
            title: "Choose Hotkeys",
            description: "Pick the toggle and hold-to-dictate shortcuts for Flow.",
            required: true,
        },
        OnboardingStep {
            kind: OnboardingStepKind::Privacy,
            title: "Review Privacy Mode",
            description: "Choose local-only mode, audit logging, and approval defaults.",
            required: true,
        },
        OnboardingStep {
            kind: OnboardingStepKind::ModelCache,
            title: "Prepare Local Modules",
            description: "Install the correct base module set for the current device tier.",
            required: true,
        },
    ];

    match os {
        OperatingSystemFamily::Windows
        | OperatingSystemFamily::Macos
        | OperatingSystemFamily::Linux => {
            steps.push(OnboardingStep {
                kind: OnboardingStepKind::Accessibility,
                title: "Enable Accessibility Access",
                description: "Grant text selection, replacement, and app-control permissions.",
                required: true,
            });
            steps.push(OnboardingStep {
                kind: OnboardingStepKind::Overlay,
                title: "Enable Overlay",
                description: "Allow the quick overlay for command mode and instant rewrites.",
                required: false,
            });
        }
        OperatingSystemFamily::Android | OperatingSystemFamily::Ios => {
            steps.push(OnboardingStep {
                kind: OnboardingStepKind::Permissions,
                title: "Enable Keyboard Extension",
                description: "Turn on the Flow keyboard or text input bridge for mobile use.",
                required: true,
            });
        }
        OperatingSystemFamily::BrowserWasm => {
            steps.push(OnboardingStep {
                kind: OnboardingStepKind::Permissions,
                title: "Grant Browser Permissions",
                description: "Approve microphone and clipboard access for browser-hosted Flow.",
                required: true,
            });
        }
        OperatingSystemFamily::Server => {
            steps.push(OnboardingStep {
                kind: OnboardingStepKind::Permissions,
                title: "Configure Daemon Policy",
                description: "Set audit, module, and runtime policies for headless operation.",
                required: true,
            });
        }
    }

    steps
}
