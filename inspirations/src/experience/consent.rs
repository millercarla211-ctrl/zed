use super::{
    control::SafetyRequirement, hostkit::FlowDefaultHostKit, permissions::PermissionRequirement,
    session::FlowSessionContext,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentSeverity {
    Normal,
    Elevated,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentPrompt {
    pub title: String,
    pub body: String,
    pub severity: ConsentSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowConsentPlan {
    pub prompts: Vec<ConsentPrompt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowConsentPlanner;

impl FlowConsentPlanner {
    pub fn for_live_host(
        host: &FlowDefaultHostKit,
        context: &FlowSessionContext,
    ) -> FlowConsentPlan {
        let mut prompts = Vec::new();

        for permission in &context.permissions.required {
            prompts.push(prompt_for_permission(permission));
        }

        for rule in &context.control.rules {
            if matches!(
                rule.safety,
                SafetyRequirement::Confirm | SafetyRequirement::ExplicitConsent
            ) {
                prompts.push(ConsentPrompt {
                    title: format!("Approve {:?}", rule.capability),
                    body: format!(
                        "Flow may request {:?} on this host. Review it during first-run native setup.",
                        rule.capability
                    ),
                    severity: if matches!(rule.safety, SafetyRequirement::ExplicitConsent) {
                        ConsentSeverity::Critical
                    } else {
                        ConsentSeverity::Elevated
                    },
                });
            }
        }

        if !host.executor.dry_run {
            prompts.push(ConsentPrompt {
                title: "Live native execution enabled".to_string(),
                body: "This host kit is configured for live native actions instead of dry-run planning.".to_string(),
                severity: ConsentSeverity::Critical,
            });
        }

        FlowConsentPlan { prompts }
    }
}

fn prompt_for_permission(permission: &PermissionRequirement) -> ConsentPrompt {
    ConsentPrompt {
        title: permission.title.to_string(),
        body: permission.rationale.to_string(),
        severity: if permission.required {
            ConsentSeverity::Elevated
        } else {
            ConsentSeverity::Normal
        },
    }
}
