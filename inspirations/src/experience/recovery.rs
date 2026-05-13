#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryEvent {
    Suspend,
    Resume,
    MicrophoneLost,
    MicrophoneRestored,
    RuntimeCrash,
    ThermalPause,
    BatteryFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    pub title: &'static str,
    pub persist_state: bool,
    pub restart_audio: bool,
    pub reload_modules: bool,
    pub reset_overlay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowRecoveryPlan {
    pub event: RecoveryEvent,
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowRecoveryPlanner;

impl FlowRecoveryPlanner {
    pub fn plan(event: RecoveryEvent) -> FlowRecoveryPlan {
        let actions = match event {
            RecoveryEvent::Suspend => vec![RecoveryAction {
                title: "Persist state before suspend",
                persist_state: true,
                restart_audio: false,
                reload_modules: false,
                reset_overlay: true,
            }],
            RecoveryEvent::Resume => vec![
                RecoveryAction {
                    title: "Reload audio pipeline after resume",
                    persist_state: false,
                    restart_audio: true,
                    reload_modules: false,
                    reset_overlay: true,
                },
                RecoveryAction {
                    title: "Return Flow to listening mode",
                    persist_state: false,
                    restart_audio: false,
                    reload_modules: false,
                    reset_overlay: false,
                },
            ],
            RecoveryEvent::MicrophoneLost => vec![RecoveryAction {
                title: "Pause dictation and wait for microphone recovery",
                persist_state: false,
                restart_audio: false,
                reload_modules: false,
                reset_overlay: false,
            }],
            RecoveryEvent::MicrophoneRestored => vec![RecoveryAction {
                title: "Restart wake and dictation audio services",
                persist_state: false,
                restart_audio: true,
                reload_modules: false,
                reset_overlay: false,
            }],
            RecoveryEvent::RuntimeCrash => vec![
                RecoveryAction {
                    title: "Persist crash-safe state",
                    persist_state: true,
                    restart_audio: false,
                    reload_modules: false,
                    reset_overlay: true,
                },
                RecoveryAction {
                    title: "Reload resident modules and resume listening",
                    persist_state: false,
                    restart_audio: true,
                    reload_modules: true,
                    reset_overlay: true,
                },
            ],
            RecoveryEvent::ThermalPause | RecoveryEvent::BatteryFallback => vec![RecoveryAction {
                title: "Drop to low-end resident profile",
                persist_state: true,
                restart_audio: false,
                reload_modules: true,
                reset_overlay: true,
            }],
        };

        FlowRecoveryPlan { event, actions }
    }
}
