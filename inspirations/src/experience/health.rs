use super::{
    capture::FlowCaptureWorker, hostkit::FlowDefaultHostKit, installer::ModuleInstallStatus,
    microphone::FlowMicrophoneService, session::FlowSessionContext, wake::FlowWakeRuntime,
    wakedetect::FlowWakeInferenceWorker,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowHealthSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowHealthIssue {
    pub severity: FlowHealthSeverity,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowHealthReport {
    pub ready: bool,
    pub score: u8,
    pub issues: Vec<FlowHealthIssue>,
}

impl FlowHealthReport {
    pub fn evaluate(host: &FlowDefaultHostKit, context: &FlowSessionContext) -> Self {
        let mut issues = Vec::new();

        let mut pending_required = 0;
        let mut failed_required = 0;
        for record in context.install_state.records.values() {
            if record.descriptor.required {
                match record.status {
                    ModuleInstallStatus::Pending => pending_required += 1,
                    ModuleInstallStatus::Failed => failed_required += 1,
                    _ => {}
                }
            }
        }

        if failed_required > 0 {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Critical,
                title: "Required modules failed".to_string(),
                detail: format!(
                    "{} required module(s) are marked failed in the current install state.",
                    failed_required
                ),
            });
        }

        if pending_required > 0 {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Required modules still pending".to_string(),
                detail: format!(
                    "{} required module(s) are still pending installation.",
                    pending_required
                ),
            });
        }

        if !host.audio.active {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Audio runtime inactive".to_string(),
                detail: "The managed audio runtime is not marked active yet.".to_string(),
            });
        }

        let microphone = host.microphone.snapshot();
        if !microphone.configured {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Microphone service not configured".to_string(),
                detail: "The managed microphone service has not been configured from the audio pipeline yet."
                    .to_string(),
            });
        }

        if matches!(
            context.lifecycle.state,
            super::lifecycle::FlowRuntimeState::Listening
                | super::lifecycle::FlowRuntimeState::Dictating
                | super::lifecycle::FlowRuntimeState::CommandMode
        ) && matches!(
            microphone.mode,
            super::microphone::MicrophoneMode::Paused | super::microphone::MicrophoneMode::Stopped
        ) {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Microphone service not active".to_string(),
                detail: "The session expects live microphone service, but the managed microphone mode is paused or stopped."
                    .to_string(),
            });
        }

        let capture = host.capture.status();
        if !capture.configured {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Capture worker not configured".to_string(),
                detail: "The low-level capture worker has not been configured from the current audio pipeline."
                    .to_string(),
            });
        }

        if matches!(
            context.lifecycle.state,
            super::lifecycle::FlowRuntimeState::Listening
                | super::lifecycle::FlowRuntimeState::Dictating
                | super::lifecycle::FlowRuntimeState::CommandMode
        ) && !capture.running
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Capture worker not running".to_string(),
                detail: "The session expects active microphone capture, but the low-level capture worker is not running."
                    .to_string(),
            });
        }

        if matches!(
            context.lifecycle.state,
            super::lifecycle::FlowRuntimeState::Listening
                | super::lifecycle::FlowRuntimeState::Dictating
                | super::lifecycle::FlowRuntimeState::CommandMode
        ) && capture.frames_seen == 0
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Info,
                title: "No audio frames observed yet".to_string(),
                detail: "The host is configured for active capture, but no microphone frames have been fed into Flow yet."
                    .to_string(),
            });
        }

        if host.overlay.last.as_ref() != Some(&context.overlay) {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Info,
                title: "Overlay not yet synced".to_string(),
                detail: "The overlay presenter has not caught up with the current session overlay state."
                    .to_string(),
            });
        }

        let wake = host.wake.snapshot();
        let wake_worker = host.wake_worker.snapshot();
        if context.activation.allow_background_detection && !wake.armed {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Wake runtime not armed".to_string(),
                detail:
                    "Wake-word detection should be armed for this profile but is currently off."
                        .to_string(),
            });
        }

        if context.activation.allow_background_detection && !wake_worker.armed {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Wake inference worker not armed".to_string(),
                detail:
                    "Background detection is enabled but the wake inference worker is disarmed."
                        .to_string(),
            });
        }

        if context.activation.allow_background_detection && wake_worker.model_roots.is_empty() {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Wake model roots missing".to_string(),
                detail: "The wake inference worker has no configured model roots.".to_string(),
            });
        }

        if context.activation.allow_background_detection
            && wake_worker.configured_model_count == 0
            && !wake_worker.accepted_aliases.is_empty()
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "No local wake models matched".to_string(),
                detail: "Wake aliases are configured, but no matching local wake-word model files were discovered."
                    .to_string(),
            });
        }

        if context.activation.allow_background_detection
            && wake_worker.configured_model_count > 0
            && !wake_worker.detector_ready
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Wake detector fell back to alias mode".to_string(),
                detail: wake_worker.last_error.clone().unwrap_or_else(|| {
                    "The wake runtime could not initialize the local ONNX detector.".to_string()
                }),
            });
        }

        if matches!(
            context.lifecycle.state,
            super::lifecycle::FlowRuntimeState::Listening
        ) && !wake.listening
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Wake runtime not listening".to_string(),
                detail: "The session is in listening mode but the wake runtime is not listening."
                    .to_string(),
            });
        }

        if matches!(
            host.accessibility.mode,
            super::accessibility::AccessibilityMode::Disabled
        ) && matches!(
            host.snapshot.os,
            super::modules::OperatingSystemFamily::Windows
                | super::modules::OperatingSystemFamily::Macos
                | super::modules::OperatingSystemFamily::Linux
        ) {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Warning,
                title: "Desktop accessibility path disabled".to_string(),
                detail:
                    "A desktop host is running without an active accessibility automation path."
                        .to_string(),
            });
        }

        if host.snapshot.os.is_desktop()
            && !host.accessibility.is_full()
            && host.accessibility.available
        {
            issues.push(FlowHealthIssue {
                severity: FlowHealthSeverity::Info,
                title: "Desktop automation is using fallback mode".to_string(),
                detail: host
                    .accessibility
                    .notes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Clipboard fallback is active instead of a full accessibility automation path."
                        .to_string()),
            });
        }

        let score_penalty: u8 = issues
            .iter()
            .map(|issue| match issue.severity {
                FlowHealthSeverity::Critical => 30,
                FlowHealthSeverity::Warning => 12,
                FlowHealthSeverity::Info => 4,
            })
            .sum();
        let score = 100u8.saturating_sub(score_penalty);

        Self {
            ready: failed_required == 0 && pending_required == 0 && host.audio.active,
            score,
            issues,
        }
    }
}
