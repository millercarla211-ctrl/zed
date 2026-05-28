use serde_json::Value;

mod receipt_strings;

use self::receipt_strings::{
    receipt_string_array_field, receipt_string_field, receipt_string_values_field,
};
use super::{
    DxAgentActionErrorSummary, DxAgentContractSummary, DxAgentImportSummary, DxAgentReceipt,
    DxAgentReceiptInboxSummary, DxAgentReceiptIndexSummary, DxAgentRecoveryControlCounts,
    DxAgentReleaseGateSummary, array_field, bool_field, usize_field,
};

pub(super) fn contract_summary(value: Option<&Value>, root_exists: bool) -> DxAgentContractSummary {
    let provider_catalog = value.and_then(|value| value.get("provider_catalog"));
    let redaction = value.and_then(|value| value.get("redaction"));
    let status = value
        .and_then(|value| receipt_string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_contract_receipt".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });

    let exports_secret_values = redaction
        .and_then(|value| bool_field(value, &["exports_secret_values"]))
        .unwrap_or(false);
    let exports_account_targets = redaction
        .and_then(|value| bool_field(value, &["exports_account_targets"]))
        .unwrap_or(false);
    let exports_automation_bodies = redaction
        .and_then(|value| bool_field(value, &["exports_automation_bodies"]))
        .unwrap_or(false);
    let exports_tool_payloads = redaction
        .and_then(|value| bool_field(value, &["exports_tool_payloads"]))
        .unwrap_or(false);
    let exports_task_payloads = redaction
        .and_then(|value| bool_field(value, &["exports_task_payloads"]))
        .unwrap_or(false);
    let exports_transcripts = redaction
        .and_then(|value| bool_field(value, &["exports_transcripts"]))
        .unwrap_or(false);
    let exports_provider_credentials = redaction
        .and_then(|value| bool_field(value, &["exports_provider_credentials"]))
        .unwrap_or(false);
    let redaction_requires_review = exports_secret_values
        || exports_account_targets
        || exports_automation_bodies
        || exports_tool_payloads
        || exports_task_payloads
        || exports_transcripts
        || exports_provider_credentials;
    let redaction_summary = if redaction_requires_review {
        "review required".to_string()
    } else if redaction.is_some() {
        "metadata only".to_string()
    } else {
        "unknown".to_string()
    };

    DxAgentContractSummary {
        present: value.is_some(),
        status,
        command_count: value
            .and_then(|value| value.get("commands"))
            .and_then(|value| value.as_object())
            .map(|commands| commands.len())
            .unwrap_or_default(),
        receipt_count: value
            .and_then(|value| array_field(value, &["receipts"]))
            .map(|receipts| receipts.len())
            .unwrap_or_default(),
        provider_catalog_source: provider_catalog
            .and_then(|value| receipt_string_field(value, &["source_format"]))
            .unwrap_or_else(|| "unknown".to_string()),
        provider_catalog_receipt_count: provider_catalog
            .and_then(|value| array_field(value, &["json_receipts"]))
            .map(|receipts| receipts.len())
            .unwrap_or_default(),
        safe_regeneration_command: provider_catalog
            .and_then(|value| receipt_string_field(value, &["safe_regeneration_command"]))
            .unwrap_or_else(|| "dx agents providers catalog regenerate --json".to_string()),
        redaction_summary,
        redaction_requires_review,
        next_action: value
            .and_then(|value| receipt_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents contract --json".to_string()),
        commands: value
            .map(|value| receipt_string_values_field(value, &["commands"]))
            .unwrap_or_default(),
        receipt_notes: receipt_notes(value),
    }
}

fn receipt_notes(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|value| array_field(value, &["receipts"]))
        .map(|receipts| {
            receipts
                .iter()
                .filter_map(|receipt| {
                    let name = receipt_string_field(receipt, &["name"])?;
                    let command = receipt_string_field(receipt, &["command"]).unwrap_or_default();
                    if command.is_empty() {
                        Some(name)
                    } else {
                        Some(format!("{name}: {command}"))
                    }
                })
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn import_summary(value: Option<&Value>, root_exists: bool) -> DxAgentImportSummary {
    let release_gate = value.and_then(|value| value.get("release_gate"));
    let action_map = value.and_then(|value| value.get("action_map"));
    let recovery_controls = value.and_then(|value| value.get("recovery_controls"));
    let recovery_counts = recovery_control_counts(recovery_controls, action_map);
    let freshness_policy = value.and_then(|value| value.get("freshness_policy"));
    let status = value
        .and_then(|value| receipt_string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_import_summary".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let next_action = release_gate
        .and_then(|value| receipt_string_field(value, &["next_action"]))
        .or_else(|| action_map.and_then(|value| receipt_string_field(value, &["next_action"])))
        .or_else(|| {
            recovery_controls.and_then(|value| receipt_string_field(value, &["next_action"]))
        })
        .or_else(|| value.and_then(|value| receipt_string_field(value, &["next_action"])))
        .unwrap_or_else(|| "dx agents import-summary --json".to_string());

    DxAgentImportSummary {
        present: value.is_some(),
        status,
        operator_summary: value
            .and_then(|value| receipt_string_field(value, &["operator_summary"]))
            .unwrap_or_default(),
        release_gate_status: release_gate
            .and_then(|value| receipt_string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        release_gate_warning_count: release_gate
            .and_then(|value| usize_field(value, &["warning_count"]))
            .unwrap_or_default(),
        release_gate_failed_count: release_gate
            .and_then(|value| usize_field(value, &["failed_count"]))
            .unwrap_or_default(),
        action_map_status: action_map
            .and_then(|value| receipt_string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        no_command_fanout: value
            .and_then(|value| bool_field(value, &["no_command_fanout"]))
            .or_else(|| action_map.and_then(|value| bool_field(value, &["no_command_fanout"])))
            .or_else(|| {
                recovery_controls.and_then(|value| bool_field(value, &["no_command_fanout"]))
            })
            .unwrap_or(false),
        recovery_controls_status: recovery_controls
            .and_then(|value| receipt_string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_render_first: recovery_controls
            .and_then(|value| receipt_string_field(value, &["render_first"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_counts,
        recovery_states: recovery_controls
            .map(|value| receipt_string_array_field(value, &["states"]))
            .unwrap_or_default(),
        recovery_fixture_count: recovery_controls
            .and_then(|value| usize_field(value, &["fixture_count"]))
            .unwrap_or_default(),
        freshness_state: freshness_policy
            .and_then(|value| receipt_string_field(value, &["latest_freshness_state"]))
            .unwrap_or_else(|| "unknown".to_string()),
        next_action,
        warning_reasons: release_gate
            .map(|value| receipt_string_array_field(value, &["warning_reasons"]))
            .unwrap_or_default(),
        blocking_reasons: release_gate
            .map(|value| receipt_string_array_field(value, &["blocking_reasons"]))
            .unwrap_or_default(),
        recovery_commands: value
            .map(|value| receipt_string_values_field(value, &["recovery_commands"]))
            .unwrap_or_default(),
    }
}

pub(super) fn release_gate(value: Option<&Value>, root_exists: bool) -> DxAgentReleaseGateSummary {
    let recovery_controls = value.and_then(|value| value.get("recovery_controls"));
    let recovery_counts = recovery_control_counts(recovery_controls, None);
    let status = value
        .and_then(|value| receipt_string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_release_gate".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let next_action = value
        .and_then(|value| receipt_string_field(value, &["next_action"]))
        .or_else(|| {
            recovery_controls.and_then(|value| receipt_string_field(value, &["next_action"]))
        })
        .unwrap_or_else(|| "dx agents release-gate --json".to_string());

    DxAgentReleaseGateSummary {
        present: value.is_some(),
        status,
        operator_summary: value
            .and_then(|value| receipt_string_field(value, &["operator_summary"]))
            .unwrap_or_default(),
        acceptance_count: value
            .and_then(|value| usize_field(value, &["acceptance_count"]))
            .unwrap_or_default(),
        passed_count: value
            .and_then(|value| usize_field(value, &["passed_count"]))
            .unwrap_or_default(),
        warning_count: value
            .and_then(|value| usize_field(value, &["warning_count"]))
            .unwrap_or_default(),
        failed_count: value
            .and_then(|value| usize_field(value, &["failed_count"]))
            .unwrap_or_default(),
        packet_count: value
            .and_then(|value| usize_field(value, &["packet_count"]))
            .unwrap_or_default(),
        fixture_family_count: value
            .and_then(|value| usize_field(value, &["fixture_family_count"]))
            .unwrap_or_default(),
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        retained_run_overflow_count: value
            .and_then(|value| usize_field(value, &["retained_run_overflow_count"]))
            .unwrap_or_default(),
        import_manifest_status: value
            .and_then(|value| receipt_string_field(value, &["import_manifest_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        smoke_status: value
            .and_then(|value| receipt_string_field(value, &["smoke_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        receipt_inbox_status: value
            .and_then(|value| receipt_string_field(value, &["receipt_inbox_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        retention_preview_status: value
            .and_then(|value| receipt_string_field(value, &["retention_preview_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        action_map_status: value
            .and_then(|value| receipt_string_field(value, &["action_map_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        no_command_fanout: value
            .and_then(|value| bool_field(value, &["no_command_fanout"]))
            .or_else(|| {
                recovery_controls.and_then(|value| bool_field(value, &["no_command_fanout"]))
            })
            .unwrap_or(false),
        recovery_controls_status: recovery_controls
            .and_then(|value| receipt_string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_render_first: recovery_controls
            .and_then(|value| receipt_string_field(value, &["render_first"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_counts,
        recovery_fixture_count: recovery_controls
            .and_then(|value| usize_field(value, &["fixture_count"]))
            .unwrap_or_default(),
        next_action,
        warning_reasons: value
            .map(|value| receipt_string_array_field(value, &["warning_reasons"]))
            .unwrap_or_default(),
        blocking_reasons: value
            .map(|value| receipt_string_array_field(value, &["blocking_reasons"]))
            .unwrap_or_default(),
        acceptance_rows: release_gate_acceptance_rows(value),
    }
}

fn recovery_control_counts(
    recovery_controls: Option<&Value>,
    fallback_action_map: Option<&Value>,
) -> DxAgentRecoveryControlCounts {
    DxAgentRecoveryControlCounts {
        required_intent_count: recovery_controls
            .and_then(|value| usize_field(value, &["required_intent_count"]))
            .or_else(|| {
                fallback_action_map.and_then(|value| usize_field(value, &["required_intent_count"]))
            })
            .unwrap_or_default(),
        action_count: recovery_controls
            .and_then(|value| usize_field(value, &["action_count"]))
            .or_else(|| fallback_action_map.and_then(|value| usize_field(value, &["action_count"])))
            .unwrap_or_default(),
        check_count: recovery_controls
            .and_then(|value| usize_field(value, &["check_count"]))
            .or_else(|| fallback_action_map.and_then(|value| usize_field(value, &["check_count"])))
            .unwrap_or_default(),
    }
}

pub(super) fn release_gate_acceptance_rows(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|value| array_field(value, &["acceptance"]))
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let label = receipt_string_field(row, &["label"])?;
                    let status =
                        receipt_string_field(row, &["status"]).unwrap_or_else(|| "unknown".into());
                    Some(format!("{label}: {status}"))
                })
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn receipt_inbox(
    value: Option<&Value>,
    root_exists: bool,
) -> DxAgentReceiptInboxSummary {
    let receipt_dir_present = value.and_then(|value| bool_field(value, &["receipt_dir_present"]));
    let status = value
        .and_then(|value| receipt_string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if receipt_dir_present == Some(false) || !root_exists {
                "missing_config".to_string()
            } else {
                "waiting_for_receipt_inbox".to_string()
            }
        });
    let status = if receipt_dir_present == Some(false) {
        "missing_config".to_string()
    } else {
        status
    };

    DxAgentReceiptInboxSummary {
        present: value.is_some(),
        status,
        receipt_dir_present,
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        latest_count: value
            .and_then(|value| usize_field(value, &["latest_count"]))
            .unwrap_or_default(),
        malformed_count: value
            .and_then(|value| usize_field(value, &["malformed_count"]))
            .unwrap_or_default(),
        missing_latest_count: value
            .and_then(|value| usize_field(value, &["missing_latest_count"]))
            .unwrap_or_default(),
        stale_count: value
            .and_then(|value| usize_field(value, &["stale_count"]))
            .unwrap_or_default(),
        expired_count: value
            .and_then(|value| usize_field(value, &["expired_count"]))
            .unwrap_or_default(),
        last_error: value.and_then(|value| receipt_string_field(value, &["last_error"])),
        next_action: value
            .and_then(|value| receipt_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents receipts --json".to_string()),
    }
}

pub(super) fn action_error(value: Option<&Value>) -> DxAgentActionErrorSummary {
    let redaction = value.and_then(|value| value.get("redaction"));
    let exports_secret_values = redaction
        .and_then(|value| bool_field(value, &["exports_secret_values"]))
        .unwrap_or(value.is_some());
    let exports_provider_credentials = redaction
        .and_then(|value| bool_field(value, &["exports_provider_credentials"]))
        .unwrap_or(value.is_some());
    let exports_receipt_bodies = redaction
        .and_then(|value| bool_field(value, &["exports_receipt_bodies"]))
        .unwrap_or(value.is_some());
    let redaction_requires_review =
        exports_secret_values || exports_provider_credentials || exports_receipt_bodies;
    let redaction_summary = if value.is_none() {
        "No failed DX Agents action".to_string()
    } else if redaction_requires_review {
        "Action-error receipt redaction requires review".to_string()
    } else {
        "Action-error receipt is redacted metadata only".to_string()
    };

    DxAgentActionErrorSummary {
        present: value.is_some(),
        status: value
            .and_then(|value| receipt_string_field(value, &["status"]))
            .unwrap_or_else(|| "ready".to_string()),
        command: value
            .and_then(|value| receipt_string_field(value, &["command"]))
            .unwrap_or_default(),
        error: value.and_then(|value| receipt_string_field(value, &["error"])),
        generated_at: value
            .and_then(|value| receipt_string_field(value, &["generated_at"]))
            .unwrap_or_default(),
        next_action: value
            .and_then(|value| receipt_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents status --json".to_string()),
        redaction_summary,
        redaction_requires_review,
    }
}

pub(super) fn receipt_index_summary(
    value: Option<&Value>,
    root_exists: bool,
) -> DxAgentReceiptIndexSummary {
    let receipt_root_present = value.and_then(|value| bool_field(value, &["receipt_root_present"]));
    let status = value
        .and_then(|value| receipt_string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if receipt_root_present == Some(false) {
                "missing_config".to_string()
            } else if root_exists {
                "waiting_for_receipts_list".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let status = if receipt_root_present == Some(false) {
        "missing_config".to_string()
    } else {
        status
    };

    DxAgentReceiptIndexSummary {
        present: value.is_some(),
        status,
        receipt_root_present,
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        returned_receipt_count: value
            .and_then(|value| usize_field(value, &["returned_receipt_count"]))
            .unwrap_or_default(),
        active_task_count: value
            .and_then(|value| usize_field(value, &["active_task_count"]))
            .unwrap_or_default(),
        latest_receipt_path: value.and_then(|value| {
            receipt_string_field(value, &["latest_receipt_path"])
                .filter(|path| !path.trim().is_empty())
        }),
        last_error: value.and_then(|value| receipt_string_field(value, &["last_error"])),
        next_action: value
            .and_then(|value| receipt_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents receipts list --json".to_string()),
    }
}

pub(super) fn receipts(value: &Value) -> Vec<DxAgentReceipt> {
    array_field(value, &["receipts"])
        .map(|receipts| receipts.iter().take(12).filter_map(receipt_row).collect())
        .unwrap_or_default()
}

fn receipt_row(value: &Value) -> Option<DxAgentReceipt> {
    let safe_to_render = bool_field(value, &["safe_to_render"]).unwrap_or(false);
    let metadata_redacted = bool_field(value, &["metadata_redacted"]).unwrap_or(false);
    let command = receipt_string_field(value, &["command"]).unwrap_or_default();
    let task_id = receipt_string_field(value, &["task_id"]).unwrap_or_default();
    let last_error = receipt_string_field(value, &["last_error"]);
    let next_action = receipt_string_field(value, &["next_action"]).unwrap_or_default();

    Some(DxAgentReceipt {
        id: receipt_string_field(value, &["id"])?,
        kind: receipt_string_field(value, &["kind"]).unwrap_or_else(|| "receipt".to_string()),
        schema_version: receipt_string_field(value, &["schema_version"]).unwrap_or_default(),
        command: if safe_to_render {
            command
        } else {
            String::new()
        },
        generated_at: receipt_string_field(value, &["generated_at"]).unwrap_or_default(),
        task_id: if safe_to_render {
            task_id
        } else {
            String::new()
        },
        task_state: if safe_to_render {
            receipt_string_field(value, &["task_state"]).unwrap_or_default()
        } else {
            String::new()
        },
        status: receipt_string_field(value, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        active_task: bool_field(value, &["active_task"]).unwrap_or(false),
        safe_to_render,
        metadata_redacted,
        receipt_path: receipt_string_field(value, &["receipt_path"]).unwrap_or_default(),
        size_bytes: usize_field(value, &["size_bytes"]).unwrap_or_default(),
        modified_at: receipt_string_field(value, &["modified_at"]).unwrap_or_default(),
        last_error,
        next_action,
        provider_status: receipt_string_field(value, &["provider_status"]),
        model_status: receipt_string_field(value, &["model_status"]),
        duration_state: receipt_string_field(value, &["duration_state"]),
        retry_supported: bool_field(value, &["retry_supported"]),
        cancel_supported: bool_field(value, &["cancel_supported"]),
        social_connected: usize_field(value, &["social_connected"]),
        social_needs_auth: usize_field(value, &["social_needs_auth"]),
        automation_enabled: usize_field(value, &["automation_enabled"]),
        automation_warning: usize_field(value, &["automation_warning"]),
    })
}
