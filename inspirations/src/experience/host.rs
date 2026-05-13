use super::control::{ControlActionPlan, ControlCapability, ControlSurface};

pub trait FlowHostControlAdapter {
    fn surface(&self) -> ControlSurface;
    fn supported_capabilities(&self) -> &[ControlCapability];

    fn can_execute(&self, action: &ControlActionPlan) -> bool {
        self.supported_capabilities().contains(&action.capability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAdapterDescriptor {
    pub name: &'static str,
    pub surface: ControlSurface,
    pub capabilities: Vec<ControlCapability>,
    pub requires_accessibility_api: bool,
    pub supports_background_detection: bool,
}

impl HostAdapterDescriptor {
    pub fn windows_desktop() -> Self {
        Self {
            name: "windows-desktop",
            surface: ControlSurface::Desktop,
            capabilities: vec![
                ControlCapability::ReadClipboard,
                ControlCapability::WriteClipboard,
                ControlCapability::ReadSelection,
                ControlCapability::ReplaceSelection,
                ControlCapability::SimulateShortcut,
                ControlCapability::OpenUrl,
                ControlCapability::OpenApplication,
                ControlCapability::OpenFile,
                ControlCapability::RevealFile,
                ControlCapability::CreateDraftFile,
                ControlCapability::FocusWindow,
                ControlCapability::MediaPlayback,
                ControlCapability::VolumeControl,
                ControlCapability::BrightnessControl,
                ControlCapability::SystemSearch,
                ControlCapability::Notification,
            ],
            requires_accessibility_api: true,
            supports_background_detection: true,
        }
    }

    pub fn macos_desktop() -> Self {
        Self {
            name: "macos-desktop",
            surface: ControlSurface::Desktop,
            capabilities: vec![
                ControlCapability::ReadClipboard,
                ControlCapability::WriteClipboard,
                ControlCapability::ReadSelection,
                ControlCapability::ReplaceSelection,
                ControlCapability::SimulateShortcut,
                ControlCapability::OpenUrl,
                ControlCapability::OpenApplication,
                ControlCapability::OpenFile,
                ControlCapability::RevealFile,
                ControlCapability::FocusWindow,
                ControlCapability::MediaPlayback,
                ControlCapability::VolumeControl,
                ControlCapability::BrightnessControl,
                ControlCapability::SystemSearch,
                ControlCapability::Notification,
            ],
            requires_accessibility_api: true,
            supports_background_detection: true,
        }
    }

    pub fn linux_desktop() -> Self {
        Self {
            name: "linux-desktop",
            surface: ControlSurface::Desktop,
            capabilities: vec![
                ControlCapability::ReadClipboard,
                ControlCapability::WriteClipboard,
                ControlCapability::ReplaceSelection,
                ControlCapability::SimulateShortcut,
                ControlCapability::OpenUrl,
                ControlCapability::OpenApplication,
                ControlCapability::OpenFile,
                ControlCapability::RevealFile,
                ControlCapability::FocusWindow,
                ControlCapability::MediaPlayback,
                ControlCapability::VolumeControl,
                ControlCapability::SystemSearch,
                ControlCapability::Notification,
            ],
            requires_accessibility_api: false,
            supports_background_detection: true,
        }
    }

    pub fn mobile_shell() -> Self {
        Self {
            name: "mobile-shell",
            surface: ControlSurface::Mobile,
            capabilities: vec![
                ControlCapability::WriteClipboard,
                ControlCapability::OpenUrl,
                ControlCapability::OpenApplication,
                ControlCapability::Notification,
                ControlCapability::MediaPlayback,
                ControlCapability::SystemSearch,
            ],
            requires_accessibility_api: false,
            supports_background_detection: false,
        }
    }

    pub fn editor_embed() -> Self {
        Self {
            name: "editor-embed",
            surface: ControlSurface::EditorEmbed,
            capabilities: vec![
                ControlCapability::ReadSelection,
                ControlCapability::ReplaceSelection,
                ControlCapability::SimulateShortcut,
                ControlCapability::OpenFile,
                ControlCapability::RevealFile,
                ControlCapability::SystemSearch,
            ],
            requires_accessibility_api: false,
            supports_background_detection: false,
        }
    }
}
