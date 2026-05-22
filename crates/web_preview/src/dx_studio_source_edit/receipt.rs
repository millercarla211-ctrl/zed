use serde_json::Value;

use super::values::string_at;

pub(crate) fn style_edit_receipt_context(
    payload: &Value,
    selection: Option<&Value>,
    outcome: &str,
) -> Option<Value> {
    let operation = string_at(payload, &["/operation", "/edit/operation"]);
    let style_edit_prefill = payload.pointer("/edit/style_edit_prefill").cloned();
    let style_edit_plan = payload
        .pointer("/edit/style_edit_plan")
        .cloned()
        .or_else(|| selection.and_then(|selection| selection.get("style_edit_plan").cloned()));
    let computed_summary = string_at(payload, &["/edit/computed_summary"]).or_else(|| {
        style_edit_prefill
            .as_ref()
            .and_then(|prefill| string_at(prefill, &["/computed_summary"]))
    });

    if operation.as_deref() != Some("update_design_token")
        && style_edit_prefill.is_none()
        && style_edit_plan.is_none()
        && computed_summary.is_none()
    {
        return None;
    }

    let prefill_status = style_edit_prefill
        .as_ref()
        .and_then(|prefill| string_at(prefill, &["/status"]));
    let plan_status = style_edit_plan
        .as_ref()
        .and_then(|plan| string_at(plan, &["/status"]))
        .or_else(|| prefill_status.clone());
    let declared_style_contract_used = plan_status.as_deref() == Some("token_contract_ready");
    let token_candidates = style_edit_prefill
        .as_ref()
        .and_then(|prefill| prefill.get("token_candidates"))
        .cloned();

    Some(serde_json::json!({
        "schema": "zed.web_preview.dx_studio_style_edit_receipt_context.v1",
        "outcome": outcome,
        "operation": operation,
        "plan_status": plan_status,
        "declared_style_contract_used": declared_style_contract_used,
        "computed_summary": computed_summary,
        "token_candidates": token_candidates,
        "style_edit_prefill": style_edit_prefill,
        "style_edit_plan": style_edit_plan,
        "policy": {
            "rust_source_edit_must_verify_contract": true,
            "no_inline_style_write": true,
        },
    }))
}

pub(crate) fn refusal_status_detail(error: &anyhow::Error) -> &'static str {
    let error = error.to_string();
    if error.contains("source_snippet/insert_template") {
        "requires_manifest_source_template"
    } else if error.contains("trusted Zed source snapshot") {
        "missing_trusted_source_snapshot"
    } else if error.contains("source snapshot does not match") {
        "stale_source"
    } else if error.contains("stale text edit") || error.contains("stale source file") {
        "stale_source"
    } else if error.contains("ambiguous") {
        "ambiguous_source"
    } else if error.contains("generated/runtime") {
        "generated_runtime_refused"
    } else if error.contains("readonly") {
        "readonly_source"
    } else if error.contains("outside the workspace") {
        "outside_workspace_refused"
    } else if error.contains("not a detected DX-WWW project") {
        "non_dx_project"
    } else if error.contains("does not declare") {
        "operation_not_declared"
    } else if error.contains("breakpoint prefix") {
        "responsive_breakpoint_mismatch"
    } else if error.contains("requires_declared_text_marker") {
        "requires_declared_text_marker"
    } else if error.contains("requires_token_or_class_marker") {
        "requires_token_or_class_marker"
    } else if error.contains("requires_reorder_group_marker") {
        "requires_reorder_group_marker"
    } else if error.contains("requires_insert_slot_marker") {
        "requires_insert_slot_marker"
    } else if error.contains("requires_media_slot_marker") {
        "requires_media_slot_marker"
    } else if error.contains("requires_manifest_source_template") {
        "requires_manifest_source_template"
    } else {
        "source_edit_refused"
    }
}

pub(super) fn source_policy_for_receipt(mut source_policy: Value) -> Value {
    if let Some(policy) = source_policy.as_object_mut() {
        policy.insert(
            "trusted_source_snapshot_required".to_string(),
            Value::Bool(true),
        );
        policy.insert(
            "stale_source_snapshot_refused".to_string(),
            Value::Bool(true),
        );
        policy.insert(
            "rollback_attempted_on_write_error".to_string(),
            Value::Bool(true),
        );
    }
    source_policy
}
