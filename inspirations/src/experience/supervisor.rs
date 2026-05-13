use super::{
    health::FlowHealthReport,
    hostkit::FlowDefaultHostKit,
    recovery::{FlowRecoveryPlan, RecoveryEvent},
    runtime_policy::DeviceBenchmarkSnapshot,
    session::FlowSessionContext,
    wake::WakeRuntimeState,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlowRuntimeSupervisor {
    last_health: Option<FlowHealthReport>,
}

impl FlowRuntimeSupervisor {
    pub fn bootstrap(&mut self, host: &mut FlowDefaultHostKit) -> FlowSessionContext {
        let mut context = host.bootstrap();
        let _ = host.advance(
            &mut context,
            super::lifecycle::FlowRuntimeEvent::BootCompleted,
        );
        self.last_health = Some(host.health_report(&context));
        context
    }

    pub fn arm(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
    ) -> FlowHealthReport {
        let _ = host.advance(context, super::lifecycle::FlowRuntimeEvent::ResumeRequested);
        let report = host.health_report(context);
        self.last_health = Some(report.clone());
        report
    }

    pub fn note_wake_detection(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
        phrase: impl Into<String>,
    ) -> WakeRuntimeState {
        let snapshot = host.note_wake_detection(context, phrase);
        self.last_health = Some(host.health_report(context));
        snapshot
    }

    pub fn feed_audio_frame(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
        samples: &[f32],
    ) -> Option<WakeRuntimeState> {
        let snapshot = host.feed_audio_frame(context, samples)?;
        self.last_health = Some(host.health_report(context));
        Some(snapshot)
    }

    pub fn refresh_tier(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
        benchmark: DeviceBenchmarkSnapshot,
    ) -> Option<super::engine::FlowTierRefreshReport> {
        let report = host.refresh_runtime(context, benchmark);
        self.last_health = Some(host.health_report(context));
        report
    }

    pub fn evaluate_environment(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
        battery_percent: Option<u8>,
        thermal_celsius: Option<u8>,
    ) -> FlowHealthReport {
        let battery_trigger = battery_percent.map(|value| value <= 15).unwrap_or(false);
        let thermal_trigger = thermal_celsius.map(|value| value >= 82).unwrap_or(false);

        if thermal_trigger {
            let _ = host.recover(context, RecoveryEvent::ThermalPause);
        } else if battery_trigger {
            let _ = host.recover(context, RecoveryEvent::BatteryFallback);
        }

        let report = host.health_report(context);
        self.last_health = Some(report.clone());
        report
    }

    pub fn recover(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
        event: RecoveryEvent,
    ) -> FlowRecoveryPlan {
        let plan = host.recover(context, event);
        self.last_health = Some(host.health_report(context));
        plan
    }

    pub fn sync(
        &mut self,
        host: &mut FlowDefaultHostKit,
        context: &mut FlowSessionContext,
    ) -> FlowHealthReport {
        host.sync(context);
        let report = host.health_report(context);
        self.last_health = Some(report.clone());
        report
    }

    pub fn last_health(&self) -> Option<&FlowHealthReport> {
        self.last_health.as_ref()
    }
}
