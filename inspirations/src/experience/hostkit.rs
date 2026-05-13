use super::{
    FlowExperienceHub, FlowHostSnapshot,
    accessibility::FlowAccessibilityRuntime,
    bundle::FlowHostBundle,
    capture::{CpalCaptureWorker, FlowCaptureWorker},
    consent::{FlowConsentPlan, FlowConsentPlanner},
    contracts::{
        FlowModuleInstaller, FlowStateStore, MemoryPermissionGate, RecordingModuleInstaller,
    },
    engine::{FlowCommandExecution, FlowTextExecution, FlowTierRefreshReport},
    executors::NativeControlExecutor,
    health::FlowHealthReport,
    microphone::{FlowMicrophoneService, ManagedMicrophoneService},
    persistence::FlowPersistentState,
    presenters::{ManagedAudioRuntime, NativeOverlayPresenter},
    recovery::{FlowRecoveryPlan, RecoveryEvent},
    runtime_policy::DeviceBenchmarkSnapshot,
    selection::NativeSelectionBridge,
    session::FlowSessionContext,
    stores::FlowFileStateStore,
    types::TypingAssistRequest,
    wake::{FlowWakeRuntime, ManagedWakeRuntime, WakeRuntimeState},
    wakedetect::{FlowWakeInferenceWorker, OpenWakeInferenceWorker},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowDefaultHostKit {
    pub snapshot: FlowHostSnapshot,
    pub bundle: FlowHostBundle,
    pub installer: RecordingModuleInstaller,
    pub store: FlowFileStateStore,
    pub permissions: MemoryPermissionGate,
    pub executor: NativeControlExecutor,
    pub overlay: NativeOverlayPresenter,
    pub audio: ManagedAudioRuntime,
    pub capture: CpalCaptureWorker,
    pub microphone: ManagedMicrophoneService,
    pub accessibility: FlowAccessibilityRuntime,
    pub automation: NativeSelectionBridge,
    pub wake: ManagedWakeRuntime,
    pub wake_worker: OpenWakeInferenceWorker,
}

impl FlowDefaultHostKit {
    pub fn new(
        snapshot: FlowHostSnapshot,
        hub: FlowExperienceHub,
        state_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        let bundle = FlowHostBundle::for_host(&snapshot, hub);
        let accessibility = FlowAccessibilityRuntime::dry_run(snapshot.os.clone());
        Self {
            snapshot: snapshot.clone(),
            bundle,
            installer: RecordingModuleInstaller::default(),
            store: FlowFileStateStore::new(state_path),
            permissions: MemoryPermissionGate::default(),
            executor: NativeControlExecutor::new(snapshot.os.clone()),
            overlay: NativeOverlayPresenter::new(snapshot.os.clone()),
            audio: ManagedAudioRuntime::default(),
            capture: CpalCaptureWorker::new(),
            microphone: ManagedMicrophoneService::default(),
            accessibility: accessibility.clone(),
            automation: NativeSelectionBridge::with_accessibility(accessibility, true),
            wake: ManagedWakeRuntime::default(),
            wake_worker: OpenWakeInferenceWorker::default(),
        }
    }

    pub fn live(
        snapshot: FlowHostSnapshot,
        hub: FlowExperienceHub,
        state_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        let bundle = FlowHostBundle::for_host(&snapshot, hub);
        let accessibility = FlowAccessibilityRuntime::live(snapshot.os.clone());
        Self {
            snapshot: snapshot.clone(),
            bundle,
            installer: RecordingModuleInstaller::default(),
            store: FlowFileStateStore::new(state_path),
            permissions: MemoryPermissionGate::default(),
            executor: NativeControlExecutor::live(snapshot.os.clone()),
            overlay: NativeOverlayPresenter::live(snapshot.os.clone()),
            audio: ManagedAudioRuntime::default(),
            capture: CpalCaptureWorker::live(),
            microphone: ManagedMicrophoneService::default(),
            accessibility: accessibility.clone(),
            automation: NativeSelectionBridge::with_accessibility(accessibility, false),
            wake: ManagedWakeRuntime::default(),
            wake_worker: OpenWakeInferenceWorker::default(),
        }
    }

    pub fn bootstrap(&mut self) -> FlowSessionContext {
        let report = self
            .bundle
            .bootstrap(&self.snapshot, &mut self.installer, &mut self.store);
        let context = report.context;
        self.bundle
            .sync_presenters(&context, &mut self.overlay, &mut self.audio);
        self.capture.configure(&context.audio);
        self.capture.start();
        self.microphone.configure(&context.audio);
        self.microphone.arm();
        self.wake.configure(&context.activation, &context.audio);
        self.wake_worker.configure(&context.activation);
        self.wake_worker.arm();
        self.wake.sync_lifecycle(&context.lifecycle.state);
        if let Some(state) = self.store.load_state() {
            let _ = self.restore_permissions(&state);
        }
        context
    }

    pub fn process_text(
        &mut self,
        context: &mut FlowSessionContext,
        request: TypingAssistRequest,
    ) -> FlowTextExecution {
        let execution = self.bundle.engine.process_text(
            context,
            request,
            &mut self.permissions,
            &mut self.executor,
        );
        self.sync(context);
        execution
    }

    pub fn process_command(
        &mut self,
        context: &mut FlowSessionContext,
        transcript: impl Into<String>,
    ) -> FlowCommandExecution {
        let execution = self.bundle.engine.process_command(
            context,
            transcript,
            &mut self.permissions,
            &mut self.executor,
        );
        self.sync(context);
        execution
    }

    pub fn rewrite_selection(
        &mut self,
        context: &mut FlowSessionContext,
    ) -> Option<super::automation::FlowSelectionExecution> {
        let execution =
            self.bundle
                .rewrite_selection(context, &mut self.permissions, &mut self.automation);
        self.sync(context);
        execution
    }

    pub fn refresh_runtime(
        &mut self,
        context: &mut FlowSessionContext,
        benchmark: DeviceBenchmarkSnapshot,
    ) -> Option<FlowTierRefreshReport> {
        let report = self.bundle.engine.refresh_runtime(
            context,
            benchmark,
            &mut self.installer,
            &mut self.store,
        );
        self.sync(context);
        report
    }

    pub fn advance(
        &mut self,
        context: &mut FlowSessionContext,
        event: super::lifecycle::FlowRuntimeEvent,
    ) -> super::lifecycle::FlowLifecycleSnapshot {
        let snapshot = self.bundle.advance(context, event);
        self.sync(context);
        snapshot
    }

    pub fn recover(
        &mut self,
        context: &mut FlowSessionContext,
        event: RecoveryEvent,
    ) -> FlowRecoveryPlan {
        let plan = self.bundle.recovery_plan(event.clone());

        for action in &plan.actions {
            if action.persist_state {
                self.persist(context);
            }
            if action.reload_modules {
                let required: Vec<_> = context
                    .install_state
                    .installed_required_modules()
                    .into_iter()
                    .cloned()
                    .collect();
                let receipts = self.installer.install_modules(&required);
                context.install_state.apply_install_receipts(&receipts);
            }
            if action.restart_audio || action.reset_overlay {
                if action.restart_audio {
                    self.microphone.restart();
                }
                self.sync(context);
            }
        }

        match event {
            RecoveryEvent::Suspend
            | RecoveryEvent::ThermalPause
            | RecoveryEvent::BatteryFallback
            | RecoveryEvent::MicrophoneLost => {
                let _ = self.advance(context, super::lifecycle::FlowRuntimeEvent::PauseRequested);
            }
            RecoveryEvent::Resume | RecoveryEvent::MicrophoneRestored => {
                let _ = self.advance(context, super::lifecycle::FlowRuntimeEvent::ResumeRequested);
            }
            RecoveryEvent::RuntimeCrash => {
                let _ = self.advance(context, super::lifecycle::FlowRuntimeEvent::BootCompleted);
            }
        }

        plan
    }

    pub fn sync(&mut self, context: &FlowSessionContext) {
        self.bundle
            .sync_presenters(context, &mut self.overlay, &mut self.audio);
        self.capture.configure(&context.audio);
        self.microphone.configure(&context.audio);
        match context.lifecycle.state {
            super::lifecycle::FlowRuntimeState::Listening
            | super::lifecycle::FlowRuntimeState::Overlay
            | super::lifecycle::FlowRuntimeState::CommandMode => {
                self.microphone.arm();
                self.capture.start();
                self.wake_worker.arm();
            }
            super::lifecycle::FlowRuntimeState::Dictating => {
                self.microphone.stream();
                self.capture.start();
                self.wake_worker.arm();
            }
            super::lifecycle::FlowRuntimeState::Sleeping
            | super::lifecycle::FlowRuntimeState::Paused
            | super::lifecycle::FlowRuntimeState::ColdBoot => {
                self.microphone.pause();
                self.capture.stop();
                self.wake_worker.disarm();
            }
        }
        self.wake.configure(&context.activation, &context.audio);
        self.wake_worker.configure(&context.activation);
        self.wake.sync_lifecycle(&context.lifecycle.state);
        self.persist(context);
    }

    pub fn note_wake_detection(
        &mut self,
        context: &mut FlowSessionContext,
        phrase: impl Into<String>,
    ) -> WakeRuntimeState {
        let phrase = phrase.into();
        let detected = self.wake_worker.evaluate_phrase(&phrase).unwrap_or(phrase);
        self.wake.note_detection(detected.clone());
        let _ = self.advance(
            context,
            super::lifecycle::FlowRuntimeEvent::WakeWordDetected(detected),
        );
        self.wake.snapshot()
    }

    pub fn feed_audio_frame(
        &mut self,
        context: &mut FlowSessionContext,
        samples: &[f32],
    ) -> Option<WakeRuntimeState> {
        let capture_report = self.capture.feed_samples(samples);
        let detected = self
            .wake_worker
            .evaluate_audio_frame(samples, capture_report.speech_detected)?;
        self.wake.note_detection(detected.clone());
        let _ = self.advance(
            context,
            super::lifecycle::FlowRuntimeEvent::WakeWordDetected(detected),
        );
        Some(self.wake.snapshot())
    }

    pub fn health_report(&self, context: &FlowSessionContext) -> FlowHealthReport {
        FlowHealthReport::evaluate(self, context)
    }

    pub fn live_consent_plan(&self, context: &FlowSessionContext) -> FlowConsentPlan {
        FlowConsentPlanner::for_live_host(self, context)
    }

    fn persist(&mut self, context: &FlowSessionContext) {
        let state = FlowPersistentState::from_runtime(
            &context.install_state,
            &context.audit,
            self.bundle.engine.benchmark_history.clone(),
        );
        self.store.save_state(state);
    }

    fn restore_permissions(&mut self, state: &FlowPersistentState) -> usize {
        let mut restored = 0;
        for approval in &state.approvals {
            if approval.granted {
                if let Some(capability) = parse_capability(&approval.capability) {
                    self.permissions.grant(capability);
                    restored += 1;
                }
            }
        }
        restored
    }
}

fn parse_capability(value: &str) -> Option<super::control::ControlCapability> {
    Some(match value {
        "ReadClipboard" => super::control::ControlCapability::ReadClipboard,
        "WriteClipboard" => super::control::ControlCapability::WriteClipboard,
        "ReadSelection" => super::control::ControlCapability::ReadSelection,
        "ReplaceSelection" => super::control::ControlCapability::ReplaceSelection,
        "SimulateShortcut" => super::control::ControlCapability::SimulateShortcut,
        "OpenUrl" => super::control::ControlCapability::OpenUrl,
        "OpenApplication" => super::control::ControlCapability::OpenApplication,
        "OpenFile" => super::control::ControlCapability::OpenFile,
        "RevealFile" => super::control::ControlCapability::RevealFile,
        "CreateDraftFile" => super::control::ControlCapability::CreateDraftFile,
        "FocusWindow" => super::control::ControlCapability::FocusWindow,
        "MediaPlayback" => super::control::ControlCapability::MediaPlayback,
        "VolumeControl" => super::control::ControlCapability::VolumeControl,
        "BrightnessControl" => super::control::ControlCapability::BrightnessControl,
        "SystemSearch" => super::control::ControlCapability::SystemSearch,
        "Notification" => super::control::ControlCapability::Notification,
        "ShellCommand" => super::control::ControlCapability::ShellCommand,
        _ => return None,
    })
}
