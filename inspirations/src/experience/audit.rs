use super::control::ControlCapability;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    Once,
    Session,
    Application,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlApproval {
    pub capability: ControlCapability,
    pub scope: ApprovalScope,
    pub granted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionAuditEntry {
    pub capability: ControlCapability,
    pub surface: String,
    pub description: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowControlAuditLog {
    approvals: Vec<ControlApproval>,
    entries: Vec<ActionAuditEntry>,
}

impl FlowControlAuditLog {
    pub fn grant(&mut self, capability: ControlCapability, scope: ApprovalScope) {
        self.approvals.push(ControlApproval {
            capability,
            scope,
            granted: true,
        });
    }

    pub fn record(
        &mut self,
        capability: ControlCapability,
        surface: impl Into<String>,
        description: impl Into<String>,
        approved: bool,
    ) {
        self.entries.push(ActionAuditEntry {
            capability,
            surface: surface.into(),
            description: description.into(),
            approved,
        });
    }

    pub fn approvals(&self) -> &[ControlApproval] {
        &self.approvals
    }

    pub fn entries(&self) -> &[ActionAuditEntry] {
        &self.entries
    }
}
