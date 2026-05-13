use std::process::Command;

use super::{
    audio::FlowAudioPipeline,
    contracts::{FlowAudioRuntime, FlowOverlayPresenter},
    modules::OperatingSystemFamily,
    overlay::{FlowOverlayState, OverlayMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOverlayPresenter {
    pub os: OperatingSystemFamily,
    pub dry_run: bool,
    pub last: Option<FlowOverlayState>,
}

impl NativeOverlayPresenter {
    pub fn new(os: OperatingSystemFamily) -> Self {
        Self {
            os,
            dry_run: true,
            last: None,
        }
    }

    pub fn live(os: OperatingSystemFamily) -> Self {
        Self {
            os,
            dry_run: false,
            last: None,
        }
    }
}

impl FlowOverlayPresenter for NativeOverlayPresenter {
    fn present(&mut self, overlay: &FlowOverlayState) {
        self.last = Some(overlay.clone());
        if self.dry_run || matches!(overlay.mode, OverlayMode::Hidden) {
            return;
        }

        let title = match overlay.mode {
            OverlayMode::Compact => "Flow Ready",
            OverlayMode::Dictation => "Flow Dictation",
            OverlayMode::CommandPalette => "Flow Command Mode",
            OverlayMode::RewritePreview => "Flow Rewrite Preview",
            OverlayMode::ProofingPanel => "Flow Proofing",
            OverlayMode::Hidden => "Flow",
        };

        let body = format!("{} quick actions available", overlay.quick_actions.len());

        match self.os {
            OperatingSystemFamily::Windows => {
                let _ = Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; [System.Windows.Forms.MessageBox]::Show('{}','{}')",
                            escape_single_quotes(&body),
                            escape_single_quotes(title)
                        ),
                    ])
                    .status();
            }
            OperatingSystemFamily::Macos => {
                let _ = Command::new("osascript")
                    .args([
                        "-e",
                        &format!(
                            "display notification '{}' with title '{}'",
                            escape_single_quotes(&body),
                            escape_single_quotes(title)
                        ),
                    ])
                    .status();
            }
            OperatingSystemFamily::Linux => {
                let _ = Command::new("notify-send").args([title, &body]).status();
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManagedAudioRuntime {
    pub last: Option<FlowAudioPipeline>,
    pub active: bool,
}

impl ManagedAudioRuntime {
    pub fn stop(&mut self) {
        self.active = false;
    }
}

impl FlowAudioRuntime for ManagedAudioRuntime {
    fn configure(&mut self, pipeline: &FlowAudioPipeline) {
        self.last = Some(pipeline.clone());
        self.active = true;
    }
}

fn escape_single_quotes(value: &str) -> String {
    value.replace('\'', "\\'")
}
