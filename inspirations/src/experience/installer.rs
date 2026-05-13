use std::collections::BTreeMap;

use super::{
    always_on::FlowDeviceTier,
    contracts::InstalledModuleReceipt,
    modules::{
        FlowModuleBootstrapper, FlowModuleDescriptor, FlowModuleInstallPlan, InstallTrigger,
        OperatingSystemFamily,
    },
    runtime_policy::{DeviceBenchmarkSnapshot, FlowRuntimeTierPolicy, TierAdjustment},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleInstallStatus {
    Pending,
    Installed,
    Deferred,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModuleRecord {
    pub descriptor: FlowModuleDescriptor,
    pub status: ModuleInstallStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowInstallState {
    pub os: OperatingSystemFamily,
    pub current_tier: FlowDeviceTier,
    pub records: BTreeMap<String, InstalledModuleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTransitionPlan {
    pub from_tier: FlowDeviceTier,
    pub to_tier: FlowDeviceTier,
    pub install_now: Vec<FlowModuleDescriptor>,
    pub keep_installed: Vec<String>,
    pub defer_again: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowInstallerFacade {
    pub bootstrapper: FlowModuleBootstrapper,
    pub tier_policy: FlowRuntimeTierPolicy,
}

impl Default for FlowInstallerFacade {
    fn default() -> Self {
        Self {
            bootstrapper: FlowModuleBootstrapper::new(),
            tier_policy: FlowRuntimeTierPolicy::new(),
        }
    }
}

impl FlowInstallerFacade {
    pub fn first_run_state(
        &self,
        os: OperatingSystemFamily,
        tier: FlowDeviceTier,
    ) -> (FlowModuleInstallPlan, FlowInstallState) {
        let plan = self.bootstrapper.first_run_plan(os.clone(), tier.clone());
        let state = FlowInstallState::from_plan(&plan);
        (plan, state)
    }

    pub fn reevaluate(
        &self,
        state: &FlowInstallState,
        benchmark: &DeviceBenchmarkSnapshot,
    ) -> Option<ModuleTransitionPlan> {
        let recommendation = self
            .tier_policy
            .evaluate(state.current_tier.clone(), benchmark);
        let target_tier = match recommendation.adjustment {
            TierAdjustment::Keep => return None,
            TierAdjustment::Promote(ref tier) | TierAdjustment::Demote(ref tier) => tier.clone(),
        };

        if target_tier == state.current_tier {
            return None;
        }

        let plan = self.bootstrapper.plan(
            InstallTrigger::Upgrade,
            state.os.clone(),
            target_tier.clone(),
        );
        let keep_installed = state
            .records
            .values()
            .filter(|record| matches!(record.status, ModuleInstallStatus::Installed))
            .map(|record| record.descriptor.id.to_string())
            .collect();
        let defer_again = state
            .records
            .values()
            .filter(|record| matches!(record.status, ModuleInstallStatus::Deferred))
            .map(|record| record.descriptor.id.to_string())
            .collect();

        Some(ModuleTransitionPlan {
            from_tier: state.current_tier.clone(),
            to_tier: target_tier,
            install_now: plan.modules,
            keep_installed,
            defer_again,
            reason: recommendation.reason,
        })
    }
}

impl FlowInstallState {
    pub fn from_plan(plan: &FlowModuleInstallPlan) -> Self {
        let mut records = BTreeMap::new();

        for module in &plan.modules {
            records.insert(
                module.id.to_string(),
                InstalledModuleRecord {
                    descriptor: module.clone(),
                    status: ModuleInstallStatus::Pending,
                },
            );
        }

        for module in &plan.deferred_modules {
            records.insert(
                module.id.to_string(),
                InstalledModuleRecord {
                    descriptor: module.clone(),
                    status: ModuleInstallStatus::Deferred,
                },
            );
        }

        Self {
            os: plan.os.clone(),
            current_tier: plan.tier.clone(),
            records,
        }
    }

    pub fn installed_required_modules(&self) -> Vec<&FlowModuleDescriptor> {
        self.records
            .values()
            .filter(|record| {
                matches!(record.status, ModuleInstallStatus::Installed)
                    && record.descriptor.required
            })
            .map(|record| &record.descriptor)
            .collect()
    }

    pub fn apply_transition(&mut self, plan: &ModuleTransitionPlan) {
        self.current_tier = plan.to_tier.clone();

        for module_id in &plan.defer_again {
            if let Some(record) = self.records.get_mut(module_id) {
                record.status = ModuleInstallStatus::Deferred;
            }
        }

        for descriptor in &plan.install_now {
            self.records.insert(
                descriptor.id.to_string(),
                InstalledModuleRecord {
                    descriptor: descriptor.clone(),
                    status: ModuleInstallStatus::Pending,
                },
            );
        }
    }

    pub fn apply_install_receipts(&mut self, receipts: &[InstalledModuleReceipt]) {
        for receipt in receipts {
            if let Some(record) = self.records.get_mut(&receipt.module_id) {
                record.status = if receipt.installed {
                    ModuleInstallStatus::Installed
                } else {
                    ModuleInstallStatus::Failed
                };
            }
        }
    }
}
