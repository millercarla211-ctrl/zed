use super::{
    always_on::FlowDeviceTier,
    audit::{ApprovalScope, FlowControlAuditLog},
    installer::{FlowInstallState, InstalledModuleRecord, ModuleInstallStatus},
    modules::{FlowModuleInstallPlan, OperatingSystemFamily},
    runtime_policy::DeviceBenchmarkSnapshot,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedApprovalRecord {
    pub capability: String,
    pub scope: ApprovalScope,
    pub granted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedModuleRecord {
    pub id: String,
    pub status: ModuleInstallStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowPersistentState {
    pub os: OperatingSystemFamily,
    pub tier: FlowDeviceTier,
    pub modules: Vec<PersistedModuleRecord>,
    pub approvals: Vec<PersistedApprovalRecord>,
    pub benchmark_history: Vec<DeviceBenchmarkSnapshot>,
}

impl FlowPersistentState {
    pub fn from_runtime(
        install_state: &FlowInstallState,
        audit: &FlowControlAuditLog,
        benchmark_history: Vec<DeviceBenchmarkSnapshot>,
    ) -> Self {
        let approvals = audit
            .approvals()
            .iter()
            .map(|approval| PersistedApprovalRecord {
                capability: format!("{:?}", approval.capability),
                scope: approval.scope.clone(),
                granted: approval.granted,
            })
            .collect();
        let modules = install_state
            .records
            .values()
            .map(|record| PersistedModuleRecord {
                id: record.descriptor.id.to_string(),
                status: record.status.clone(),
            })
            .collect();

        Self {
            os: install_state.os.clone(),
            tier: install_state.current_tier.clone(),
            modules,
            approvals,
            benchmark_history,
        }
    }

    pub fn installed_modules(&self) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|record| matches!(record.status, ModuleInstallStatus::Installed))
            .map(|record| record.id.as_str())
            .collect()
    }

    pub fn merge_with_plan(&self, plan: &FlowModuleInstallPlan) -> FlowInstallState {
        let mut install_state = FlowInstallState::from_plan(plan);
        install_state.current_tier = self.tier.clone();

        for module in &self.modules {
            if let Some(record) = install_state.records.get_mut(&module.id) {
                record.status = module.status.clone();
            }
        }

        install_state
    }
}

impl PersistedModuleRecord {
    pub fn from_runtime(record: &InstalledModuleRecord) -> Self {
        Self {
            id: record.descriptor.id.to_string(),
            status: record.status.clone(),
        }
    }
}
