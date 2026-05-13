use super::audio::FlowAudioPipeline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MicrophoneMode {
    Stopped,
    Armed,
    Streaming,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrophoneSnapshot {
    pub mode: MicrophoneMode,
    pub low_power_mode: bool,
    pub configured: bool,
    pub restarts: u32,
    pub last_error: Option<String>,
}

pub trait FlowMicrophoneService {
    fn configure(&mut self, pipeline: &FlowAudioPipeline);
    fn arm(&mut self);
    fn stream(&mut self);
    fn pause(&mut self);
    fn stop(&mut self);
    fn restart(&mut self);
    fn snapshot(&self) -> MicrophoneSnapshot;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManagedMicrophoneService {
    pipeline: Option<FlowAudioPipeline>,
    snapshot: MicrophoneSnapshot,
}

impl Default for ManagedMicrophoneService {
    fn default() -> Self {
        Self {
            pipeline: None,
            snapshot: MicrophoneSnapshot {
                mode: MicrophoneMode::Stopped,
                low_power_mode: false,
                configured: false,
                restarts: 0,
                last_error: None,
            },
        }
    }
}

impl FlowMicrophoneService for ManagedMicrophoneService {
    fn configure(&mut self, pipeline: &FlowAudioPipeline) {
        self.pipeline = Some(pipeline.clone());
        self.snapshot.configured = true;
        self.snapshot.low_power_mode = pipeline.low_power_mode;
        if matches!(self.snapshot.mode, MicrophoneMode::Stopped) {
            self.snapshot.mode = MicrophoneMode::Armed;
        }
    }

    fn arm(&mut self) {
        self.snapshot.mode = MicrophoneMode::Armed;
    }

    fn stream(&mut self) {
        self.snapshot.mode = MicrophoneMode::Streaming;
    }

    fn pause(&mut self) {
        self.snapshot.mode = MicrophoneMode::Paused;
    }

    fn stop(&mut self) {
        self.snapshot.mode = MicrophoneMode::Stopped;
    }

    fn restart(&mut self) {
        self.snapshot.restarts += 1;
        self.snapshot.mode = MicrophoneMode::Armed;
    }

    fn snapshot(&self) -> MicrophoneSnapshot {
        self.snapshot.clone()
    }
}
