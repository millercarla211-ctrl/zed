use super::{
    activation::FlowActivationProfile,
    always_on::FlowAlwaysOnProfile,
    audio::{FlowAudioPipeline, FlowAudioPlanner},
    audit::FlowControlAuditLog,
    command::{FlowCommandPlan, FlowCommandRouter},
    control::{ControlActionPlan, FlowControlPolicy},
    facade::FlowProductSurface,
    installer::{FlowInstallState, FlowInstallerFacade},
    lifecycle::{FlowLifecycleController, FlowLifecycleSnapshot},
    modules::{FlowModuleInstallPlan, OperatingSystemFamily},
    onboarding::{FlowOnboardingBuilder, FlowOnboardingPlan},
    overlay::{FlowOverlayController, FlowOverlayState},
    permissions::{FlowPermissionBundle, FlowPermissionPlanner},
    proofing::{FlowProofingPlanner, ProofingIssue},
    runtime_policy::{DeviceBenchmarkSnapshot, FlowRuntimeTierPolicy},
    types::{
        AppContext, TextCommandRequest, TextCommandResult, TypingAssistRequest, TypingAssistResult,
    },
    typing::FlowTypingAssistant,
    workspace::FlowExperienceHub,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowSessionContext {
    pub os: OperatingSystemFamily,
    pub install_plan: FlowModuleInstallPlan,
    pub install_state: FlowInstallState,
    pub onboarding: FlowOnboardingPlan,
    pub lifecycle: FlowLifecycleSnapshot,
    pub permissions: FlowPermissionBundle,
    pub audio: FlowAudioPipeline,
    pub overlay: FlowOverlayState,
    pub activation: FlowActivationProfile,
    pub always_on: FlowAlwaysOnProfile,
    pub control: FlowControlPolicy,
    pub audit: FlowControlAuditLog,
    pub proofing: FlowProofingPlanner,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowSessionRuntime {
    pub installer: FlowInstallerFacade,
    pub tier_policy: FlowRuntimeTierPolicy,
    pub commands: FlowCommandRouter,
    pub typing: FlowTypingAssistant,
    pub hub: FlowExperienceHub,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowTextPass {
    pub typing: TypingAssistResult,
    pub proofing: Vec<ProofingIssue>,
    pub insert_action: Option<ControlActionPlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowCommandPass {
    pub command: FlowCommandPlan,
    pub text_command: TextCommandResult,
}

impl FlowSessionRuntime {
    pub fn from_surface(surface: &FlowProductSurface, hub: FlowExperienceHub) -> Self {
        Self {
            installer: FlowInstallerFacade {
                bootstrapper: surface.modules.clone(),
                tier_policy: FlowRuntimeTierPolicy::new(),
            },
            tier_policy: FlowRuntimeTierPolicy::new(),
            commands: surface.commands.clone(),
            typing: FlowTypingAssistant::default(),
            hub,
        }
    }

    pub fn first_run_context(
        &self,
        surface: &FlowProductSurface,
        os: OperatingSystemFamily,
    ) -> FlowSessionContext {
        let tier = surface.always_on.tier.clone();
        let (install_plan, install_state) = self.installer.first_run_state(os.clone(), tier);
        let onboarding = FlowOnboardingBuilder::build(os.clone());
        let lifecycle =
            FlowLifecycleController.initial_snapshot(&surface.activation, &surface.always_on);
        let permissions = FlowPermissionPlanner::build(os.clone());
        let audio = FlowAudioPlanner::build(&os, &surface.activation, &surface.always_on);
        let overlay = if matches!(
            surface.always_on.tier,
            super::always_on::FlowDeviceTier::LowEnd
        ) {
            FlowOverlayController::low_end_default()
        } else {
            FlowOverlayController::expanded_default()
        };

        FlowSessionContext {
            os,
            install_plan,
            install_state,
            onboarding,
            lifecycle,
            permissions,
            audio,
            overlay,
            activation: surface.activation.clone(),
            always_on: surface.always_on.clone(),
            control: surface.control.clone(),
            audit: surface.audit.clone(),
            proofing: surface.proofing.clone(),
        }
    }

    pub fn process_text(
        &self,
        context: &FlowSessionContext,
        request: TypingAssistRequest,
    ) -> FlowTextPass {
        let original_text = request.text.clone();
        let typing = self
            .typing
            .process(request)
            .unwrap_or_else(|error| TypingAssistResult {
                original_text: original_text.clone(),
                final_text: original_text,
                issues: Vec::new(),
                expanded_snippets: Vec::new(),
                normalized_terms: Vec::new(),
                notes: vec![format!("Typing assistance failed: {error}")],
            });
        let proofing = context.proofing.inspect(&typing.final_text);
        let insert_action = if typing.final_text != typing.original_text {
            Some(context.control.plan_text_insert(typing.final_text.clone()))
        } else {
            None
        };

        FlowTextPass {
            typing,
            proofing,
            insert_action,
        }
    }

    pub fn route_command(
        &self,
        context: &FlowSessionContext,
        transcript: impl Into<String>,
    ) -> FlowCommandPass {
        let transcript = transcript.into();
        let command = self.commands.route(transcript.clone(), &context.control);
        let app_context = AppContext::default();
        let text_command = self
            .typing
            .execute_command(TextCommandRequest {
                selected_text: transcript,
                command: "grammar".to_string(),
                app_context: app_context.clone(),
                styles: self.hub.styles_for_context(&app_context),
            })
            .unwrap_or_else(|error| TextCommandResult {
                original_text: String::new(),
                transformed_text: String::new(),
                applied_command: "grammar".to_string(),
                notes: vec![format!("Command rewrite failed: {error}")],
            });

        FlowCommandPass {
            command,
            text_command,
        }
    }

    pub fn reevaluate_modules(
        &self,
        context: &FlowSessionContext,
        benchmark: &DeviceBenchmarkSnapshot,
    ) -> Option<super::installer::ModuleTransitionPlan> {
        self.installer.reevaluate(&context.install_state, benchmark)
    }
}
