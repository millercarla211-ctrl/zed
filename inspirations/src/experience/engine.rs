use super::{
    FlowExperienceHub, FlowProductSurface,
    audit::ApprovalScope,
    contracts::{
        ExecutedActionReceipt, FlowControlExecutor, FlowHostSnapshot, FlowModuleInstaller,
        FlowPermissionGate, FlowStateStore, InstalledModuleReceipt,
    },
    installer::ModuleTransitionPlan,
    persistence::FlowPersistentState,
    session::{FlowCommandPass, FlowSessionContext, FlowSessionRuntime, FlowTextPass},
    types::{AppContext, TypingAssistRequest},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowBootstrapReport {
    pub surface: FlowProductSurface,
    pub context: FlowSessionContext,
    pub restored_state: Option<FlowPersistentState>,
    pub installed_modules: Vec<InstalledModuleReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowTextExecution {
    pub pass: FlowTextPass,
    pub insert_receipt: Option<ExecutedActionReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowCommandExecution {
    pub pass: FlowCommandPass,
    pub action_receipts: Vec<ExecutedActionReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowTierRefreshReport {
    pub transition: ModuleTransitionPlan,
    pub installed_modules: Vec<InstalledModuleReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowEngine {
    pub surface: FlowProductSurface,
    pub session: FlowSessionRuntime,
    pub benchmark_history: Vec<super::runtime_policy::DeviceBenchmarkSnapshot>,
}

impl FlowEngine {
    pub fn for_host(snapshot: &FlowHostSnapshot, hub: FlowExperienceHub) -> Self {
        let surface = FlowProductSurface::for_host(
            &snapshot.host_label,
            snapshot.ram_gb,
            snapshot.vram_gb,
            snapshot.cpu_only,
        );
        let session = FlowSessionRuntime::from_surface(&surface, hub);

        Self {
            surface,
            session,
            benchmark_history: Vec::new(),
        }
    }

    pub fn bootstrap_host<I, S>(
        &self,
        snapshot: &FlowHostSnapshot,
        installer: &mut I,
        store: &mut S,
    ) -> FlowBootstrapReport
    where
        I: FlowModuleInstaller,
        S: FlowStateStore,
    {
        let restored_state = store.load_state();
        let mut context = self
            .session
            .first_run_context(&self.surface, snapshot.os.clone());

        if let Some(ref persisted) = restored_state {
            context.install_state = persisted.merge_with_plan(&context.install_plan);
        }

        let installed_modules = installer.install_modules(&context.install_plan.modules);
        context
            .install_state
            .apply_install_receipts(&installed_modules);
        let persistent = FlowPersistentState::from_runtime(
            &context.install_state,
            &context.audit,
            self.benchmark_history.clone(),
        );
        store.save_state(persistent);

        FlowBootstrapReport {
            surface: self.surface.clone(),
            context,
            restored_state,
            installed_modules,
        }
    }

    pub fn process_text<P, E>(
        &self,
        context: &mut FlowSessionContext,
        request: TypingAssistRequest,
        permissions: &mut P,
        executor: &mut E,
    ) -> FlowTextExecution
    where
        P: FlowPermissionGate,
        E: FlowControlExecutor,
    {
        let pass = self.session.process_text(context, request);
        let insert_receipt = pass.insert_action.as_ref().and_then(|action| {
            if self.approve_action(
                context,
                permissions,
                &action.capability,
                &action.description,
                action.requires_user_confirmation,
            ) {
                let receipt = executor.execute(action);
                context.audit.record(
                    action.capability.clone(),
                    format!("{:?}", context.control.surface),
                    action.description.clone(),
                    receipt.executed,
                );
                Some(receipt)
            } else {
                context.audit.record(
                    action.capability.clone(),
                    format!("{:?}", context.control.surface),
                    action.description.clone(),
                    false,
                );
                None
            }
        });

        FlowTextExecution {
            pass,
            insert_receipt,
        }
    }

    pub fn process_command<P, E>(
        &self,
        context: &mut FlowSessionContext,
        transcript: impl Into<String>,
        permissions: &mut P,
        executor: &mut E,
    ) -> FlowCommandExecution
    where
        P: FlowPermissionGate,
        E: FlowControlExecutor,
    {
        let transcript = transcript.into();
        let pass = self.session.route_command(context, transcript.clone());
        let mut action_receipts = Vec::new();

        for action in &pass.command.control_actions {
            let approved = self.approve_action(
                context,
                permissions,
                &action.capability,
                &action.description,
                action.requires_user_confirmation,
            );
            if approved {
                let receipt = executor.execute(action);
                context.audit.record(
                    action.capability.clone(),
                    format!("{:?}", context.control.surface),
                    action.description.clone(),
                    receipt.executed,
                );
                action_receipts.push(receipt);
            } else {
                context.audit.record(
                    action.capability.clone(),
                    format!("{:?}", context.control.surface),
                    action.description.clone(),
                    false,
                );
            }
        }

        if matches!(
            pass.command.intent,
            super::command::FlowCommandIntent::RewriteSelection
        ) {
            let app_context = AppContext::default();
            let _ = self.session.process_text(
                context,
                TypingAssistRequest {
                    text: transcript.clone(),
                    app_context: app_context.clone(),
                    dictionary: self.session.hub.dictionary_for_context(),
                    snippets: self.session.hub.snippets_for_context(),
                    styles: self.session.hub.styles_for_context(&app_context),
                    auto_correct: true,
                    expand_snippets: true,
                },
            );
        }

        FlowCommandExecution {
            pass,
            action_receipts,
        }
    }

    pub fn refresh_runtime<I, S>(
        &mut self,
        context: &mut FlowSessionContext,
        benchmark: super::runtime_policy::DeviceBenchmarkSnapshot,
        installer: &mut I,
        store: &mut S,
    ) -> Option<FlowTierRefreshReport>
    where
        I: FlowModuleInstaller,
        S: FlowStateStore,
    {
        self.benchmark_history.push(benchmark.clone());
        let transition = self.session.reevaluate_modules(context, &benchmark)?;
        let installed_modules = installer.install_modules(&transition.install_now);
        context.install_state.apply_transition(&transition);
        context
            .install_state
            .apply_install_receipts(&installed_modules);
        let persistent = FlowPersistentState::from_runtime(
            &context.install_state,
            &context.audit,
            self.benchmark_history.clone(),
        );
        store.save_state(persistent);

        Some(FlowTierRefreshReport {
            transition,
            installed_modules,
        })
    }

    pub fn advance_lifecycle(
        &self,
        context: &mut FlowSessionContext,
        event: super::lifecycle::FlowRuntimeEvent,
    ) -> super::lifecycle::FlowLifecycleSnapshot {
        let next = self.surface.lifecycle.transition(&context.lifecycle, event);
        context.overlay = super::overlay::FlowOverlayController::mode_for_lifecycle(
            &next.state,
            &context.overlay,
        );
        context.lifecycle = next.clone();
        next
    }

    fn approve_action<P>(
        &self,
        context: &mut FlowSessionContext,
        permissions: &mut P,
        capability: &super::control::ControlCapability,
        reason: &str,
        requires_confirmation: bool,
    ) -> bool
    where
        P: FlowPermissionGate,
    {
        if !requires_confirmation {
            return true;
        }

        if permissions.is_granted(capability) {
            return true;
        }

        let approved = permissions.request(capability, reason);
        if approved {
            context
                .audit
                .grant(capability.clone(), ApprovalScope::Session);
        }
        approved
    }
}
