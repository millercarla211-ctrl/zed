use super::{
    audio::FlowAudioPipeline,
    control::{ControlActionPlan, ControlCapability},
    modules::{FlowModuleDescriptor, OperatingSystemFamily},
    overlay::FlowOverlayState,
    persistence::FlowPersistentState,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct FlowHostSnapshot {
    pub os: OperatingSystemFamily,
    pub host_label: String,
    pub app_name: Option<String>,
    pub ram_gb: f32,
    pub vram_gb: Option<f32>,
    pub cpu_only: bool,
    pub battery_percent: Option<u8>,
    pub thermal_celsius: Option<u8>,
}

impl FlowHostSnapshot {
    pub fn new(
        os: OperatingSystemFamily,
        host_label: impl Into<String>,
        ram_gb: f32,
        vram_gb: Option<f32>,
        cpu_only: bool,
    ) -> Self {
        Self {
            os,
            host_label: host_label.into(),
            app_name: None,
            ram_gb,
            vram_gb,
            cpu_only,
            battery_percent: None,
            thermal_celsius: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModuleReceipt {
    pub module_id: String,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedActionReceipt {
    pub capability: ControlCapability,
    pub executed: bool,
    pub message: String,
}

pub trait FlowModuleInstaller {
    fn install_modules(&mut self, modules: &[FlowModuleDescriptor]) -> Vec<InstalledModuleReceipt>;
}

pub trait FlowStateStore {
    fn load_state(&self) -> Option<FlowPersistentState>;
    fn save_state(&mut self, state: FlowPersistentState);
}

pub trait FlowPermissionGate {
    fn is_granted(&self, capability: &ControlCapability) -> bool;
    fn request(&mut self, capability: &ControlCapability, reason: &str) -> bool;
}

pub trait FlowControlExecutor {
    fn execute(&mut self, action: &ControlActionPlan) -> ExecutedActionReceipt;
}

pub trait FlowOverlayPresenter {
    fn present(&mut self, overlay: &FlowOverlayState);
}

pub trait FlowAudioRuntime {
    fn configure(&mut self, pipeline: &FlowAudioPipeline);
}

pub trait FlowAutomationBridge {
    fn read_selection(&mut self) -> Option<String>;
    fn replace_selection(&mut self, text: &str) -> bool;
    fn send_shortcut(&mut self, shortcut: &str) -> bool;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingModuleInstaller {
    pub installed: Vec<InstalledModuleReceipt>,
}

impl FlowModuleInstaller for RecordingModuleInstaller {
    fn install_modules(&mut self, modules: &[FlowModuleDescriptor]) -> Vec<InstalledModuleReceipt> {
        let receipts: Vec<_> = modules
            .iter()
            .map(|module| InstalledModuleReceipt {
                module_id: module.id.to_string(),
                installed: true,
            })
            .collect();
        self.installed.extend(receipts.clone());
        receipts
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryStateStore {
    pub state: Option<FlowPersistentState>,
}

impl FlowStateStore for MemoryStateStore {
    fn load_state(&self) -> Option<FlowPersistentState> {
        self.state.clone()
    }

    fn save_state(&mut self, state: FlowPersistentState) {
        self.state = Some(state);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantAllPermissionGate;

impl FlowPermissionGate for GrantAllPermissionGate {
    fn is_granted(&self, _: &ControlCapability) -> bool {
        true
    }

    fn request(&mut self, _: &ControlCapability, _: &str) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryPermissionGate {
    granted: BTreeSet<ControlCapability>,
}

impl MemoryPermissionGate {
    pub fn grant(&mut self, capability: ControlCapability) {
        self.granted.insert(capability);
    }

    pub fn revoke(&mut self, capability: &ControlCapability) {
        self.granted.remove(capability);
    }

    pub fn granted(&self) -> &BTreeSet<ControlCapability> {
        &self.granted
    }
}

impl FlowPermissionGate for MemoryPermissionGate {
    fn is_granted(&self, capability: &ControlCapability) -> bool {
        self.granted.contains(capability)
    }

    fn request(&mut self, capability: &ControlCapability, _: &str) -> bool {
        self.grant(capability.clone());
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingControlExecutor {
    pub receipts: Vec<ExecutedActionReceipt>,
}

impl FlowControlExecutor for RecordingControlExecutor {
    fn execute(&mut self, action: &ControlActionPlan) -> ExecutedActionReceipt {
        let receipt = ExecutedActionReceipt {
            capability: action.capability.clone(),
            executed: true,
            message: action.description.clone(),
        };
        self.receipts.push(receipt.clone());
        receipt
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingOverlayPresenter {
    pub last: Option<FlowOverlayState>,
}

impl FlowOverlayPresenter for RecordingOverlayPresenter {
    fn present(&mut self, overlay: &FlowOverlayState) {
        self.last = Some(overlay.clone());
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingAudioRuntime {
    pub last: Option<FlowAudioPipeline>,
}

impl FlowAudioRuntime for RecordingAudioRuntime {
    fn configure(&mut self, pipeline: &FlowAudioPipeline) {
        self.last = Some(pipeline.clone());
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingAutomationBridge {
    pub selection: Option<String>,
    pub replacements: Vec<String>,
    pub shortcuts: Vec<String>,
}

impl FlowAutomationBridge for RecordingAutomationBridge {
    fn read_selection(&mut self) -> Option<String> {
        self.selection.clone()
    }

    fn replace_selection(&mut self, text: &str) -> bool {
        self.replacements.push(text.to_string());
        true
    }

    fn send_shortcut(&mut self, shortcut: &str) -> bool {
        self.shortcuts.push(shortcut.to_string());
        true
    }
}
