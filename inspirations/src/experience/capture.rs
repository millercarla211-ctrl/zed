use super::audio::FlowAudioPipeline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureBackend {
    Cpal,
    Stub,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureFrameReport {
    pub frame_samples: usize,
    pub rms_milli: u16,
    pub speech_detected: bool,
    pub clipping_detected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureWorkerStatus {
    pub backend: CaptureBackend,
    pub configured: bool,
    pub running: bool,
    pub device_name: Option<String>,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub last_error: Option<String>,
    pub frames_seen: u64,
    pub speech_frames: u64,
    pub last_rms_milli: u16,
    pub last_clipping_detected: bool,
    pub speech_gate_milli: u16,
}

pub trait FlowCaptureWorker {
    fn configure(&mut self, pipeline: &FlowAudioPipeline);
    fn start(&mut self);
    fn stop(&mut self);
    fn feed_samples(&mut self, samples: &[f32]) -> CaptureFrameReport;
    fn status(&self) -> CaptureWorkerStatus;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpalCaptureWorker {
    pub dry_run: bool,
    status: CaptureWorkerStatus,
}

impl Default for CpalCaptureWorker {
    fn default() -> Self {
        Self {
            dry_run: true,
            status: CaptureWorkerStatus {
                backend: CaptureBackend::Cpal,
                configured: false,
                running: false,
                device_name: None,
                sample_rate_hz: 0,
                channels: 0,
                last_error: None,
                frames_seen: 0,
                speech_frames: 0,
                last_rms_milli: 0,
                last_clipping_detected: false,
                speech_gate_milli: 18,
            },
        }
    }
}

impl CpalCaptureWorker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn live() -> Self {
        let mut worker = Self::default();
        worker.dry_run = false;
        worker
    }

    fn gate_for_pipeline(pipeline: &FlowAudioPipeline) -> u16 {
        if pipeline.low_power_mode {
            12
        } else if pipeline.dictation.streaming {
            16
        } else {
            20
        }
    }
}

impl FlowCaptureWorker for CpalCaptureWorker {
    fn configure(&mut self, pipeline: &FlowAudioPipeline) {
        self.status.sample_rate_hz = pipeline.sample_rate_hz;
        self.status.channels = pipeline.channels as u16;
        self.status.configured = true;
        self.status.last_error = None;
        self.status.speech_gate_milli = Self::gate_for_pipeline(pipeline);

        if self.dry_run {
            self.status.device_name = Some("dry-run-default-input".to_string());
            return;
        }

        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(device) => {
                self.status.device_name = device
                    .description()
                    .ok()
                    .map(|description| description.name().to_string());
                if let Ok(config) = device.default_input_config() {
                    self.status.sample_rate_hz = config.sample_rate();
                    self.status.channels = config.channels();
                }
            }
            None => {
                self.status.last_error = Some("No default input device found.".to_string());
            }
        }
    }

    fn start(&mut self) {
        if self.status.configured && self.status.last_error.is_none() {
            self.status.running = true;
        }
    }

    fn stop(&mut self) {
        self.status.running = false;
    }

    fn feed_samples(&mut self, samples: &[f32]) -> CaptureFrameReport {
        let rms_milli = rms_milli(samples);
        let clipping_detected = samples.iter().any(|sample| sample.abs() >= 0.98);
        let speech_detected = rms_milli >= self.status.speech_gate_milli;

        self.status.frames_seen += 1;
        if speech_detected {
            self.status.speech_frames += 1;
        }
        self.status.last_rms_milli = rms_milli;
        self.status.last_clipping_detected = clipping_detected;

        CaptureFrameReport {
            frame_samples: samples.len(),
            rms_milli,
            speech_detected,
            clipping_detected,
        }
    }

    fn status(&self) -> CaptureWorkerStatus {
        self.status.clone()
    }
}

fn rms_milli(samples: &[f32]) -> u16 {
    if samples.is_empty() {
        return 0;
    }

    let sum: f32 = samples.iter().map(|sample| sample * sample).sum();
    let rms = (sum / samples.len() as f32).sqrt();
    (rms.clamp(0.0, 1.0) * 1000.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_worker_reports_speech_when_energy_is_high() {
        let mut worker = CpalCaptureWorker::default();
        worker.start();
        let report = worker.feed_samples(&vec![0.4; 320]);
        assert!(report.speech_detected);
        assert!(worker.status().speech_frames >= 1);
    }

    #[test]
    fn capture_worker_reports_silence_when_energy_is_low() {
        let mut worker = CpalCaptureWorker::default();
        worker.start();
        let report = worker.feed_samples(&vec![0.0; 320]);
        assert!(!report.speech_detected);
        assert_eq!(worker.status().speech_frames, 0);
    }
}
