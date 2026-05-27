use std::{fs, path::Path};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;

use crate::dx_studio;

mod manifest;
pub(crate) mod manifest_ts;
mod operations;
mod paths;
mod plan;
mod receipt;
mod snapshot;
mod source_ranges;
mod values;

pub(crate) use self::{
    plan::source_edit_plan,
    receipt::{refusal_status_detail, style_edit_receipt_context},
};

#[cfg(test)]
use self::snapshot::source_file_snapshot;
use self::{
    manifest::selection_with_manifest_contract,
    operations::{
        apply_source_operation, operation_declared, operation_missing_status,
        operation_ready_for_selection, source_operation_has_transformer,
    },
    paths::{ensure_source_policy_allows_edit, resolve_selection_source, source_policy_for_edit},
    receipt::source_policy_for_receipt,
    snapshot::{
        system_time_ms, validate_expected_source_contents, validate_expected_source_snapshot,
    },
    values::{compact_json, string_at},
};

const DX_STUDIO_MAX_SOURCE_FILE_BYTES: u64 = 2_000_000;
const DX_STUDIO_MAX_SOURCE_EDIT_DELTA_BYTES: i64 = 200_000;

pub(crate) const DX_STUDIO_SOURCE_EDIT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.dx_studio_source_edit_receipt.v1";
pub(crate) const DX_STUDIO_SOURCE_EDIT_PLAN_SCHEMA: &str =
    "zed.web_preview.dx_studio_source_edit_plan.v1";

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
    ensure_source_file_size_allows_edit(&source, metadata.len())?;
    if metadata.permissions().readonly() {
        bail!(
            "DX Studio refused to edit readonly source file {}",
            source.display()
        );
    }

    let original = fs::read_to_string(&source)
        .with_context(|| format!("Read source file {}", source.display()))?;
    validate_expected_source_contents(&source, &original, payload)?;
    let edit = apply_source_operation(&original, selection, payload, &operation)?;
    ensure_source_write_bounds(&source, edit.updated.len(), edit.changed_bytes)?;

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

fn ensure_source_file_size_allows_edit(source: &Path, source_len: u64) -> Result<()> {
    if source_len > DX_STUDIO_MAX_SOURCE_FILE_BYTES {
        bail!(
            "DX Studio refused to edit oversized source file {}: {source_len} bytes exceeds {DX_STUDIO_MAX_SOURCE_FILE_BYTES}",
            source.display()
        );
    }
    Ok(())
}

fn ensure_source_write_bounds(source: &Path, updated_len: usize, changed_bytes: i64) -> Result<()> {
    let updated_len = u64::try_from(updated_len).unwrap_or(u64::MAX);
    if updated_len > DX_STUDIO_MAX_SOURCE_FILE_BYTES {
        bail!(
            "DX Studio refused source edit because updated file {} would exceed {DX_STUDIO_MAX_SOURCE_FILE_BYTES} bytes",
            source.display()
        );
    }
    if changed_bytes.unsigned_abs() > DX_STUDIO_MAX_SOURCE_EDIT_DELTA_BYTES as u64 {
        bail!(
            "DX Studio refused source edit because write delta for {} exceeds {DX_STUDIO_MAX_SOURCE_EDIT_DELTA_BYTES} bytes",
            source.display()
        );
    }
    Ok(())
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
        let snapshot =
            source_file_snapshot(&fixture.source_file, &token_selection("launch-hero-panel"))
                .expect("fixture snapshot");
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
        let mut snapshot =
            source_file_snapshot(&fixture.source_file, &token_selection("launch-hero-panel"))
                .expect("fixture snapshot");
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

    #[test]
    fn apply_source_edit_refuses_snapshot_selection_identity_mismatch() {
        let fixture = dx_studio_apply_source_edit_fixture_root("token_identity_mismatch");
        write_token_fixture_source(&fixture.source_file);
        let original = fs::read_to_string(&fixture.source_file).expect("original source");
        let snapshot = source_file_snapshot(&fixture.source_file, &token_selection("launch.hero"))
            .expect("fixture snapshot");
        let payload = token_edit_payload(snapshot, "launch-hero-panel", "launch-hero-soft");

        let error = apply_source_edit(Some(&fixture.root), &payload).expect_err("identity refusal");
        let after_refusal = fs::read_to_string(&fixture.source_file).expect("source after refusal");

        assert_eq!(refusal_status_detail(&error), "stale_source");
        assert_eq!(after_refusal, original);
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

    fn token_selection(old_token: &str) -> Value {
        json!({
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
        })
    }

    fn token_edit_payload(snapshot: Value, old_token: &str, new_token: &str) -> Value {
        json!({
            "operation": "update_design_token",
            "selection": token_selection(old_token),
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
