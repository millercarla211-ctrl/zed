use std::io::Write;
use std::process::{Command, Stdio};

use super::{contracts::FlowAutomationBridge, modules::OperatingSystemFamily};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardAutomationBridge {
    pub os: OperatingSystemFamily,
    pub dry_run: bool,
}

impl ClipboardAutomationBridge {
    pub fn new(os: OperatingSystemFamily) -> Self {
        Self { os, dry_run: true }
    }

    pub fn live(os: OperatingSystemFamily) -> Self {
        Self { os, dry_run: false }
    }

    pub fn current_text(&self) -> Option<String> {
        self.read_clipboard()
    }

    pub fn stage_text(&self, text: &str) -> bool {
        self.write_clipboard(text)
    }

    fn read_clipboard(&self) -> Option<String> {
        if self.dry_run {
            return Some(String::new());
        }

        let output = match self.os {
            OperatingSystemFamily::Windows => Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-Clipboard"])
                .output()
                .ok()?,
            OperatingSystemFamily::Macos => Command::new("pbpaste").output().ok()?,
            OperatingSystemFamily::Linux => {
                Command::new("wl-paste").output().ok().or_else(|| {
                    Command::new("xclip")
                        .args(["-o", "-selection", "clipboard"])
                        .output()
                        .ok()
                })?
            }
            _ => return None,
        };

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }

    fn write_clipboard(&self, text: &str) -> bool {
        if self.dry_run {
            return true;
        }

        match self.os {
            OperatingSystemFamily::Windows => Command::new("powershell")
                .args(["-NoProfile", "-Command", "Set-Clipboard"])
                .stdin(Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    if let Some(stdin) = child.stdin.as_mut() {
                        stdin.write_all(text.as_bytes())?;
                    }
                    child.wait()
                })
                .map(|status| status.success())
                .unwrap_or(false),
            OperatingSystemFamily::Macos => pipe_text("pbcopy", &[], text),
            OperatingSystemFamily::Linux => {
                pipe_text("wl-copy", &[], text)
                    || pipe_text("xclip", &["-selection", "clipboard"], text)
            }
            _ => false,
        }
    }

    fn send_native_shortcut(&self, shortcut: &str) -> bool {
        if self.dry_run {
            return true;
        }

        match self.os {
            OperatingSystemFamily::Windows => {
                let keys = windows_sendkeys(shortcut);
                Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "$wshell = New-Object -ComObject WScript.Shell; $wshell.SendKeys('{}')",
                            keys
                        ),
                    ])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            }
            OperatingSystemFamily::Macos => {
                let script = macos_shortcut_script(shortcut);
                Command::new("osascript")
                    .args(["-e", &script])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            }
            OperatingSystemFamily::Linux => Command::new("xdotool")
                .args(["key", &linux_shortcut(shortcut)])
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
            _ => false,
        }
    }
}

impl FlowAutomationBridge for ClipboardAutomationBridge {
    fn read_selection(&mut self) -> Option<String> {
        self.read_clipboard()
    }

    fn replace_selection(&mut self, text: &str) -> bool {
        if !self.write_clipboard(text) {
            return false;
        }

        let paste_shortcut = match self.os {
            OperatingSystemFamily::Macos => "Cmd+V",
            _ => "Ctrl+V",
        };

        self.send_native_shortcut(paste_shortcut)
    }

    fn send_shortcut(&mut self, shortcut: &str) -> bool {
        self.send_native_shortcut(shortcut)
    }
}

fn pipe_text(program: &str, args: &[&str], text: &str) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}

fn windows_sendkeys(shortcut: &str) -> String {
    shortcut
        .replace("Ctrl", "^")
        .replace("Alt", "%")
        .replace("Shift", "+")
        .replace("Cmd", "^")
        .replace("+", "")
}

fn macos_shortcut_script(shortcut: &str) -> String {
    let mut modifiers = Vec::new();
    if shortcut.contains("Cmd") {
        modifiers.push("command down");
    }
    if shortcut.contains("Ctrl") {
        modifiers.push("control down");
    }
    if shortcut.contains("Alt") {
        modifiers.push("option down");
    }
    if shortcut.contains("Shift") {
        modifiers.push("shift down");
    }
    let key = shortcut
        .split('+')
        .last()
        .unwrap_or(shortcut)
        .trim()
        .to_ascii_lowercase();
    let using = if modifiers.is_empty() {
        String::new()
    } else {
        format!(" using {{{}}}", modifiers.join(", "))
    };
    format!(
        "tell application \"System Events\" to keystroke \"{}\"{}",
        key, using
    )
}

fn linux_shortcut(shortcut: &str) -> String {
    shortcut
        .replace("Ctrl", "ctrl")
        .replace("Alt", "alt")
        .replace("Shift", "shift")
        .replace("Cmd", "super")
        .replace('+', "+")
        .to_ascii_lowercase()
}
