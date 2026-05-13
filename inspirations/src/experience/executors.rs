use std::process::Command;

use super::{
    contracts::{ExecutedActionReceipt, FlowControlExecutor},
    control::{ControlActionPlan, ControlCapability},
    modules::OperatingSystemFamily,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInvocation {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeControlExecutor {
    pub os: OperatingSystemFamily,
    pub dry_run: bool,
}

impl NativeControlExecutor {
    pub fn new(os: OperatingSystemFamily) -> Self {
        Self { os, dry_run: true }
    }

    pub fn live(os: OperatingSystemFamily) -> Self {
        Self { os, dry_run: false }
    }

    fn plan(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        match self.os {
            OperatingSystemFamily::Windows => self.plan_windows(action),
            OperatingSystemFamily::Macos => self.plan_macos(action),
            OperatingSystemFamily::Linux => self.plan_linux(action),
            OperatingSystemFamily::Android => self.plan_android(action),
            OperatingSystemFamily::Ios => self.plan_ios(action),
            OperatingSystemFamily::BrowserWasm => None,
            OperatingSystemFamily::Server => self.plan_server(action),
        }
    }

    fn plan_windows(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        let payload = action.suggested_payload.clone().unwrap_or_default();

        match action.capability {
            ControlCapability::OpenUrl
            | ControlCapability::OpenApplication
            | ControlCapability::OpenFile
            | ControlCapability::RevealFile => Some(vec![PlatformInvocation {
                program: "cmd".to_string(),
                args: vec![
                    "/C".to_string(),
                    "start".to_string(),
                    "".to_string(),
                    payload,
                ],
            }]),
            ControlCapability::SystemSearch => Some(vec![PlatformInvocation {
                program: "explorer.exe".to_string(),
                args: vec![format!("search-ms:query={}", payload)],
            }]),
            ControlCapability::Notification => Some(vec![PlatformInvocation {
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    format!(
                        "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; [System.Windows.Forms.MessageBox]::Show('{}','Flow')",
                        escape_single_quotes(&payload)
                    ),
                ],
            }]),
            _ => None,
        }
    }

    fn plan_macos(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        let payload = action.suggested_payload.clone().unwrap_or_default();

        match action.capability {
            ControlCapability::OpenUrl
            | ControlCapability::OpenApplication
            | ControlCapability::OpenFile
            | ControlCapability::RevealFile => Some(vec![PlatformInvocation {
                program: "open".to_string(),
                args: vec![payload],
            }]),
            ControlCapability::Notification => Some(vec![PlatformInvocation {
                program: "osascript".to_string(),
                args: vec![
                    "-e".to_string(),
                    format!(
                        "display notification '{}' with title 'Flow'",
                        escape_single_quotes(&payload)
                    ),
                ],
            }]),
            ControlCapability::MediaPlayback => Some(vec![PlatformInvocation {
                program: "osascript".to_string(),
                args: vec![
                    "-e".to_string(),
                    "tell application \"Music\" to playpause".to_string(),
                ],
            }]),
            _ => None,
        }
    }

    fn plan_linux(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        let payload = action.suggested_payload.clone().unwrap_or_default();

        match action.capability {
            ControlCapability::OpenUrl
            | ControlCapability::OpenApplication
            | ControlCapability::OpenFile
            | ControlCapability::RevealFile => Some(vec![PlatformInvocation {
                program: "xdg-open".to_string(),
                args: vec![payload],
            }]),
            ControlCapability::Notification => Some(vec![PlatformInvocation {
                program: "notify-send".to_string(),
                args: vec!["Flow".to_string(), payload],
            }]),
            ControlCapability::MediaPlayback => Some(vec![PlatformInvocation {
                program: "playerctl".to_string(),
                args: vec!["play-pause".to_string()],
            }]),
            _ => None,
        }
    }

    fn plan_android(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        let payload = action.suggested_payload.clone().unwrap_or_default();

        match action.capability {
            ControlCapability::OpenUrl => Some(vec![PlatformInvocation {
                program: "am".to_string(),
                args: vec![
                    "start".to_string(),
                    "-a".to_string(),
                    "android.intent.action.VIEW".to_string(),
                    "-d".to_string(),
                    payload,
                ],
            }]),
            ControlCapability::OpenApplication => Some(vec![PlatformInvocation {
                program: "monkey".to_string(),
                args: vec![
                    "-p".to_string(),
                    payload,
                    "-c".to_string(),
                    "android.intent.category.LAUNCHER".to_string(),
                    "1".to_string(),
                ],
            }]),
            _ => None,
        }
    }

    fn plan_ios(&self, _: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        None
    }

    fn plan_server(&self, action: &ControlActionPlan) -> Option<Vec<PlatformInvocation>> {
        if matches!(
            action.capability,
            ControlCapability::OpenFile | ControlCapability::OpenApplication
        ) {
            let payload = action.suggested_payload.clone().unwrap_or_default();
            return Some(vec![PlatformInvocation {
                program: payload,
                args: Vec::new(),
            }]);
        }

        None
    }
}

impl FlowControlExecutor for NativeControlExecutor {
    fn execute(&mut self, action: &ControlActionPlan) -> ExecutedActionReceipt {
        let Some(invocations) = self.plan(action) else {
            return ExecutedActionReceipt {
                capability: action.capability.clone(),
                executed: false,
                message: "No native executor is available for this action on the current OS."
                    .to_string(),
            };
        };

        if self.dry_run {
            return ExecutedActionReceipt {
                capability: action.capability.clone(),
                executed: true,
                message: format!("Planned {} native invocation(s).", invocations.len()),
            };
        }

        let mut failures = 0;
        for invocation in &invocations {
            let status = Command::new(&invocation.program)
                .args(&invocation.args)
                .status();
            match status {
                Ok(exit) if exit.success() => {}
                _ => failures += 1,
            }
        }

        ExecutedActionReceipt {
            capability: action.capability.clone(),
            executed: failures == 0,
            message: if failures == 0 {
                "Native action executed successfully.".to_string()
            } else {
                format!("Native action execution had {} failure(s).", failures)
            },
        }
    }
}

fn escape_single_quotes(value: &str) -> String {
    value.replace('\'', "\\'")
}
