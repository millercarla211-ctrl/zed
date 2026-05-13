use super::{activation::FlowActivationProfile, always_on::FlowAlwaysOnProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowRuntimeState {
    ColdBoot,
    Listening,
    Overlay,
    Dictating,
    CommandMode,
    Sleeping,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowRuntimeEvent {
    BootCompleted,
    WakeWordDetected(String),
    ToggleShortcutPressed,
    HoldToDictateStarted,
    HoldToDictateReleased,
    CommandModeRequested,
    OverlayDismissed,
    InactivityTimeout,
    LowBattery,
    ThermalBackoff,
    ResumeRequested,
    PauseRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowLifecycleSnapshot {
    pub state: FlowRuntimeState,
    pub microphone_hot: bool,
    pub overlay_visible: bool,
    pub command_mode: bool,
    pub last_trigger: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowLifecycleController;

impl FlowLifecycleController {
    pub fn initial_snapshot(
        &self,
        activation: &FlowActivationProfile,
        always_on: &FlowAlwaysOnProfile,
    ) -> FlowLifecycleSnapshot {
        FlowLifecycleSnapshot {
            state: if activation.allow_background_detection {
                FlowRuntimeState::Listening
            } else {
                FlowRuntimeState::ColdBoot
            },
            microphone_hot: activation.keep_microphone_hot
                || always_on.features.wake_word_detection,
            overlay_visible: false,
            command_mode: false,
            last_trigger: None,
        }
    }

    pub fn transition(
        &self,
        snapshot: &FlowLifecycleSnapshot,
        event: FlowRuntimeEvent,
    ) -> FlowLifecycleSnapshot {
        match event {
            FlowRuntimeEvent::BootCompleted | FlowRuntimeEvent::ResumeRequested => {
                FlowLifecycleSnapshot {
                    state: FlowRuntimeState::Listening,
                    microphone_hot: true,
                    overlay_visible: false,
                    command_mode: false,
                    last_trigger: snapshot.last_trigger.clone(),
                }
            }
            FlowRuntimeEvent::WakeWordDetected(trigger) => FlowLifecycleSnapshot {
                state: FlowRuntimeState::Overlay,
                microphone_hot: true,
                overlay_visible: true,
                command_mode: false,
                last_trigger: Some(trigger),
            },
            FlowRuntimeEvent::ToggleShortcutPressed => {
                let overlay_visible = !snapshot.overlay_visible;
                FlowLifecycleSnapshot {
                    state: if overlay_visible {
                        FlowRuntimeState::Overlay
                    } else {
                        FlowRuntimeState::Listening
                    },
                    microphone_hot: true,
                    overlay_visible,
                    command_mode: false,
                    last_trigger: Some("keyboard-shortcut".to_string()),
                }
            }
            FlowRuntimeEvent::HoldToDictateStarted => FlowLifecycleSnapshot {
                state: FlowRuntimeState::Dictating,
                microphone_hot: true,
                overlay_visible: true,
                command_mode: false,
                last_trigger: Some("hold-to-dictate".to_string()),
            },
            FlowRuntimeEvent::HoldToDictateReleased | FlowRuntimeEvent::OverlayDismissed => {
                FlowLifecycleSnapshot {
                    state: FlowRuntimeState::Listening,
                    microphone_hot: true,
                    overlay_visible: false,
                    command_mode: false,
                    last_trigger: snapshot.last_trigger.clone(),
                }
            }
            FlowRuntimeEvent::CommandModeRequested => FlowLifecycleSnapshot {
                state: FlowRuntimeState::CommandMode,
                microphone_hot: true,
                overlay_visible: true,
                command_mode: true,
                last_trigger: Some("command-mode".to_string()),
            },
            FlowRuntimeEvent::InactivityTimeout => FlowLifecycleSnapshot {
                state: FlowRuntimeState::Listening,
                microphone_hot: snapshot.microphone_hot,
                overlay_visible: false,
                command_mode: false,
                last_trigger: snapshot.last_trigger.clone(),
            },
            FlowRuntimeEvent::LowBattery | FlowRuntimeEvent::ThermalBackoff => {
                FlowLifecycleSnapshot {
                    state: FlowRuntimeState::Paused,
                    microphone_hot: false,
                    overlay_visible: false,
                    command_mode: false,
                    last_trigger: snapshot.last_trigger.clone(),
                }
            }
            FlowRuntimeEvent::PauseRequested => FlowLifecycleSnapshot {
                state: FlowRuntimeState::Sleeping,
                microphone_hot: false,
                overlay_visible: false,
                command_mode: false,
                last_trigger: snapshot.last_trigger.clone(),
            },
        }
    }
}
