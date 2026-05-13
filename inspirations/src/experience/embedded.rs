use std::path::PathBuf;

use super::{
    FlowExperienceHub, FlowHostSnapshot,
    automation::FlowSelectionExecution,
    consent::FlowConsentPlan,
    engine::{FlowCommandExecution, FlowTextExecution, FlowTierRefreshReport},
    health::FlowHealthReport,
    hostkit::FlowDefaultHostKit,
    recovery::{FlowRecoveryPlan, RecoveryEvent},
    runtime_policy::DeviceBenchmarkSnapshot,
    session::FlowSessionContext,
    supervisor::FlowRuntimeSupervisor,
    types::TypingAssistRequest,
    wake::WakeRuntimeState,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowEmbeddedHost {
    pub kit: FlowDefaultHostKit,
    pub supervisor: FlowRuntimeSupervisor,
    pub context: Option<FlowSessionContext>,
}

impl FlowEmbeddedHost {
    pub fn new(
        snapshot: FlowHostSnapshot,
        hub: FlowExperienceHub,
        state_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kit: FlowDefaultHostKit::new(snapshot, hub, state_path),
            supervisor: FlowRuntimeSupervisor::default(),
            context: None,
        }
    }

    pub fn live(
        snapshot: FlowHostSnapshot,
        hub: FlowExperienceHub,
        state_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kit: FlowDefaultHostKit::live(snapshot, hub, state_path),
            supervisor: FlowRuntimeSupervisor::default(),
            context: None,
        }
    }

    pub fn bootstrap(&mut self) -> &FlowSessionContext {
        let context = self.supervisor.bootstrap(&mut self.kit);
        self.context = Some(context);
        self.context.as_ref().expect("context initialized")
    }

    pub fn context(&self) -> Option<&FlowSessionContext> {
        self.context.as_ref()
    }

    pub fn health_report(&self) -> Option<FlowHealthReport> {
        let context = self.context.as_ref()?;
        Some(self.kit.health_report(context))
    }

    pub fn consent_plan(&self) -> Option<FlowConsentPlan> {
        let context = self.context.as_ref()?;
        Some(self.kit.live_consent_plan(context))
    }

    pub fn process_text(&mut self, request: TypingAssistRequest) -> Option<FlowTextExecution> {
        let context = self.context.as_mut()?;
        Some(self.kit.process_text(context, request))
    }

    pub fn process_command(
        &mut self,
        transcript: impl Into<String>,
    ) -> Option<FlowCommandExecution> {
        let context = self.context.as_mut()?;
        Some(self.kit.process_command(context, transcript))
    }

    pub fn rewrite_selection(&mut self) -> Option<FlowSelectionExecution> {
        let context = self.context.as_mut()?;
        self.kit.rewrite_selection(context)
    }

    pub fn advance(
        &mut self,
        event: super::lifecycle::FlowRuntimeEvent,
    ) -> Option<super::lifecycle::FlowLifecycleSnapshot> {
        let context = self.context.as_mut()?;
        Some(self.kit.advance(context, event))
    }

    pub fn note_wake_detection(&mut self, phrase: impl Into<String>) -> Option<WakeRuntimeState> {
        let context = self.context.as_mut()?;
        Some(
            self.supervisor
                .note_wake_detection(&mut self.kit, context, phrase),
        )
    }

    pub fn feed_audio_frame(&mut self, samples: &[f32]) -> Option<WakeRuntimeState> {
        let context = self.context.as_mut()?;
        self.supervisor
            .feed_audio_frame(&mut self.kit, context, samples)
    }

    pub fn refresh_runtime(
        &mut self,
        benchmark: DeviceBenchmarkSnapshot,
    ) -> Option<FlowTierRefreshReport> {
        let context = self.context.as_mut()?;
        self.supervisor
            .refresh_tier(&mut self.kit, context, benchmark)
    }

    pub fn evaluate_environment(
        &mut self,
        battery_percent: Option<u8>,
        thermal_celsius: Option<u8>,
    ) -> Option<FlowHealthReport> {
        let context = self.context.as_mut()?;
        Some(self.supervisor.evaluate_environment(
            &mut self.kit,
            context,
            battery_percent,
            thermal_celsius,
        ))
    }

    pub fn recover(&mut self, event: RecoveryEvent) -> Option<FlowRecoveryPlan> {
        let context = self.context.as_mut()?;
        Some(self.supervisor.recover(&mut self.kit, context, event))
    }
}
