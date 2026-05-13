use super::{
    FlowExperienceHub,
    audio::{FlowAudioPipeline, FlowAudioPlanner},
    automation::{FlowAutomationEngine, FlowSelectionExecution, FlowShortcutExecution},
    bridges::ClipboardAutomationBridge,
    contracts::{
        FlowAudioRuntime, FlowAutomationBridge, FlowHostSnapshot, FlowModuleInstaller,
        FlowOverlayPresenter, FlowPermissionGate, FlowStateStore,
    },
    engine::{FlowBootstrapReport, FlowEngine},
    executors::NativeControlExecutor,
    onboarding::{FlowOnboardingBuilder, FlowOnboardingPlan},
    overlay::{FlowOverlayController, FlowOverlayState},
    permissions::{FlowPermissionBundle, FlowPermissionPlanner},
    recovery::{FlowRecoveryPlan, FlowRecoveryPlanner, RecoveryEvent},
    session::FlowSessionContext,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowHostBundle {
    pub engine: FlowEngine,
    pub permissions: FlowPermissionBundle,
    pub onboarding: FlowOnboardingPlan,
    pub audio: FlowAudioPipeline,
    pub overlay: FlowOverlayState,
    pub recovery: FlowRecoveryPlanner,
}

impl FlowHostBundle {
    pub fn for_host(snapshot: &FlowHostSnapshot, hub: FlowExperienceHub) -> Self {
        let engine = FlowEngine::for_host(snapshot, hub);
        let permissions = FlowPermissionPlanner::build(snapshot.os.clone());
        let onboarding = FlowOnboardingBuilder::build(snapshot.os.clone());
        let audio = FlowAudioPlanner::build(
            &snapshot.os,
            &engine.surface.activation,
            &engine.surface.always_on,
        );
        let overlay = if matches!(
            engine.surface.always_on.tier,
            super::always_on::FlowDeviceTier::LowEnd
        ) {
            FlowOverlayController::low_end_default()
        } else {
            FlowOverlayController::expanded_default()
        };

        Self {
            engine,
            permissions,
            onboarding,
            audio,
            overlay,
            recovery: FlowRecoveryPlanner,
        }
    }

    pub fn bootstrap<I, S>(
        &self,
        snapshot: &FlowHostSnapshot,
        installer: &mut I,
        store: &mut S,
    ) -> FlowBootstrapReport
    where
        I: FlowModuleInstaller,
        S: FlowStateStore,
    {
        self.engine.bootstrap_host(snapshot, installer, store)
    }

    pub fn advance(
        &self,
        context: &mut FlowSessionContext,
        event: super::lifecycle::FlowRuntimeEvent,
    ) -> super::lifecycle::FlowLifecycleSnapshot {
        self.engine.advance_lifecycle(context, event)
    }

    pub fn recovery_plan(&self, event: RecoveryEvent) -> FlowRecoveryPlan {
        FlowRecoveryPlanner::plan(event)
    }

    pub fn sync_presenters<P, A>(
        &self,
        context: &FlowSessionContext,
        overlay: &mut P,
        audio: &mut A,
    ) where
        P: FlowOverlayPresenter,
        A: FlowAudioRuntime,
    {
        overlay.present(&context.overlay);
        audio.configure(&context.audio);
    }

    pub fn native_executor(&self, snapshot: &FlowHostSnapshot) -> NativeControlExecutor {
        NativeControlExecutor::new(snapshot.os.clone())
    }

    pub fn live_native_executor(&self, snapshot: &FlowHostSnapshot) -> NativeControlExecutor {
        NativeControlExecutor::live(snapshot.os.clone())
    }

    pub fn clipboard_bridge(&self, snapshot: &FlowHostSnapshot) -> ClipboardAutomationBridge {
        ClipboardAutomationBridge::new(snapshot.os.clone())
    }

    pub fn live_clipboard_bridge(&self, snapshot: &FlowHostSnapshot) -> ClipboardAutomationBridge {
        ClipboardAutomationBridge::live(snapshot.os.clone())
    }

    pub fn rewrite_selection<P, A>(
        &self,
        context: &mut FlowSessionContext,
        permissions: &mut P,
        bridge: &mut A,
    ) -> Option<FlowSelectionExecution>
    where
        P: FlowPermissionGate,
        A: FlowAutomationBridge,
    {
        FlowAutomationEngine.rewrite_selection(&self.engine, context, permissions, bridge)
    }

    pub fn dispatch_shortcut<A>(
        &self,
        bridge: &mut A,
        shortcut: impl Into<String>,
    ) -> FlowShortcutExecution
    where
        A: FlowAutomationBridge,
    {
        FlowAutomationEngine.dispatch_shortcut(bridge, shortcut)
    }
}
