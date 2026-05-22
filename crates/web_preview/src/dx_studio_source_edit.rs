use std::{
    fs::{self, Metadata},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;

use crate::dx_studio;

mod manifest;
mod manifest_ts;
mod operations;
mod paths;
mod source_ranges;
mod values;

use self::{
    manifest::selection_with_manifest_contract,
    operations::{
        apply_source_operation, operation_declared, operation_missing_status,
        operation_ready_for_selection, source_operation_has_transformer,
    },
    paths::{
        ensure_source_policy_allows_edit, resolve_selection_source, resolved_source_from_selection,
        source_policy_for_edit,
    },
    values::{compact_json, string_at},
};

pub(crate) const DX_STUDIO_SOURCE_EDIT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.dx_studio_source_edit_receipt.v1";
pub(crate) const DX_STUDIO_SOURCE_EDIT_PLAN_SCHEMA: &str =
    "zed.web_preview.dx_studio_source_edit_plan.v1";

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

pub(crate) fn source_edit_plan(root_path: Option<&Path>, selection: &Value) -> Value {
    let selection = selection_with_manifest_contract(root_path, selection);
    let selection = &selection;
    let operation_support = dx_studio::edit_operation_ids()
        .into_iter()
        .map(|operation| {
            let declared = operation_declared(selection, operation);
            let implemented = source_operation_has_transformer(operation);
            let ready = operation_ready_for_selection(selection, operation);
            let requires_source_template =
                matches!(operation, "insert_component" | "insert_icon_media");
            let has_source_template = source_template_available(selection, operation);
            let required_marker = match operation {
                "update_text_content" => "data-dx-editable-text",
                "update_design_token" => "data-dx-design-token or class token",
                "move_reorder_section" => "data-dx-reorder-group",
                "insert_component" => "data-dx-insert-slot",
                "insert_icon_media" => "data-dx-media-slot",
                _ => "unknown",
            };
            serde_json::json!({
                "operation": operation,
                "declared": declared,
                "writes_files": declared,
                "implemented": implemented,
                "required_marker": required_marker,
                "requires_source_template": requires_source_template,
                "has_source_template": has_source_template,
                "status": if ready {
                    "ready"
                } else if implemented && declared {
                    operation_missing_status(selection, operation)
                } else if implemented {
                    "not_declared_for_selection"
                } else if declared && matches!(operation, "insert_component" | "insert_icon_media") {
                    "requires_manifest_source_template"
                } else if declared {
                    "planned_requires_source_transformer"
                } else {
                    "not_declared_for_selection"
                },
            })
        })
        .collect::<Vec<_>>();

    let resolved_source =
        root_path.and_then(|root_path| resolved_source_from_selection(root_path, selection));
    let source_file = resolved_source
        .as_ref()
        .map(|path| path.display().to_string());
    let source_snapshot = resolved_source
        .as_ref()
        .and_then(|source| source_file_snapshot(source));
    let source_policy = root_path
        .zip(resolved_source.as_ref())
        .map(|(root_path, source)| source_policy_for_edit(root_path, source, selection));

    serde_json::json!({
        "schema": DX_STUDIO_SOURCE_EDIT_PLAN_SCHEMA,
        "source_file": source_file,
        "source_snapshot": source_snapshot,
        "source_policy": source_policy,
        "text_marker": string_at(selection, &["/text_marker", "/attributes/data-dx-editable-text"]),
        "edit_id": string_at(selection, &["/edit_id", "/attributes/data-dx-edit-id"]),
        "edit_kind": string_at(selection, &["/edit_kind", "/attributes/data-dx-edit-kind"]),
        "design_token": string_at(selection, &["/design_token", "/attributes/data-dx-design-token"]),
        "token_scope": string_at(selection, &["/token_scope", "/attributes/data-dx-token-scope"]),
        "style_surface": string_at(selection, &["/style_surface", "/attributes/data-dx-style-surface"]),
        "responsive_class_tokens": selection.get("responsive_class_tokens").cloned(),
        "breakpoint": selection.get("breakpoint").cloned(),
        "style_metrics": selection.get("style_metrics").cloned(),
        "style_edit_plan": selection.get("style_edit_plan").cloned(),
        "reorder_group": string_at(selection, &["/reorder_group", "/attributes/data-dx-reorder-group"]),
        "insert_slot": string_at(selection, &["/insert_slot", "/attributes/data-dx-insert-slot"]),
        "media_slot": string_at(selection, &["/media_slot", "/attributes/data-dx-media-slot"]),
        "manifest_surface": selection.get("manifest_surface").cloned(),
        "manifest_ambiguity": selection.get("manifest_ambiguity").cloned(),
        "manifest_operation_contracts": selection.get("manifest_operation_contracts").cloned(),
        "edit_contract": selection.get("edit_contract").cloned(),
        "trusted_source_snapshot_required": true,
        "operations": operation_support,
    })
}

fn source_template_available(selection: &Value, operation: &str) -> bool {
    if string_at(selection, &["/insert_template", "/source_snippet"]).is_some() {
        return true;
    }

    selection
        .get("manifest_operation_contracts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|contract| {
            string_at(contract, &["/operation", "/id", "/name"]).as_deref() == Some(operation)
                && string_at(
                    contract,
                    &[
                        "/source_snippet",
                        "/sourceSnippet",
                        "/insert_template",
                        "/insertTemplate",
                    ],
                )
                .is_some()
        })
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

pub(crate) fn apply_source_edit(root_path: Option<&Path>, payload: &Value) -> Result<Value> {
    let root_path = root_path.ok_or_else(|| anyhow!("DX Studio edit needs a workspace root"))?;
    if dx_studio::detect_project(root_path).is_none() {
        bail!(
            "DX Studio source edits are inactive because this workspace is not a detected DX-WWW project"
        );
    }

    let selection = payload
        .get("selection")
        .ok_or_else(|| anyhow!("DX Studio edit payload is missing selection"))?;
    let enriched_selection = selection_with_manifest_contract(Some(root_path), selection);
    let selection = &enriched_selection;
    if let Some(ambiguity) = selection.get("manifest_ambiguity") {
        bail!(
            "DX Studio refused ambiguous manifest selector before source edit: {}",
            compact_json(ambiguity)
        );
    }
    let operation = string_at(payload, &["/operation", "/edit/operation"])
        .ok_or_else(|| anyhow!("DX Studio edit payload is missing operation"))?;

    if !source_operation_has_transformer(&operation) {
        let plan = source_edit_plan(Some(root_path), selection);
        bail!(
            "DX Studio operation `{operation}` needs a declared source template or transformer before Zed can write it safely: {}",
            compact_json(&plan)
        );
    }

    if !operation_declared(selection, &operation) {
        bail!("Selected DX surface does not declare {operation}");
    }
    if !operation_ready_for_selection(selection, &operation) {
        let missing_status = operation_missing_status(selection, &operation);
        bail!(
            "Selected DX surface declares {operation} but is not ready for source edit: {missing_status}"
        );
    }

    let source = resolve_selection_source(root_path, selection)?;
    ensure_source_policy_allows_edit(root_path, &source, selection)?;
    let source_policy = source_policy_for_edit(root_path, &source, selection);

    let metadata =
        fs::metadata(&source).with_context(|| format!("Read metadata for {}", source.display()))?;
    validate_expected_source_snapshot(&source, &metadata, payload)?;
    if metadata.permissions().readonly() {
        bail!(
            "DX Studio refused to edit readonly source file {}",
            source.display()
        );
    }

    let original = fs::read_to_string(&source)
        .with_context(|| format!("Read source file {}", source.display()))?;
    let edit = apply_source_operation(&original, selection, payload, &operation)?;

    if let Err(error) = fs::write(&source, edit.updated.as_bytes()) {
        let _ = fs::write(&source, original.as_bytes());
        return Err(error).with_context(|| format!("Write source file {}", source.display()));
    }

    let modified_ms = fs::metadata(&source)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_ms);

    Ok(serde_json::json!({
        "schema": DX_STUDIO_SOURCE_EDIT_RECEIPT_SCHEMA,
        "status": "source_updated",
        "operation": operation,
        "source_file": source.display().to_string(),
        "text_marker": string_at(selection, &["/text_marker", "/attributes/data-dx-editable-text"]),
        "edit_id": string_at(selection, &["/edit_id", "/attributes/data-dx-edit-id"]),
        "edit_kind": string_at(selection, &["/edit_kind", "/attributes/data-dx-edit-kind"]),
        "changed_bytes": edit.changed_bytes,
        "edit": edit.details,
        "style_edit_context": style_edit_receipt_context(payload, Some(selection), "source_updated"),
        "modified_ms": modified_ms,
        "hot_reload": {
            "target": string_at(selection, &[
                "/hot_reload_target",
                "/attributes/data-dx-hot-reload-target"
            ]),
            "before_version": payload.get("hot_reload_before_version").cloned(),
            "version_endpoint": string_at(selection, &["/hot_reload_version_endpoint"])
                .unwrap_or_else(|| dx_studio::DX_HOT_RELOAD_VERSION_ENDPOINT.to_string()),
            "selection_preservation_key": string_at(selection, &[
                "/edit_id",
                "/text_marker",
                "/attributes/data-dx-edit-id",
                "/attributes/data-dx-editable-text"
            ]),
        },
        "source_policy": source_policy_for_receipt(source_policy),
    }))
}

fn source_policy_for_receipt(mut source_policy: Value) -> Value {
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

fn source_file_snapshot(source: &Path) -> Option<Value> {
    let metadata = fs::metadata(source).ok()?;
    Some(serde_json::json!({
        "source_file": source.display().to_string(),
        "len": metadata.len(),
        "modified_ms": metadata.modified().ok().and_then(system_time_ms),
        "readonly": metadata.permissions().readonly(),
    }))
}

fn validate_expected_source_snapshot(
    source: &Path,
    metadata: &Metadata,
    payload: &Value,
) -> Result<()> {
    let Some(expected) = payload.get("expected_source_snapshot") else {
        bail!(
            "DX Studio refused source edit without a trusted Zed source snapshot for {}",
            source.display()
        );
    };

    let expected_source = string_at(expected, &["/source_file"]).ok_or_else(|| {
        anyhow!(
            "DX Studio refused source edit without a trusted Zed source snapshot file identity for {}",
            source.display()
        )
    })?;
    let expected_source = Path::new(&expected_source);
    let expected_source = fs::canonicalize(expected_source).with_context(|| {
        format!(
            "Resolve trusted DX Studio source snapshot path {}",
            expected_source.display()
        )
    })?;
    let actual_source = fs::canonicalize(source)
        .with_context(|| format!("Resolve DX Studio edit source {}", source.display()))?;
    if expected_source != actual_source {
        bail!(
            "DX Studio refused source edit because the trusted source snapshot does not match {}",
            source.display()
        );
    }

    if let Some(expected_len) = expected.get("len").and_then(Value::as_u64)
        && expected_len != metadata.len()
    {
        bail!(
            "DX Studio refused stale source file {}: file length changed after selection",
            source.display()
        );
    }

    let current_modified_ms = metadata.modified().ok().and_then(system_time_ms);
    let expected_modified_ms = expected.get("modified_ms").and_then(Value::as_u64);
    if expected_modified_ms.is_some()
        && current_modified_ms.is_some()
        && expected_modified_ms != current_modified_ms
    {
        bail!(
            "DX Studio refused stale source file {}: file modified after selection",
            source.display()
        );
    }

    Ok(())
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn style_edit_receipt_context_marks_declared_token_contract() {
        let payload = json!({
            "operation": "update_design_token",
            "edit": {
                "computed_summary": "size 320px x 120px; margin 0px; padding 16px",
                "style_edit_prefill": {
                    "schema": "zed.web_preview.dx_studio_style_token_prefill.v1",
                    "status": "token_contract_ready",
                    "computed_summary": "fallback summary",
                    "token_candidates": ["launch-hero-panel", "sm:grid-cols-2"]
                },
                "style_edit_plan": {
                    "schema": "zed.web_preview.dx_studio_style_edit_plan.v1",
                    "status": "token_contract_ready",
                    "operation": "update_design_token"
                }
            }
        });

        let context = style_edit_receipt_context(&payload, None, "source_updated")
            .expect("token edit payload should produce style receipt context");

        assert_eq!(
            context.pointer("/schema").and_then(Value::as_str),
            Some("zed.web_preview.dx_studio_style_edit_receipt_context.v1")
        );
        assert_eq!(
            context.pointer("/outcome").and_then(Value::as_str),
            Some("source_updated")
        );
        assert_eq!(
            context.pointer("/plan_status").and_then(Value::as_str),
            Some("token_contract_ready")
        );
        assert_eq!(
            context
                .pointer("/declared_style_contract_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            context.pointer("/computed_summary").and_then(Value::as_str),
            Some("size 320px x 120px; margin 0px; padding 16px")
        );
        assert_eq!(
            context
                .pointer("/token_candidates/0")
                .and_then(Value::as_str),
            Some("launch-hero-panel")
        );
        assert_eq!(
            context
                .pointer("/style_edit_prefill/status")
                .and_then(Value::as_str),
            Some("token_contract_ready")
        );
        assert_eq!(
            context
                .pointer("/style_edit_plan/status")
                .and_then(Value::as_str),
            Some("token_contract_ready")
        );
        assert_eq!(
            context
                .pointer("/policy/no_inline_style_write")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn style_edit_receipt_context_preserves_refusal_outcome() {
        let payload = json!({
            "operation": "update_design_token",
            "edit": {
                "style_edit_prefill": {
                    "schema": "zed.web_preview.dx_studio_style_token_prefill.v1",
                    "status": "missing_declared_style_contract",
                    "computed_summary": "size 240px x 80px; margin 0px"
                }
            }
        });
        let selection = json!({
            "style_edit_plan": {
                "schema": "zed.web_preview.dx_studio_style_edit_plan.v1",
                "status": "missing_declared_style_contract",
                "operation": "update_design_token"
            }
        });

        let context = style_edit_receipt_context(&payload, Some(&selection), "refused")
            .expect("refused token edit should keep style receipt context");

        assert_eq!(
            context.pointer("/outcome").and_then(Value::as_str),
            Some("refused")
        );
        assert_eq!(
            context.pointer("/plan_status").and_then(Value::as_str),
            Some("missing_declared_style_contract")
        );
        assert_eq!(
            context
                .pointer("/declared_style_contract_used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            context.pointer("/computed_summary").and_then(Value::as_str),
            Some("size 240px x 80px; margin 0px")
        );
        assert_eq!(
            context
                .pointer("/style_edit_plan/status")
                .and_then(Value::as_str),
            Some("missing_declared_style_contract")
        );
    }

    #[test]
    fn style_edit_receipt_context_ignores_unrelated_edits_without_style_metadata() {
        let payload = json!({
            "operation": "update_text_content",
            "edit": {
                "text": "Launch-ready copy"
            }
        });

        assert!(style_edit_receipt_context(&payload, None, "source_updated").is_none());
    }

    #[test]
    fn apply_source_edit_returns_style_receipt_context_for_declared_token_fixture() {
        let fixture = dx_studio_apply_source_edit_fixture_root("token_success");
        write_token_fixture_source(&fixture.source_file);
        let snapshot = source_file_snapshot(&fixture.source_file).expect("fixture snapshot");
        let payload = token_edit_payload(snapshot, "launch-hero-panel", "launch-hero-soft");

        let receipt =
            apply_source_edit(Some(&fixture.root), &payload).expect("source edit receipt");
        let updated = fs::read_to_string(&fixture.source_file).expect("updated source");

        assert_eq!(
            receipt.pointer("/schema").and_then(Value::as_str),
            Some(DX_STUDIO_SOURCE_EDIT_RECEIPT_SCHEMA)
        );
        assert_eq!(
            receipt.pointer("/status").and_then(Value::as_str),
            Some("source_updated")
        );
        assert_eq!(
            receipt.pointer("/operation").and_then(Value::as_str),
            Some("update_design_token")
        );
        assert_eq!(
            receipt.pointer("/edit/strategy").and_then(Value::as_str),
            Some("data-dx-design-token")
        );
        assert_eq!(
            receipt
                .pointer("/style_edit_context/outcome")
                .and_then(Value::as_str),
            Some("source_updated")
        );
        assert_eq!(
            receipt
                .pointer("/style_edit_context/declared_style_contract_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            receipt
                .pointer("/style_edit_context/policy/no_inline_style_write")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            receipt
                .pointer("/source_policy/source_kind")
                .and_then(Value::as_str),
            Some("source_owned")
        );
        assert_eq!(
            receipt
                .pointer("/hot_reload/target")
                .and_then(Value::as_str),
            Some("route:/launch")
        );
        assert_eq!(
            receipt
                .pointer("/hot_reload/before_version")
                .and_then(Value::as_i64),
            Some(11)
        );
        assert!(updated.contains(r#"data-dx-design-token="launch-hero-soft""#));
        assert!(!updated.contains(r#"data-dx-design-token="launch-hero-panel""#));
    }

    #[test]
    fn apply_source_edit_refuses_stale_token_fixture_without_mutating_source() {
        let fixture = dx_studio_apply_source_edit_fixture_root("token_stale");
        write_token_fixture_source(&fixture.source_file);
        let original = fs::read_to_string(&fixture.source_file).expect("original source");
        let mut snapshot = source_file_snapshot(&fixture.source_file).expect("fixture snapshot");
        let stale_len = snapshot.get("len").and_then(Value::as_u64).unwrap_or(0) + 1;
        snapshot
            .as_object_mut()
            .expect("snapshot object")
            .insert("len".to_string(), json!(stale_len));
        let payload = token_edit_payload(snapshot, "launch-hero-panel", "launch-hero-soft");

        let error = apply_source_edit(Some(&fixture.root), &payload).expect_err("stale refusal");
        let after_refusal = fs::read_to_string(&fixture.source_file).expect("source after refusal");
        let context = style_edit_receipt_context(&payload, payload.get("selection"), "refused")
            .expect("refused style context");

        assert_eq!(refusal_status_detail(&error), "stale_source");
        assert_eq!(after_refusal, original);
        assert_eq!(
            context.pointer("/outcome").and_then(Value::as_str),
            Some("refused")
        );
        assert_eq!(
            context.pointer("/plan_status").and_then(Value::as_str),
            Some("token_contract_ready")
        );
        assert_eq!(
            context
                .pointer("/declared_style_contract_used")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    struct DxStudioFixture {
        root: PathBuf,
        source_file: PathBuf,
    }

    impl Drop for DxStudioFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn dx_studio_apply_source_edit_fixture_root(name: &str) -> DxStudioFixture {
        let root = env::temp_dir().join(format!(
            "zed_dx_studio_apply_source_edit_{name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("app")).expect("fixture app dir");
        fs::write(root.join("dx"), "name = \"dx-studio-source-edit-test\"\n")
            .expect("fixture dx marker");
        let source_file = root.join("app").join("page.tsx");

        DxStudioFixture { root, source_file }
    }

    fn write_token_fixture_source(source_file: &Path) {
        fs::write(
            source_file,
            r#"export function Page() {
  return (
    <main data-dx-route="/launch">
      <section
        data-dx-edit-id="launch.hero"
        data-dx-edit-kind="hero"
        data-dx-edit-ops="update_design_token"
        data-dx-source-file="app/page.tsx"
        data-dx-design-token="launch-hero-panel"
        data-dx-style-surface="theme-token"
        data-dx-hot-reload-target="route:/launch"
      >
        Hero
      </section>
    </main>
  );
}
"#,
        )
        .expect("fixture source");
    }

    fn token_edit_payload(snapshot: Value, old_token: &str, new_token: &str) -> Value {
        json!({
            "operation": "update_design_token",
            "selection": {
                "source_file": "app/page.tsx",
                "edit_id": "launch.hero",
                "edit_kind": "hero",
                "design_token": old_token,
                "style_surface": "theme-token",
                "operations": ["update_design_token"],
                "hot_reload_target": "route:/launch",
                "attributes": {
                    "data-dx-source-file": "app/page.tsx",
                    "data-dx-edit-id": "launch.hero",
                    "data-dx-edit-kind": "hero",
                    "data-dx-edit-ops": "update_design_token",
                    "data-dx-design-token": old_token,
                    "data-dx-style-surface": "theme-token",
                    "data-dx-hot-reload-target": "route:/launch"
                },
                "style_edit_plan": {
                    "schema": "zed.web_preview.dx_studio_style_edit_plan.v1",
                    "status": "token_contract_ready",
                    "operation": "update_design_token"
                }
            },
            "edit": {
                "old_token": old_token,
                "new_token": new_token,
                "computed_summary": "size 320px x 120px; margin 0px; padding 16px",
                "style_edit_prefill": {
                    "schema": "zed.web_preview.dx_studio_style_token_prefill.v1",
                    "status": "token_contract_ready",
                    "computed_summary": "size 320px x 120px; margin 0px; padding 16px",
                    "token_candidates": [old_token, new_token]
                },
                "style_edit_plan": {
                    "schema": "zed.web_preview.dx_studio_style_edit_plan.v1",
                    "status": "token_contract_ready",
                    "operation": "update_design_token"
                }
            },
            "expected_source_snapshot": snapshot,
            "hot_reload_before_version": 11
        })
    }
}
