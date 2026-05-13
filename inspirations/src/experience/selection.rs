use std::process::Command;

use super::{
    accessibility::FlowAccessibilityRuntime, bridges::ClipboardAutomationBridge,
    contracts::FlowAutomationBridge, modules::OperatingSystemFamily,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSelectionBridge {
    pub os: OperatingSystemFamily,
    pub dry_run: bool,
    pub accessibility: FlowAccessibilityRuntime,
    pub clipboard: ClipboardAutomationBridge,
}

impl NativeSelectionBridge {
    pub fn new(os: OperatingSystemFamily) -> Self {
        let accessibility = FlowAccessibilityRuntime::dry_run(os.clone());
        Self {
            clipboard: ClipboardAutomationBridge::new(os.clone()),
            os,
            dry_run: true,
            accessibility,
        }
    }

    pub fn live(os: OperatingSystemFamily) -> Self {
        let accessibility = FlowAccessibilityRuntime::live(os.clone());
        Self {
            clipboard: ClipboardAutomationBridge::live(os.clone()),
            os,
            dry_run: false,
            accessibility,
        }
    }

    pub fn with_accessibility(accessibility: FlowAccessibilityRuntime, dry_run: bool) -> Self {
        let os = accessibility.os.clone();
        Self {
            clipboard: if dry_run {
                ClipboardAutomationBridge::new(os.clone())
            } else {
                ClipboardAutomationBridge::live(os.clone())
            },
            os,
            dry_run,
            accessibility,
        }
    }

    fn send_copy_shortcut(&self) -> bool {
        let shortcut = match self.os {
            OperatingSystemFamily::Macos => "Cmd+C",
            _ => "Ctrl+C",
        };
        self.send_shortcut_native(shortcut)
    }

    fn send_paste_shortcut(&self) -> bool {
        let shortcut = match self.os {
            OperatingSystemFamily::Macos => "Cmd+V",
            _ => "Ctrl+V",
        };
        self.send_shortcut_native(shortcut)
    }

    fn send_shortcut_native(&self, shortcut: &str) -> bool {
        if self.dry_run {
            return true;
        }

        if !self.accessibility.can_send_shortcuts {
            return false;
        }

        match self.os {
            OperatingSystemFamily::Windows => Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "$wshell = New-Object -ComObject WScript.Shell; $wshell.SendKeys('{}')",
                        windows_sendkeys(shortcut)
                    ),
                ])
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
            OperatingSystemFamily::Macos => Command::new("osascript")
                .args(["-e", &macos_shortcut_script(shortcut)])
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
            OperatingSystemFamily::Linux => Command::new("xdotool")
                .args(["key", &linux_shortcut(shortcut)])
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
            _ => false,
        }
    }

    fn replace_selection_preserving_clipboard(&mut self, text: &str) -> bool {
        let previous_clipboard = self.clipboard.current_text();
        if !self.clipboard.stage_text(text) {
            return false;
        }

        let pasted = if self.dry_run {
            true
        } else {
            self.send_paste_shortcut()
        };

        if let Some(previous) = previous_clipboard {
            let _ = self.clipboard.stage_text(&previous);
        }

        pasted
    }
}

impl FlowAutomationBridge for NativeSelectionBridge {
    fn read_selection(&mut self) -> Option<String> {
        if self.accessibility.can_read_selection && self.send_copy_shortcut() {
            if let Some(selection) = self.clipboard.current_text() {
                if !selection.is_empty() || self.dry_run {
                    return Some(selection);
                }
            }
        }

        if matches!(
            self.accessibility.mode,
            super::accessibility::AccessibilityMode::ClipboardFallback
        ) {
            return self.clipboard.current_text();
        }

        None
    }

    fn replace_selection(&mut self, text: &str) -> bool {
        if !self.accessibility.can_replace_selection {
            return false;
        }

        self.replace_selection_preserving_clipboard(text)
    }

    fn send_shortcut(&mut self, shortcut: &str) -> bool {
        self.send_shortcut_native(shortcut)
    }
}

fn windows_sendkeys(shortcut: &str) -> String {
    shortcut
        .replace("Ctrl", "^")
        .replace("Alt", "%")
        .replace("Shift", "+")
        .replace("Cmd", "^")
        .replace('+', "")
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
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_bridge_keeps_runtime_and_clipboard_in_sync() {
        let runtime = FlowAccessibilityRuntime::dry_run(OperatingSystemFamily::Windows);
        let bridge = NativeSelectionBridge::with_accessibility(runtime, true);
        assert!(bridge.dry_run);
        assert!(bridge.clipboard.dry_run);
    }
}
