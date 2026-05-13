use super::{
    activation::FlowActivationProfile, always_on::FlowAlwaysOnProfile, audio::FlowAudioPlanner,
    audit::FlowControlAuditLog, command::FlowCommandRouter, control::FlowControlPolicy,
    editor::FlowEditorAssistPlanner, lifecycle::FlowLifecycleController,
    modules::FlowModuleBootstrapper, onboarding::FlowOnboardingBuilder,
    overlay::FlowOverlayController, permissions::FlowPermissionPlanner,
    proofing::FlowProofingPlanner, recovery::FlowRecoveryPlanner,
    runtime_policy::FlowRuntimeTierPolicy,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowProductSurface {
    pub activation: FlowActivationProfile,
    pub always_on: FlowAlwaysOnProfile,
    pub control: FlowControlPolicy,
    pub audit: FlowControlAuditLog,
    pub commands: FlowCommandRouter,
    pub lifecycle: FlowLifecycleController,
    pub audio: FlowAudioPlanner,
    pub modules: FlowModuleBootstrapper,
    pub onboarding: FlowOnboardingBuilder,
    pub overlay: FlowOverlayController,
    pub permissions: FlowPermissionPlanner,
    pub proofing: FlowProofingPlanner,
    pub recovery: FlowRecoveryPlanner,
    pub editor: FlowEditorAssistPlanner,
}

impl FlowProductSurface {
    pub fn for_host(os: &str, ram_gb: f32, vram_gb: Option<f32>, cpu_only: bool) -> Self {
        let os_family = super::modules::OperatingSystemFamily::from_host_label(os);
        let tier_policy = FlowRuntimeTierPolicy::new();
        let tier = tier_policy.baseline_tier(ram_gb, vram_gb, cpu_only);

        match (&os_family, tier) {
            (super::modules::OperatingSystemFamily::Android, _)
            | (super::modules::OperatingSystemFamily::Ios, _)
            | (super::modules::OperatingSystemFamily::BrowserWasm, _) => Self::mobile_assistant(),
            (_, super::always_on::FlowDeviceTier::LowEnd) => Self::low_end_desktop(),
            _ => Self::balanced_desktop(),
        }
    }

    pub fn low_end_desktop() -> Self {
        Self {
            activation: FlowActivationProfile::low_end_default(),
            always_on: FlowAlwaysOnProfile::low_end_24_7(),
            control: FlowControlPolicy::desktop_default(),
            audit: FlowControlAuditLog::default(),
            commands: FlowCommandRouter,
            lifecycle: FlowLifecycleController,
            audio: FlowAudioPlanner,
            modules: FlowModuleBootstrapper::new(),
            onboarding: FlowOnboardingBuilder,
            overlay: FlowOverlayController,
            permissions: FlowPermissionPlanner,
            proofing: FlowProofingPlanner::business_default(),
            recovery: FlowRecoveryPlanner,
            editor: FlowEditorAssistPlanner::coding_default(),
        }
    }

    pub fn balanced_desktop() -> Self {
        Self {
            activation: FlowActivationProfile::desktop_power_user(),
            always_on: FlowAlwaysOnProfile::balanced_desktop(),
            control: FlowControlPolicy::desktop_default(),
            audit: FlowControlAuditLog::default(),
            commands: FlowCommandRouter,
            lifecycle: FlowLifecycleController,
            audio: FlowAudioPlanner,
            modules: FlowModuleBootstrapper::new(),
            onboarding: FlowOnboardingBuilder,
            overlay: FlowOverlayController,
            permissions: FlowPermissionPlanner,
            proofing: FlowProofingPlanner::business_default(),
            recovery: FlowRecoveryPlanner,
            editor: FlowEditorAssistPlanner::coding_default(),
        }
    }

    pub fn mobile_assistant() -> Self {
        Self {
            activation: FlowActivationProfile::low_end_default(),
            always_on: FlowAlwaysOnProfile::low_end_24_7(),
            control: FlowControlPolicy::mobile_default(),
            audit: FlowControlAuditLog::default(),
            commands: FlowCommandRouter,
            lifecycle: FlowLifecycleController,
            audio: FlowAudioPlanner,
            modules: FlowModuleBootstrapper::new(),
            onboarding: FlowOnboardingBuilder,
            overlay: FlowOverlayController,
            permissions: FlowPermissionPlanner,
            proofing: FlowProofingPlanner::business_default(),
            recovery: FlowRecoveryPlanner,
            editor: FlowEditorAssistPlanner::coding_default(),
        }
    }
}
