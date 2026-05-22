pub(crate) const DX_STUDIO_BRIDGE_SCRIPT: &str = concat!(
    include_str!("dx_studio_bridge/preamble.ts"),
    include_str!("dx_studio_bridge/selection.ts"),
    include_str!("dx_studio_bridge/overlay.ts"),
    include_str!("dx_studio_bridge/capture.ts"),
    include_str!("dx_studio_bridge/source_edit.ts"),
    include_str!("dx_studio_bridge/api.ts"),
);
