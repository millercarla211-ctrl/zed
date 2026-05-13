use super::{
    activation::FlowActivationProfile, audio::FlowAudioPipeline, lifecycle::FlowRuntimeState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeRuntimeState {
    pub armed: bool,
    pub listening: bool,
    pub last_detection: Option<String>,
    pub low_power_mode: bool,
}

pub trait FlowWakeRuntime {
    fn configure(&mut self, activation: &FlowActivationProfile, audio: &FlowAudioPipeline);
    fn sync_lifecycle(&mut self, lifecycle: &FlowRuntimeState);
    fn note_detection(&mut self, phrase: impl Into<String>);
    fn snapshot(&self) -> WakeRuntimeState;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManagedWakeRuntime {
    activation: Option<FlowActivationProfile>,
    audio: Option<FlowAudioPipeline>,
    state: WakeRuntimeState,
}

impl Default for ManagedWakeRuntime {
    fn default() -> Self {
        Self {
            activation: None,
            audio: None,
            state: WakeRuntimeState {
                armed: false,
                listening: false,
                last_detection: None,
                low_power_mode: false,
            },
        }
    }
}

impl FlowWakeRuntime for ManagedWakeRuntime {
    fn configure(&mut self, activation: &FlowActivationProfile, audio: &FlowAudioPipeline) {
        self.activation = Some(activation.clone());
        self.audio = Some(audio.clone());
        self.state.armed = activation.allow_background_detection && audio.wake.keep_hot;
        self.state.listening = self.state.armed;
        self.state.low_power_mode = audio.low_power_mode;
    }

    fn sync_lifecycle(&mut self, lifecycle: &FlowRuntimeState) {
        self.state.listening = matches!(
            lifecycle,
            FlowRuntimeState::Listening
                | FlowRuntimeState::Overlay
                | FlowRuntimeState::Dictating
                | FlowRuntimeState::CommandMode
        ) && self.state.armed;
    }

    fn note_detection(&mut self, phrase: impl Into<String>) {
        self.state.last_detection = Some(phrase.into());
    }

    fn snapshot(&self) -> WakeRuntimeState {
        self.state.clone()
    }
}
