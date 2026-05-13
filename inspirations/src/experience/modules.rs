use super::always_on::FlowDeviceTier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatingSystemFamily {
    Windows,
    Macos,
    Linux,
    Android,
    Ios,
    BrowserWasm,
    Server,
}

impl OperatingSystemFamily {
    pub fn from_host_label(label: &str) -> Self {
        let lower = label.to_ascii_lowercase();

        if lower.contains("windows") || lower == "win32" {
            return Self::Windows;
        }

        if lower.contains("mac") || lower.contains("darwin") || lower.contains("osx") {
            return Self::Macos;
        }

        if lower.contains("android") {
            return Self::Android;
        }

        if lower.contains("ios") {
            return Self::Ios;
        }

        if lower.contains("wasm") || lower.contains("browser") || lower.contains("web") {
            return Self::BrowserWasm;
        }

        if lower.contains("server") || lower.contains("vps") || lower.contains("daemon") {
            return Self::Server;
        }

        Self::Linux
    }

    pub fn is_desktop(&self) -> bool {
        matches!(self, Self::Windows | Self::Macos | Self::Linux)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleChannel {
    Green,
    Balanced,
    Creator,
    Workstation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModulePurpose {
    Core,
    Speech,
    Writing,
    Commanding,
    Control,
    HostBridge,
    Vision,
    ThreeD,
    Media,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallTrigger {
    FirstRun,
    Upgrade,
    Repair,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowModuleDescriptor {
    pub id: &'static str,
    pub channel: ModuleChannel,
    pub purpose: ModulePurpose,
    pub required: bool,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowModuleInstallPlan {
    pub os: OperatingSystemFamily,
    pub tier: FlowDeviceTier,
    pub trigger: InstallTrigger,
    pub automatic: bool,
    pub modules: Vec<FlowModuleDescriptor>,
    pub deferred_modules: Vec<FlowModuleDescriptor>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowModuleBootstrapper {
    pub product_name: &'static str,
}

impl FlowModuleBootstrapper {
    pub fn new() -> Self {
        Self {
            product_name: "flow-module-bootstrapper",
        }
    }

    pub fn first_run_plan(
        &self,
        os: OperatingSystemFamily,
        tier: FlowDeviceTier,
    ) -> FlowModuleInstallPlan {
        self.plan(InstallTrigger::FirstRun, os, tier)
    }

    pub fn plan(
        &self,
        trigger: InstallTrigger,
        os: OperatingSystemFamily,
        tier: FlowDeviceTier,
    ) -> FlowModuleInstallPlan {
        let mut modules = base_modules_for_tier(&tier);
        modules.extend(os_modules(&os));
        let deferred_modules = deferred_modules_for_tier(&tier);
        let notes = build_notes(&os, &tier);

        FlowModuleInstallPlan {
            os,
            tier,
            trigger,
            automatic: true,
            modules,
            deferred_modules,
            notes,
        }
    }
}

fn base_modules_for_tier(tier: &FlowDeviceTier) -> Vec<FlowModuleDescriptor> {
    match tier {
        FlowDeviceTier::LowEnd => vec![
            module(
                "green-runtime-core",
                ModuleChannel::Green,
                ModulePurpose::Core,
                true,
                "Minimal runtime core for low-end 24/7 operation.",
            ),
            module(
                "green-wake-openwake",
                ModuleChannel::Green,
                ModulePurpose::Speech,
                true,
                "Wake-word detection using the local openwake model set.",
            ),
            module(
                "green-dictation-moonshine-tiny",
                ModuleChannel::Green,
                ModulePurpose::Speech,
                true,
                "Streaming dictation for bad hardware.",
            ),
            module(
                "green-proofing-harper",
                ModuleChannel::Green,
                ModulePurpose::Writing,
                true,
                "On-device grammar and clarity pass.",
            ),
            module(
                "green-rewrite-qwen3-0.6b",
                ModuleChannel::Green,
                ModulePurpose::Writing,
                true,
                "Small rewrite model for instant local commands and cleanup.",
            ),
            module(
                "green-tts-kokoro-int8",
                ModuleChannel::Green,
                ModulePurpose::Speech,
                false,
                "Optional lightweight voice confirmations.",
            ),
            module(
                "green-command-router",
                ModuleChannel::Green,
                ModulePurpose::Commanding,
                true,
                "Typed and spoken command routing.",
            ),
            module(
                "green-host-control-policy",
                ModuleChannel::Green,
                ModulePurpose::Control,
                true,
                "Safe OS-control policy, approval, and audit surfaces.",
            ),
            module(
                "green-3d-assist-preview",
                ModuleChannel::Green,
                ModulePurpose::ThreeD,
                false,
                "Lightweight 3D helper hooks for low-end creative workflows.",
            ),
        ],
        FlowDeviceTier::Balanced => vec![
            module(
                "balanced-runtime-core",
                ModuleChannel::Balanced,
                ModulePurpose::Core,
                true,
                "Balanced desktop runtime core.",
            ),
            module(
                "balanced-wake-openwake",
                ModuleChannel::Balanced,
                ModulePurpose::Speech,
                true,
                "Wake-word detection with higher buffering comfort.",
            ),
            module(
                "balanced-dictation-moonshine-streaming",
                ModuleChannel::Balanced,
                ModulePurpose::Speech,
                true,
                "Better streaming dictation defaults.",
            ),
            module(
                "balanced-rewrite-smollm3-3b",
                ModuleChannel::Balanced,
                ModulePurpose::Writing,
                true,
                "Stronger command and rewrite quality for everyday work.",
            ),
            module(
                "balanced-proofing-suite",
                ModuleChannel::Balanced,
                ModulePurpose::Writing,
                true,
                "Expanded grammar, clarity, and tone assistance.",
            ),
            module(
                "balanced-overlay-core",
                ModuleChannel::Balanced,
                ModulePurpose::Core,
                true,
                "Desktop overlay and quick actions.",
            ),
        ],
        FlowDeviceTier::Creator => vec![
            module(
                "creator-runtime-core",
                ModuleChannel::Creator,
                ModulePurpose::Core,
                true,
                "Creator-grade runtime core.",
            ),
            module(
                "creator-vlm-core",
                ModuleChannel::Creator,
                ModulePurpose::Vision,
                true,
                "Local image and document understanding path.",
            ),
            module(
                "creator-image-core",
                ModuleChannel::Creator,
                ModulePurpose::Media,
                true,
                "Creator-grade image generation support.",
            ),
            module(
                "creator-3d-assist",
                ModuleChannel::Creator,
                ModulePurpose::ThreeD,
                false,
                "3D workflow assistant hooks for stronger desktops.",
            ),
        ],
        FlowDeviceTier::Workstation => vec![
            module(
                "workstation-runtime-core",
                ModuleChannel::Workstation,
                ModulePurpose::Core,
                true,
                "High-end runtime core for large local workflows.",
            ),
            module(
                "workstation-multimodal-suite",
                ModuleChannel::Workstation,
                ModulePurpose::Vision,
                true,
                "High-end VLM and multimodal path.",
            ),
            module(
                "workstation-video-suite",
                ModuleChannel::Workstation,
                ModulePurpose::Media,
                false,
                "Optional local video workflows.",
            ),
            module(
                "workstation-3d-assist",
                ModuleChannel::Workstation,
                ModulePurpose::ThreeD,
                false,
                "Optional 3D workflow assistant for powerful hosts.",
            ),
        ],
    }
}

fn deferred_modules_for_tier(tier: &FlowDeviceTier) -> Vec<FlowModuleDescriptor> {
    match tier {
        FlowDeviceTier::LowEnd => vec![
            module(
                "balanced-rewrite-smollm3-3b",
                ModuleChannel::Balanced,
                ModulePurpose::Writing,
                false,
                "Deferred until the device can sustain higher memory usage.",
            ),
            module(
                "balanced-multimodal-gemma-4-e2b",
                ModuleChannel::Balanced,
                ModulePurpose::Vision,
                false,
                "Deferred on low-end machines.",
            ),
            module(
                "creator-image-flux-schnell",
                ModuleChannel::Creator,
                ModulePurpose::Media,
                false,
                "Deferred because image generation is too expensive for this tier.",
            ),
            module(
                "workstation-video-wan2.1",
                ModuleChannel::Workstation,
                ModulePurpose::Media,
                false,
                "Deferred because video generation is outside the low-end budget.",
            ),
        ],
        FlowDeviceTier::Balanced => vec![
            module(
                "creator-image-flux-schnell",
                ModuleChannel::Creator,
                ModulePurpose::Media,
                false,
                "Deferred unless creator mode is explicitly enabled.",
            ),
            module(
                "workstation-video-wan2.1",
                ModuleChannel::Workstation,
                ModulePurpose::Media,
                false,
                "Deferred unless the machine benchmarks as workstation-grade.",
            ),
        ],
        FlowDeviceTier::Creator => vec![module(
            "workstation-video-wan2.1",
            ModuleChannel::Workstation,
            ModulePurpose::Media,
            false,
            "Deferred unless the machine is promoted to workstation mode.",
        )],
        FlowDeviceTier::Workstation => Vec::new(),
    }
}

fn os_modules(os: &OperatingSystemFamily) -> Vec<FlowModuleDescriptor> {
    match os {
        OperatingSystemFamily::Windows => vec![
            module(
                "windows-global-hotkeys",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Global shortcut handling for Windows hosts.",
            ),
            module(
                "windows-accessibility-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Selection and UI automation bridge for Windows.",
            ),
            module(
                "windows-notification-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Windows notifications and quick prompts.",
            ),
        ],
        OperatingSystemFamily::Macos => vec![
            module(
                "macos-global-hotkeys",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Global shortcut handling for macOS hosts.",
            ),
            module(
                "macos-accessibility-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Accessibility and text insertion bridge for macOS.",
            ),
            module(
                "macos-notification-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "macOS notification bridge.",
            ),
        ],
        OperatingSystemFamily::Linux => vec![
            module(
                "linux-global-hotkeys",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Global shortcut handling for Linux hosts.",
            ),
            module(
                "linux-wayland-x11-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Selection, clipboard, and text insertion bridge for Linux desktops.",
            ),
            module(
                "linux-xdg-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "App launch, file open, and notification bridge for Linux.",
            ),
        ],
        OperatingSystemFamily::Android => vec![
            module(
                "android-ime-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Keyboard-service bridge for Android.",
            ),
            module(
                "android-overlay-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Floating overlay support for Android hosts.",
            ),
            module(
                "android-foreground-service",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Background wake and dictation service on Android.",
            ),
        ],
        OperatingSystemFamily::Ios => vec![
            module(
                "ios-keyboard-extension",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Custom keyboard and text insertion surface for iOS.",
            ),
            module(
                "ios-app-intents-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Shortcuts and app-intent bridge for iOS.",
            ),
        ],
        OperatingSystemFamily::BrowserWasm => vec![
            module(
                "wasm-runtime-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "WASM entry point for browser-hosted Flow.",
            ),
            module(
                "web-clipboard-bridge",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Clipboard integration for browser shells.",
            ),
            module(
                "service-worker-cache",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                false,
                "Optional offline cache for browser-hosted assets.",
            ),
        ],
        OperatingSystemFamily::Server => vec![
            module(
                "server-daemon-core",
                ModuleChannel::Green,
                ModulePurpose::HostBridge,
                true,
                "Headless daemon support for server and VPS deployments.",
            ),
            module(
                "server-audit-store",
                ModuleChannel::Green,
                ModulePurpose::Control,
                true,
                "Persistent audit storage for server-hosted Flow actions.",
            ),
        ],
    }
}

fn build_notes(os: &OperatingSystemFamily, tier: &FlowDeviceTier) -> Vec<String> {
    let mut notes = Vec::new();

    if matches!(tier, FlowDeviceTier::LowEnd) {
        notes.push(
            "Low-end device detected: install only green-tier modules automatically on first run."
                .to_string(),
        );
        notes.push(
            "Heavier multimodal, image, and video modules stay deferred until benchmarking promotes the device."
                .to_string(),
        );
    }

    match os {
        OperatingSystemFamily::Windows => notes.push(
            "Windows hosts should prioritize accessibility and global shortcut adapters first."
                .to_string(),
        ),
        OperatingSystemFamily::Macos => notes.push(
            "macOS hosts should request accessibility permissions before enabling full text replacement."
                .to_string(),
        ),
        OperatingSystemFamily::Linux => notes.push(
            "Linux hosts should select Wayland or X11 bridges at runtime based on the active session."
                .to_string(),
        ),
        OperatingSystemFamily::Android => notes.push(
            "Android hosts should bootstrap keyboard-service and overlay modules before background wake support."
                .to_string(),
        ),
        OperatingSystemFamily::Ios => notes.push(
            "iOS hosts should bootstrap keyboard-extension and app-intent paths because global desktop-style control is unavailable."
                .to_string(),
        ),
        OperatingSystemFamily::BrowserWasm => notes.push(
            "Browser-hosted Flow should use WASM-compatible modules only and leave OS control to approved browser APIs."
                .to_string(),
        ),
        OperatingSystemFamily::Server => notes.push(
            "Server-hosted Flow should disable user-facing overlays and focus on daemon, routing, and audit services."
                .to_string(),
        ),
    }

    notes
}

fn module(
    id: &'static str,
    channel: ModuleChannel,
    purpose: ModulePurpose,
    required: bool,
    note: &'static str,
) -> FlowModuleDescriptor {
    FlowModuleDescriptor {
        id,
        channel,
        purpose,
        required,
        note,
    }
}
