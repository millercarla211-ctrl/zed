#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayMode {
    Hidden,
    Compact,
    Dictation,
    CommandPalette,
    RewritePreview,
    ProofingPanel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayAction {
    pub id: &'static str,
    pub title: &'static str,
    pub shortcut_hint: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowOverlayState {
    pub mode: OverlayMode,
    pub pinned: bool,
    pub width_px: u16,
    pub height_px: u16,
    pub quick_actions: Vec<OverlayAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowOverlayController;

impl FlowOverlayController {
    pub fn low_end_default() -> FlowOverlayState {
        FlowOverlayState {
            mode: OverlayMode::Compact,
            pinned: false,
            width_px: 440,
            height_px: 96,
            quick_actions: quick_actions(),
        }
    }

    pub fn expanded_default() -> FlowOverlayState {
        FlowOverlayState {
            mode: OverlayMode::CommandPalette,
            pinned: false,
            width_px: 760,
            height_px: 420,
            quick_actions: quick_actions(),
        }
    }

    pub fn mode_for_lifecycle(
        state: &super::lifecycle::FlowRuntimeState,
        current: &FlowOverlayState,
    ) -> FlowOverlayState {
        let mut next = current.clone();
        next.mode = match state {
            super::lifecycle::FlowRuntimeState::ColdBoot
            | super::lifecycle::FlowRuntimeState::Listening
            | super::lifecycle::FlowRuntimeState::Sleeping
            | super::lifecycle::FlowRuntimeState::Paused => OverlayMode::Hidden,
            super::lifecycle::FlowRuntimeState::Overlay => OverlayMode::Compact,
            super::lifecycle::FlowRuntimeState::Dictating => OverlayMode::Dictation,
            super::lifecycle::FlowRuntimeState::CommandMode => OverlayMode::CommandPalette,
        };
        next
    }

    pub fn proofing_panel(current: &FlowOverlayState) -> FlowOverlayState {
        let mut next = current.clone();
        next.mode = OverlayMode::ProofingPanel;
        next.width_px = 840;
        next.height_px = 520;
        next
    }

    pub fn rewrite_preview(current: &FlowOverlayState) -> FlowOverlayState {
        let mut next = current.clone();
        next.mode = OverlayMode::RewritePreview;
        next.width_px = 840;
        next.height_px = 420;
        next
    }
}

fn quick_actions() -> Vec<OverlayAction> {
    vec![
        OverlayAction {
            id: "dictate",
            title: "Dictate",
            shortcut_hint: "Ctrl+Shift+Space",
        },
        OverlayAction {
            id: "rewrite",
            title: "Rewrite",
            shortcut_hint: "Say: rewrite this",
        },
        OverlayAction {
            id: "grammar",
            title: "Fix Grammar",
            shortcut_hint: "Say: fix this",
        },
        OverlayAction {
            id: "command",
            title: "Command Mode",
            shortcut_hint: "Ctrl+Alt+Space",
        },
    ]
}
