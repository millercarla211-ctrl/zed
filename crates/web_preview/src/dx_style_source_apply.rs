use serde_json::{Value, json};

const DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA: &str = "dx.style.grouped-class-source-apply-contract";
const DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA: &str =
    "dx.style.grouped-class-reverse-css-delta-contract";
const DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA: &str =
    "dx.style.css-declaration-dry-run-contract";
pub(crate) const DX_STYLE_SOURCE_APPLY_RECEIPT_SCHEMA: &str =
    "zed.web_preview.dx_style_source_apply_receipt.v1";

const DX_STYLE_APPLY_KIND: &str = "dx-style-source-apply";
pub(crate) const DX_STYLE_SOURCE_APPLY_SESSION_KIND: &str =
    "zed.web_preview.dx_style.source_apply_session";
pub(crate) const DX_STYLE_ACTIVE_EDITOR_SOURCE_REVALIDATION_SCHEMA: &str =
    "zed.web_preview.dx_style.active_editor_source_revalidation";
pub(crate) const MAX_DX_STYLE_SOURCE_APPLY_SESSION_TOKEN_BYTES: usize = 256;
const ACTIVE_STYLE_CONTEXT_SCHEMA: &str = "zed.dx_style.active_context.v1";
const MAX_SOURCE_PATH_BYTES: usize = 4096;
const MAX_CLASS_NAME_BYTES: usize = 4096;
const MAX_CSS_BYTES: usize = 32 * 1024;
const MAX_GENERATOR_ID_BYTES: usize = 128;
const MAX_SOURCE_SPAN_BYTES: u64 = 16 * 1024;
const MAX_SOURCE_DIGEST_BYTES: usize = 128;
const MAX_CONTEXT_KIND_BYTES: usize = 64;
const MAX_CSS_SOURCE_EDIT_SAFETY_BYTES: usize = 128;
const MAX_PREVIEW_KIND_BYTES: usize = 64;
const MAX_PREVIEW_ANATOMY_PART_BYTES: usize = 64;
const MAX_PREVIEW_ANATOMY_PARTS: usize = 8;
const MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS: usize = 8;
const MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES: usize = 160;
const CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES: usize = 4096;
const SOURCE_DIGEST_PREFIX: &str = "fnv1a64:";

pub(crate) fn active_source_digest(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{SOURCE_DIGEST_PREFIX}{hash:016x}")
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SourceSpan {
    start: u64,
    end: u64,
}

pub(crate) fn source_apply_review_receipt(payload: &Value) -> Value {
    let mut reasons = Vec::new();
    if payload.get("kind").and_then(Value::as_str) != Some(DX_STYLE_APPLY_KIND) {
        reasons.push("payload kind is not dx-style-source-apply".to_string());
    }

    let request = payload.get("request").unwrap_or(&Value::Null);
    let contract = request.get("contract").unwrap_or(&Value::Null);
    let reverse_css_delta_contract = request
        .get("reverse_css_delta_contract")
        .unwrap_or(&Value::Null);
    let css_declaration_dry_run_contract = request
        .get("css_declaration_dry_run_contract")
        .unwrap_or(&Value::Null);
    let css_declaration_dry_run_preview = request
        .get("css_declaration_dry_run_preview")
        .unwrap_or(&Value::Null);
    let css_declaration_dry_run_diagnostics = optional_bounded_string_array(
        request,
        "/css_declaration_dry_run_diagnostics",
        "CSS declaration dry-run diagnostics",
        MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS,
        MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES,
        &mut reasons,
    );
    let css_declaration_dry_run_preview_diagnostics = optional_bounded_string_array(
        request,
        "/css_declaration_dry_run_preview_diagnostics",
        "CSS declaration dry-run preview diagnostics",
        MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS,
        MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES,
        &mut reasons,
    );
    let reverse_css_delta_preview = request
        .get("reverse_css_delta_preview")
        .unwrap_or(&Value::Null);
    let context = request.get("context").unwrap_or(&Value::Null);
    let native_active_editor_source_revalidation = request
        .get("native_active_editor_source_revalidation")
        .unwrap_or(&Value::Null);
    let group_context = context.get("group_context").unwrap_or(&Value::Null);
    let apply_gate = context.get("apply_gate").unwrap_or(&Value::Null);
    let editor_write_bridge = apply_gate
        .get("editor_write_bridge")
        .unwrap_or(&Value::Null);

    let contract_schema = contract.get("__schema").and_then(Value::as_str);
    if contract_schema != Some(DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA) {
        reasons.push("unsupported or missing DX Style source-apply contract schema".to_string());
    }
    let contract_ipc_kind = contract.get("ipc_kind").and_then(Value::as_str);
    if contract_ipc_kind != Some(DX_STYLE_APPLY_KIND) {
        reasons.push("source-apply contract IPC kind does not match payload kind".to_string());
    }
    let contract_session_kind = contract
        .get("source_apply_session_kind")
        .and_then(Value::as_str);
    if contract_session_kind != Some(DX_STYLE_SOURCE_APPLY_SESSION_KIND) {
        reasons.push("source-apply contract is missing trusted session kind".to_string());
    }
    let contract_source_mutation_enabled = contract
        .get("source_mutation_enabled")
        .and_then(Value::as_bool);
    if contract_source_mutation_enabled != Some(true) {
        reasons.push("source-apply contract is review-only".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "trusted Web Preview source-apply session",
    ) {
        reasons.push("source-apply contract is missing trusted session guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "active context kind supported",
    ) {
        reasons.push("source-apply contract is missing active context kind guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "active source digest match",
    ) {
        reasons.push("source-apply contract is missing active source digest guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "session-bound source identity",
    ) {
        reasons.push(
            "source-apply contract is missing session-bound source identity guard".to_string(),
        );
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "native active editor source revalidation",
    ) {
        reasons.push(
            "source-apply contract is missing native active editor source revalidation guard"
                .to_string(),
        );
    }
    if !string_array_contains(contract, "/review_context_kinds", "class_token")
        || !string_array_contains(contract, "/review_context_kinds", "class_list")
        || !string_array_contains(contract, "/review_context_kinds", "css_declaration")
    {
        reasons.push("source-apply contract is missing review context kinds".to_string());
    }
    if !string_array_contains(
        contract,
        "/mutation_context_kinds_when_enabled",
        "class_token",
    ) {
        reasons.push("source-apply contract is missing mutation context kind".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "reverse CSS map receipt match",
    ) {
        reasons.push("source-apply contract is missing reverse CSS map guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "generated CSS declaration delta validation",
    ) {
        reasons.push("source-apply contract is missing declaration-delta guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "source-owned preview output metadata",
    ) {
        reasons.push("source-apply contract is missing preview output metadata guard".to_string());
    }
    if !string_array_contains(contract, "/review_receipt_fields", "preview_output") {
        reasons.push("source-apply contract is missing preview output receipt field".to_string());
    }
    if !string_array_contains(contract, "/review_receipt_fields", "source_apply_session") {
        reasons.push(
            "source-apply contract is missing source-apply session receipt field".to_string(),
        );
    }
    if !string_array_contains(
        contract,
        "/review_receipt_fields",
        "native_active_editor_source_revalidation",
    ) {
        reasons.push(
            "source-apply contract is missing native active editor source revalidation receipt field"
                .to_string(),
        );
    }
    for field in [
        "css_declaration_dry_run_contract",
        "css_declaration_dry_run_diagnostics",
        "css_declaration_dry_run_preview",
        "css_declaration_dry_run_preview_diagnostics",
    ] {
        if !string_array_contains(contract, "/review_receipt_fields", field) {
            reasons.push(format!(
                "source-apply contract is missing {field} receipt field"
            ));
        }
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "CSS declaration dry-run receipt for CSS contexts",
    ) {
        reasons.push("source-apply contract is missing CSS declaration dry-run guard".to_string());
    }
    if !string_array_contains(
        contract,
        "/required_editor_guards",
        "reverse CSS delta preview provenance match",
    ) {
        reasons.push("source-apply contract is missing reverse-delta provenance guard".to_string());
    }
    validate_contract_u64(
        contract,
        "max_source_path_bytes",
        MAX_SOURCE_PATH_BYTES as u64,
        &mut reasons,
    );

    let reverse_css_delta_schema = reverse_css_delta_contract
        .get("__schema")
        .and_then(Value::as_str);
    if reverse_css_delta_schema != Some(DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA) {
        reasons.push("missing DX Style reverse CSS delta contract schema".to_string());
    }
    if reverse_css_delta_contract
        .get("source_mutation_enabled")
        .and_then(Value::as_bool)
        != Some(false)
    {
        reasons.push("reverse CSS delta contract is not review-only".to_string());
    }
    if !string_array_contains(
        reverse_css_delta_contract,
        "/required_editor_guards",
        "generated CSS declaration delta validation",
    ) {
        reasons.push("reverse CSS delta contract is missing declaration-delta guard".to_string());
    }
    if !string_array_contains(
        reverse_css_delta_contract,
        "/required_editor_guards",
        "reverse CSS delta preview provenance match",
    ) {
        reasons.push("reverse CSS delta contract is missing provenance guard".to_string());
    }
    let reverse_css_delta_supported_property_count = reverse_css_delta_contract
        .get("supported_properties")
        .and_then(Value::as_array)
        .map_or(0, |properties| properties.len());
    if reverse_css_delta_supported_property_count == 0 {
        reasons.push("reverse CSS delta contract has no supported properties".to_string());
    }
    let reverse_css_delta_required_provenance_fields = string_array_at(
        reverse_css_delta_contract,
        "/required_preview_provenance_fields",
    );
    let reverse_css_delta_required_provenance_field_count =
        reverse_css_delta_required_provenance_fields.len();
    if reverse_css_delta_required_provenance_field_count == 0 {
        reasons.push(
            "reverse CSS delta contract has no required preview provenance fields".to_string(),
        );
    }
    if !string_array_contains(
        reverse_css_delta_contract,
        "/required_preview_provenance_fields",
        "group_alias",
    ) || !string_array_contains(
        reverse_css_delta_contract,
        "/required_preview_provenance_fields",
        "reverse_css_map_status",
    ) {
        reasons.push(
            "reverse CSS delta contract is missing required provenance identity fields".to_string(),
        );
    }
    let reverse_css_delta_provenance_reason_start = reasons.len();
    validate_reverse_delta_preview_provenance(
        reverse_css_delta_preview,
        group_context,
        &reverse_css_delta_required_provenance_fields,
        &mut reasons,
    );
    let reverse_css_delta_provenance_matches_context =
        reasons.len() == reverse_css_delta_provenance_reason_start;
    validate_contract_u64(
        contract,
        "max_class_name_bytes",
        MAX_CLASS_NAME_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_css_bytes",
        MAX_CSS_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_generator_id_bytes",
        MAX_GENERATOR_ID_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_source_span_bytes",
        MAX_SOURCE_SPAN_BYTES,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_source_digest_bytes",
        MAX_SOURCE_DIGEST_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_source_apply_session_token_bytes",
        MAX_DX_STYLE_SOURCE_APPLY_SESSION_TOKEN_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_preview_kind_bytes",
        MAX_PREVIEW_KIND_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_preview_anatomy_part_bytes",
        MAX_PREVIEW_ANATOMY_PART_BYTES as u64,
        &mut reasons,
    );
    validate_contract_u64(
        contract,
        "max_preview_anatomy_parts",
        MAX_PREVIEW_ANATOMY_PARTS as u64,
        &mut reasons,
    );

    let generator = bounded_string(
        request,
        "/generator",
        "generator",
        MAX_GENERATOR_ID_BYTES,
        &mut reasons,
    );
    let source_path = bounded_string(
        request,
        "/source_path",
        "request source_path",
        MAX_SOURCE_PATH_BYTES,
        &mut reasons,
    );
    let context_source_path = bounded_string(
        context,
        "/source_path",
        "context source_path",
        MAX_SOURCE_PATH_BYTES,
        &mut reasons,
    );
    if source_path.is_some() && context_source_path.is_some() && source_path != context_source_path
    {
        reasons.push("request source_path does not match context source_path".to_string());
    }

    let request_span = source_span_at(request, "/source_span", "request source_span", &mut reasons);
    let context_span = source_span_at(context, "/source_span", "context source_span", &mut reasons);
    if request_span.is_some() && context_span.is_some() && request_span != context_span {
        reasons.push("request source_span does not match context source_span".to_string());
    }
    let context_source_digest = bounded_string(
        context,
        "/source_digest",
        "context source_digest",
        MAX_SOURCE_DIGEST_BYTES,
        &mut reasons,
    );
    let request_source_digest = bounded_string(
        request,
        "/source_digest",
        "request source_digest",
        MAX_SOURCE_DIGEST_BYTES,
        &mut reasons,
    );
    if context_source_digest.is_some_and(|digest| !is_source_digest(digest)) {
        reasons.push("context source_digest is not a complete fnv1a64 digest".to_string());
    }
    if request_source_digest.is_some_and(|digest| !is_source_digest(digest)) {
        reasons.push("request source_digest is not a complete fnv1a64 digest".to_string());
    }
    if request_source_digest.is_some()
        && context_source_digest.is_some()
        && request_source_digest != context_source_digest
    {
        reasons.push("request source_digest does not match context source_digest".to_string());
    }
    let context_source_len = context.get("source_len_bytes").and_then(Value::as_u64);
    if let (Some(source_len), Some(span)) = (context_source_len, context_span) {
        if span.end > source_len {
            reasons.push("context source_span exceeds context source length".to_string());
        }
    }
    if let (Some(source_len), Some(span)) = (context_source_len, request_span) {
        if span.end > source_len {
            reasons.push("request source_span exceeds context source length".to_string());
        }
    }
    let native_revalidation_schema = native_active_editor_source_revalidation
        .get("schema")
        .and_then(Value::as_str);
    if native_revalidation_schema != Some(DX_STYLE_ACTIVE_EDITOR_SOURCE_REVALIDATION_SCHEMA) {
        reasons.push(
            "native active editor source revalidation schema is missing or invalid".to_string(),
        );
    }
    let native_revalidation_status = native_active_editor_source_revalidation
        .get("status")
        .and_then(Value::as_str);
    if native_revalidation_status != Some("matched") {
        reasons.push(
            "native active editor source revalidation did not match active source".to_string(),
        );
    }
    let native_revalidation_source_path = native_active_editor_source_revalidation
        .get("source_path")
        .and_then(Value::as_str);
    if native_revalidation_status == Some("matched")
        && native_revalidation_source_path != source_path
    {
        reasons.push(
            "native active editor source revalidation path does not match request source_path"
                .to_string(),
        );
    }
    let native_revalidation_source_digest = native_active_editor_source_revalidation
        .get("source_digest")
        .and_then(Value::as_str);
    if native_revalidation_status == Some("matched")
        && request_source_digest.is_some()
        && native_revalidation_source_digest != request_source_digest
    {
        reasons.push(
            "native active editor source revalidation digest does not match request source_digest"
                .to_string(),
        );
    }
    let native_revalidation_span = source_span_at(
        native_active_editor_source_revalidation,
        "/source_span",
        "native active editor source_span",
        &mut reasons,
    );
    if native_revalidation_status == Some("matched")
        && request_span.is_some()
        && native_revalidation_span != request_span
    {
        reasons.push(
            "native active editor source revalidation span does not match request source_span"
                .to_string(),
        );
    }
    let native_session_source = native_active_editor_source_revalidation
        .get("session_source")
        .unwrap_or(&Value::Null);
    if native_revalidation_status == Some("matched") && !native_session_source.is_object() {
        reasons.push(
            "native active editor source revalidation is missing session-bound source identity"
                .to_string(),
        );
    }
    let native_session_source_path = native_session_source
        .get("source_path")
        .and_then(Value::as_str);
    if native_revalidation_status == Some("matched") && native_session_source_path != source_path {
        reasons.push(
            "session-bound source identity path does not match request source_path".to_string(),
        );
    }
    let native_session_source_digest = native_session_source
        .get("source_digest")
        .and_then(Value::as_str);
    if native_revalidation_status == Some("matched")
        && request_source_digest.is_some()
        && native_session_source_digest != request_source_digest
    {
        reasons.push(
            "session-bound source identity digest does not match request source_digest".to_string(),
        );
    }
    let native_session_source_len = native_session_source
        .get("source_len_bytes")
        .and_then(Value::as_u64);
    if native_revalidation_status == Some("matched")
        && context_source_len.is_some()
        && native_session_source_len != context_source_len
    {
        reasons.push(
            "session-bound source identity length does not match context source_len_bytes"
                .to_string(),
        );
    }
    let native_session_source_span = source_span_at(
        native_session_source,
        "/source_span",
        "session-bound source identity source_span",
        &mut reasons,
    );
    if native_revalidation_status == Some("matched")
        && request_span.is_some()
        && native_session_source_span != request_span
    {
        reasons.push(
            "session-bound source identity span does not match request source_span".to_string(),
        );
    }

    let context_schema = context.get("schema").and_then(Value::as_str);
    if context_schema != Some(ACTIVE_STYLE_CONTEXT_SCHEMA) {
        reasons.push("unsupported or missing Zed Style context schema".to_string());
    }
    let context_kind = bounded_string(
        context,
        "/context_kind",
        "context kind",
        MAX_CONTEXT_KIND_BYTES,
        &mut reasons,
    );
    if context_kind
        .is_some_and(|kind| !string_array_contains(contract, "/review_context_kinds", kind))
    {
        reasons.push("context kind is not listed in the source-apply review contract".to_string());
    }
    let css_source_edit_safety = bounded_optional_string(
        context,
        "/css_source_edit_safety",
        "CSS source edit safety",
        MAX_CSS_SOURCE_EDIT_SAFETY_BYTES,
        &mut reasons,
    );
    if context_kind == Some("css_declaration") && css_source_edit_safety.is_none() {
        reasons.push("CSS declaration context is missing source edit safety".to_string());
    }
    let mut css_dry_run_proposed_declaration = css_declaration_dry_run_preview
        .get("proposed_declaration")
        .and_then(Value::as_str);
    if context_kind == Some("css_declaration") {
        if !css_declaration_dry_run_diagnostics.is_empty() {
            reasons.push("CSS declaration dry-run diagnostics are not empty".to_string());
        }
        if !css_declaration_dry_run_preview_diagnostics.is_empty() {
            reasons.push("CSS declaration dry-run preview diagnostics are not empty".to_string());
        }
        let css_dry_run_schema = css_declaration_dry_run_contract
            .get("__schema")
            .and_then(Value::as_str);
        if css_dry_run_schema != Some(DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA) {
            reasons.push("missing DX Style CSS declaration dry-run contract schema".to_string());
        }
        if css_declaration_dry_run_contract
            .get("source_mutation_enabled")
            .and_then(Value::as_bool)
            != Some(false)
        {
            reasons.push("CSS declaration dry-run contract is not review-only".to_string());
        }
        if css_declaration_dry_run_contract
            .get("review_context_kind")
            .and_then(Value::as_str)
            != Some("css_declaration")
        {
            reasons.push(
                "CSS declaration dry-run contract does not name css_declaration context"
                    .to_string(),
            );
        }
        if !string_array_contains(
            css_declaration_dry_run_contract,
            "/required_context_fields",
            "css_source_edit_safety",
        ) {
            reasons.push(
                "CSS declaration dry-run contract is missing source-edit safety context"
                    .to_string(),
            );
        }
        for field in [
            "css_declaration_dry_run_diagnostics",
            "css_declaration_dry_run_preview",
            "css_declaration_dry_run_preview_diagnostics",
        ] {
            if !string_array_contains(
                css_declaration_dry_run_contract,
                "/review_receipt_fields",
                field,
            ) {
                reasons.push(format!(
                    "CSS declaration dry-run contract is missing {field} receipt field"
                ));
            }
        }
        if let Some(css_source_edit_safety) = css_source_edit_safety
            && !string_array_contains(
                css_declaration_dry_run_contract,
                "/accepted_source_edit_safety",
                css_source_edit_safety,
            )
        {
            reasons
                .push("CSS declaration source edit safety is not accepted for dry-run".to_string());
        }
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_declaration_bytes",
            CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES as u64,
            &mut reasons,
        );
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_diagnostic_count",
            MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS as u64,
            &mut reasons,
        );
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_diagnostic_bytes",
            MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES as u64,
            &mut reasons,
        );
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_source_path_bytes",
            MAX_SOURCE_PATH_BYTES as u64,
            &mut reasons,
        );
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_source_span_bytes",
            MAX_SOURCE_SPAN_BYTES,
            &mut reasons,
        );
        validate_named_contract_u64(
            css_declaration_dry_run_contract,
            "CSS declaration dry-run contract",
            "max_source_digest_bytes",
            MAX_SOURCE_DIGEST_BYTES as u64,
            &mut reasons,
        );
        if css_declaration_dry_run_preview
            .get("status")
            .and_then(Value::as_str)
            != Some("ready_for_review")
        {
            reasons.push("CSS declaration dry-run preview is not ready for review".to_string());
        }
        css_dry_run_proposed_declaration = css_declaration_dry_run_preview
            .get("proposed_declaration")
            .and_then(Value::as_str);
        match css_dry_run_proposed_declaration {
            Some(value) if value.is_empty() => {
                reasons.push(
                    "CSS declaration dry-run preview is missing proposed declaration".to_string(),
                );
                css_dry_run_proposed_declaration = None;
            }
            Some(value) if value.len() > CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES => {
                reasons.push(format!(
                    "CSS declaration dry-run proposed declaration exceeds {CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES} bytes"
                ));
                css_dry_run_proposed_declaration = None;
            }
            Some(_) => {}
            None => {
                reasons.push(
                    "CSS declaration dry-run preview is missing proposed declaration".to_string(),
                );
            }
        }
    } else if !css_declaration_dry_run_diagnostics.is_empty()
        || !css_declaration_dry_run_preview_diagnostics.is_empty()
    {
        reasons.push(
            "CSS declaration dry-run diagnostics require a CSS declaration context".to_string(),
        );
    }

    let class_name = bounded_string(
        request,
        "/output/className",
        "output className",
        MAX_CLASS_NAME_BYTES,
        &mut reasons,
    );
    let css = bounded_optional_string(
        request,
        "/output/css",
        "output css",
        MAX_CSS_BYTES,
        &mut reasons,
    );
    let preview_kind = bounded_string(
        request,
        "/output/previewKind",
        "output preview kind",
        MAX_PREVIEW_KIND_BYTES,
        &mut reasons,
    );
    let preview_anatomy = bounded_string_array(
        request,
        "/output/previewAnatomy",
        "output preview anatomy",
        MAX_PREVIEW_ANATOMY_PARTS,
        MAX_PREVIEW_ANATOMY_PART_BYTES,
        &mut reasons,
    );
    let metadata_status = request.pointer("/metadata/status").and_then(Value::as_str);
    if metadata_status != Some("aligned") {
        reasons.push("DX Style visual-generator metadata is not aligned".to_string());
    }
    if apply_gate.get("can_enable_apply").and_then(Value::as_bool) != Some(true) {
        reasons.push("style apply gate is not ready".to_string());
    }
    if apply_gate.get("can_enable_apply").and_then(Value::as_bool) == Some(true) {
        if apply_gate
            .get("trusted_dry_run_receipt_present")
            .and_then(Value::as_bool)
            != Some(true)
        {
            reasons.push("style apply gate is ready without a trusted dry-run receipt".to_string());
        }
        if apply_gate.get("receipt_match").and_then(Value::as_str) != Some("active_source_matched")
        {
            reasons.push(
                "style apply gate is ready without an active-source receipt match".to_string(),
            );
        }
        if apply_gate
            .get("receipt_path")
            .and_then(Value::as_str)
            .is_none()
        {
            reasons.push("style apply gate is ready without a receipt path".to_string());
        }
    }
    if editor_write_bridge
        .get("can_apply")
        .and_then(Value::as_bool)
        != Some(true)
    {
        reasons.push("editor write bridge is not ready".to_string());
    }

    let web_preview_declared_review_capability =
        bool_at(payload, "/handler_capability/can_review_request").unwrap_or(false);
    let web_preview_declared_mutation_capability =
        bool_at(payload, "/handler_capability/can_mutate_source").unwrap_or(false);
    let can_review_request = true;
    let can_mutate_source = false;
    if !web_preview_declared_review_capability {
        reasons.push("Web Preview did not declare review request capability".to_string());
    }
    if web_preview_declared_mutation_capability {
        reasons.push("Web Preview cannot declare native mutation capability".to_string());
    }
    reasons.push("native source writer capability is review-only".to_string());

    let status = if can_review_request {
        "reviewed_with_blockers"
    } else {
        "refused"
    };
    json!({
        "schema": DX_STYLE_SOURCE_APPLY_RECEIPT_SCHEMA,
        "contract_schema": DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA,
        "ipc_kind": DX_STYLE_APPLY_KIND,
        "status": status,
        "review_status": if can_review_request { "reviewed" } else { "handler_review_capability_missing" },
        "mutation_ready": false,
        "source_mutation": "not_performed_by_review_receipt",
        "reason_count": reasons.len(),
        "reasons": reasons,
        "generator": generator,
        "source_path": source_path,
        "source_digest": request_source_digest,
        "source_span": span_json(request_span),
        "context": {
            "schema": context_schema,
            "status": context.get("status").and_then(Value::as_str),
            "context_kind": context_kind,
            "source_path": context_source_path,
            "source_span": span_json(context_span),
            "source_digest": context_source_digest,
            "source_len_bytes": context_source_len,
            "css_property": context.get("css_property").and_then(Value::as_str),
            "css_source_edit_safety": css_source_edit_safety,
        },
        "output": {
            "className": class_name,
            "css": css,
            "preview_kind": preview_kind,
            "preview_anatomy": preview_anatomy.clone(),
        },
        "preview_output": {
            "kind": preview_kind,
            "anatomy": preview_anatomy,
        },
        "metadata": {
            "status": metadata_status,
            "generator_count": request.pointer("/metadata/generatorCount").cloned(),
        },
        "contract": {
            "schema": contract_schema,
            "ipc_kind": contract_ipc_kind,
            "source_apply_session_kind": contract_session_kind,
            "source_mutation_enabled": contract_source_mutation_enabled,
            "source": contract.get("__source").and_then(Value::as_str),
            "review_context_kinds": string_array_at(contract, "/review_context_kinds"),
            "mutation_context_kinds_when_enabled": string_array_at(contract, "/mutation_context_kinds_when_enabled"),
            "max_source_path_bytes": contract.get("max_source_path_bytes").and_then(Value::as_u64),
            "max_class_name_bytes": contract.get("max_class_name_bytes").and_then(Value::as_u64),
            "max_css_bytes": contract.get("max_css_bytes").and_then(Value::as_u64),
            "max_generator_id_bytes": contract.get("max_generator_id_bytes").and_then(Value::as_u64),
            "max_source_span_bytes": contract.get("max_source_span_bytes").and_then(Value::as_u64),
            "max_source_digest_bytes": contract.get("max_source_digest_bytes").and_then(Value::as_u64),
            "max_source_apply_session_token_bytes": contract.get("max_source_apply_session_token_bytes").and_then(Value::as_u64),
            "max_preview_kind_bytes": contract.get("max_preview_kind_bytes").and_then(Value::as_u64),
            "max_preview_anatomy_part_bytes": contract.get("max_preview_anatomy_part_bytes").and_then(Value::as_u64),
            "max_preview_anatomy_parts": contract.get("max_preview_anatomy_parts").and_then(Value::as_u64),
        },
        "source_apply_session": {
            "kind": DX_STYLE_SOURCE_APPLY_SESSION_KIND,
            "trusted": true,
            "token_present": payload.pointer("/source_apply_session/token").and_then(Value::as_str).is_some(),
            "request_token_present": request.pointer("/source_apply_session/token").and_then(Value::as_str).is_some(),
        },
        "css_declaration_dry_run_contract": {
            "schema": css_declaration_dry_run_contract.get("__schema").and_then(Value::as_str),
            "source": css_declaration_dry_run_contract.get("__source").and_then(Value::as_str),
            "dry_run_receipt_schema": css_declaration_dry_run_contract.get("dry_run_receipt_schema").and_then(Value::as_str),
            "source_mutation_enabled": css_declaration_dry_run_contract.get("source_mutation_enabled").and_then(Value::as_bool),
            "review_context_kind": css_declaration_dry_run_contract.get("review_context_kind").and_then(Value::as_str),
            "max_declaration_bytes": css_declaration_dry_run_contract.get("max_declaration_bytes").and_then(Value::as_u64),
            "max_diagnostic_count": css_declaration_dry_run_contract.get("max_diagnostic_count").and_then(Value::as_u64),
            "max_diagnostic_bytes": css_declaration_dry_run_contract.get("max_diagnostic_bytes").and_then(Value::as_u64),
            "max_source_path_bytes": css_declaration_dry_run_contract.get("max_source_path_bytes").and_then(Value::as_u64),
            "max_source_span_bytes": css_declaration_dry_run_contract.get("max_source_span_bytes").and_then(Value::as_u64),
            "max_source_digest_bytes": css_declaration_dry_run_contract.get("max_source_digest_bytes").and_then(Value::as_u64),
            "accepted_source_edit_safety": string_array_at(css_declaration_dry_run_contract, "/accepted_source_edit_safety"),
            "review_receipt_fields": string_array_at(css_declaration_dry_run_contract, "/review_receipt_fields"),
            "required_context_field_count": css_declaration_dry_run_contract.get("required_context_fields").and_then(Value::as_array).map(|fields| fields.len()),
            "review_receipt_field_count": css_declaration_dry_run_contract.get("review_receipt_fields").and_then(Value::as_array).map(|fields| fields.len()),
        },
        "css_declaration_dry_run_diagnostics": css_declaration_dry_run_diagnostics,
        "css_declaration_dry_run_preview": {
            "status": css_declaration_dry_run_preview.get("status").and_then(Value::as_str),
            "property": css_declaration_dry_run_preview.get("property").and_then(Value::as_str),
            "value": css_declaration_dry_run_preview.get("value").and_then(Value::as_str),
            "proposed_declaration": css_dry_run_proposed_declaration,
            "source_edit_safety": css_declaration_dry_run_preview.get("source_edit_safety").and_then(Value::as_str),
        },
        "css_declaration_dry_run_preview_diagnostics": css_declaration_dry_run_preview_diagnostics,
        "reverse_css_delta_contract": {
            "schema": reverse_css_delta_schema,
            "source": reverse_css_delta_contract.get("__source").and_then(Value::as_str),
            "source_mutation_enabled": reverse_css_delta_contract.get("source_mutation_enabled").and_then(Value::as_bool),
            "supported_property_count": reverse_css_delta_supported_property_count,
            "required_guard_count": reverse_css_delta_contract.get("required_editor_guards").and_then(Value::as_array).map(|guards| guards.len()),
            "required_provenance_field_count": reverse_css_delta_required_provenance_field_count,
            "required_provenance_fields": reverse_css_delta_required_provenance_fields,
            "example_target_utility": reverse_css_delta_contract.pointer("/example_preview/target_utility").and_then(Value::as_str),
        },
        "reverse_css_delta_preview": {
            "status": reverse_css_delta_preview.get("status").and_then(Value::as_str),
            "provenance_matches_context": reverse_css_delta_provenance_matches_context,
            "group_status": reverse_css_delta_preview.get("group_status").and_then(Value::as_str),
            "group_alias": reverse_css_delta_preview.get("group_alias").and_then(Value::as_str),
            "group_syntax": reverse_css_delta_preview.get("group_syntax").and_then(Value::as_str),
            "group_expansion_status": reverse_css_delta_preview.get("group_expansion_status").and_then(Value::as_str),
            "group_registry_receipt": reverse_css_delta_preview.get("group_registry_receipt").and_then(Value::as_str),
            "reverse_css_map_receipt": reverse_css_delta_preview.get("reverse_css_map_receipt").and_then(Value::as_str),
            "reverse_css_map_status": reverse_css_delta_preview.get("reverse_css_map_status").and_then(Value::as_str),
            "group_source_state": reverse_css_delta_preview.get("group_source_state").and_then(Value::as_str),
            "group_utility_count": reverse_css_delta_preview.get("group_utility_count").and_then(Value::as_u64),
            "property": reverse_css_delta_preview.get("property").and_then(Value::as_str),
            "value": reverse_css_delta_preview.get("value").and_then(Value::as_str),
            "target_utility": reverse_css_delta_preview.get("target_utility").and_then(Value::as_str),
            "replacement_utility_count": reverse_css_delta_preview.get("replacement_utilities").and_then(Value::as_array).map(|utilities| utilities.len()),
            "replacement_source_declaration": reverse_css_delta_preview.get("replacement_source_declaration").and_then(Value::as_str),
        },
        "apply_gate": {
            "state": apply_gate.get("state").and_then(Value::as_str),
            "reason": apply_gate.get("reason").and_then(Value::as_str),
            "can_enable_apply": apply_gate.get("can_enable_apply").and_then(Value::as_bool),
            "trusted_dry_run_receipt_present": apply_gate.get("trusted_dry_run_receipt_present").and_then(Value::as_bool),
            "receipt_match": apply_gate.get("receipt_match").and_then(Value::as_str),
            "receipt_path": apply_gate.get("receipt_path").and_then(Value::as_str),
            "receipt_summary": apply_gate.get("receipt_summary").cloned(),
            "receipt_mismatch": apply_gate.get("receipt_mismatch").cloned(),
            "editor_write_bridge": {
                "state": editor_write_bridge.get("state").and_then(Value::as_str),
                "can_apply": editor_write_bridge.get("can_apply").and_then(Value::as_bool),
            }
        },
        "dry_run_review": {
            "trusted_receipt_present": apply_gate.get("trusted_dry_run_receipt_present").and_then(Value::as_bool),
            "receipt_match": apply_gate.get("receipt_match").and_then(Value::as_str),
            "receipt_path": apply_gate.get("receipt_path").and_then(Value::as_str),
            "receipt_summary": apply_gate.get("receipt_summary").cloned(),
            "receipt_mismatch": apply_gate.get("receipt_mismatch").cloned(),
        },
        "native_active_editor_source_revalidation": native_active_editor_source_revalidation,
        "native_handler": {
            "can_review_request": can_review_request,
            "can_mutate_source": can_mutate_source,
            "web_preview_declared_review_capability": web_preview_declared_review_capability,
            "web_preview_declared_mutation_capability": web_preview_declared_mutation_capability,
        },
    })
}

pub(crate) fn source_apply_session_refused_receipt(payload: &Value, reason: &str) -> Value {
    let request = payload.get("request").unwrap_or(&Value::Null);
    json!({
        "schema": DX_STYLE_SOURCE_APPLY_RECEIPT_SCHEMA,
        "contract_schema": DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA,
        "ipc_kind": DX_STYLE_APPLY_KIND,
        "status": "refused",
        "review_status": "source_apply_session_refused",
        "mutation_ready": false,
        "source_mutation": "not_performed_by_untrusted_session",
        "reason_count": 1,
        "reasons": [reason],
        "generator": request.get("generator").and_then(Value::as_str),
        "source_path": request.get("source_path").and_then(Value::as_str),
        "source_span": request.get("source_span").cloned(),
        "source_apply_session": {
            "kind": payload.pointer("/source_apply_session/kind").and_then(Value::as_str),
            "request_kind": request.pointer("/source_apply_session/kind").and_then(Value::as_str),
            "token_present": payload.pointer("/source_apply_session/token").and_then(Value::as_str).is_some(),
            "request_token_present": request.pointer("/source_apply_session/token").and_then(Value::as_str).is_some(),
        },
    })
}

fn string_array_contains(root: &Value, pointer: &str, expected: &str) -> bool {
    root.pointer(pointer)
        .and_then(Value::as_array)
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(expected)))
}

fn string_array_at<'a>(root: &'a Value, pointer: &str) -> Vec<&'a str> {
    root.pointer(pointer)
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn optional_bounded_string_array(
    root: &Value,
    pointer: &str,
    label: &str,
    max_items: usize,
    max_item_bytes: usize,
    reasons: &mut Vec<String>,
) -> Vec<String> {
    let Some(value) = root.pointer(pointer) else {
        return Vec::new();
    };
    let Some(values) = value.as_array() else {
        reasons.push(format!("{label} is not an array"));
        return Vec::new();
    };
    if values.len() > max_items {
        reasons.push(format!("{label} exceeds {max_items} item(s)"));
        return Vec::new();
    }

    let mut output = Vec::new();
    for value in values {
        let Some(value) = value.as_str() else {
            reasons.push(format!("{label} contains a non-string item"));
            continue;
        };
        if value.is_empty() {
            reasons.push(format!("{label} contains an empty item"));
            continue;
        }
        if value.len() > max_item_bytes {
            reasons.push(format!("{label} item exceeds {max_item_bytes} bytes"));
            continue;
        }
        output.push(value.to_string());
    }
    output
}

fn validate_contract_u64(contract: &Value, field: &str, expected: u64, reasons: &mut Vec<String>) {
    validate_named_contract_u64(contract, "source-apply contract", field, expected, reasons);
}

fn validate_named_contract_u64(
    contract: &Value,
    contract_name: &str,
    field: &str,
    expected: u64,
    reasons: &mut Vec<String>,
) {
    if contract.get(field).and_then(Value::as_u64) != Some(expected) {
        reasons.push(format!(
            "{contract_name} {field} does not match native limit"
        ));
    }
}

fn validate_reverse_delta_preview_provenance(
    preview: &Value,
    group_context: &Value,
    required_fields: &[&str],
    reasons: &mut Vec<String>,
) {
    let Some(status) = preview.get("status").and_then(Value::as_str) else {
        reasons.push("reverse CSS delta preview status is missing".to_string());
        return;
    };

    for field in required_fields {
        validate_required_preview_provenance_field(preview, group_context, field, reasons);
    }

    if status != "ready_for_review" {
        return;
    }
    if preview
        .get("target_utility")
        .and_then(Value::as_str)
        .is_none()
    {
        reasons.push("ready reverse CSS delta preview has no target utility".to_string());
    }
    let replacement_utility_count = preview
        .get("replacement_utilities")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if replacement_utility_count == 0 {
        reasons.push("ready reverse CSS delta preview has no replacement utilities".to_string());
    }
    if group_context
        .get("reverse_css_map_status")
        .and_then(Value::as_str)
        .is_none()
    {
        reasons
            .push("ready reverse CSS delta preview lacks reverse CSS map provenance".to_string());
    }
}

fn validate_required_preview_provenance_field(
    preview: &Value,
    group_context: &Value,
    field: &str,
    reasons: &mut Vec<String>,
) {
    match field {
        "group_status" => compare_required_str_or_null(
            preview,
            "group_status",
            group_context,
            "status",
            "reverse CSS delta preview group status",
            reasons,
        ),
        "group_alias" => compare_required_str_or_null(
            preview,
            "group_alias",
            group_context,
            "alias",
            "reverse CSS delta preview group alias",
            reasons,
        ),
        "group_syntax" => compare_required_str_or_null(
            preview,
            "group_syntax",
            group_context,
            "syntax",
            "reverse CSS delta preview group syntax",
            reasons,
        ),
        "group_expansion_status" => compare_required_str_or_null(
            preview,
            "group_expansion_status",
            group_context,
            "expansion_status",
            "reverse CSS delta preview group expansion status",
            reasons,
        ),
        "group_registry_receipt" => compare_required_str_or_null(
            preview,
            "group_registry_receipt",
            group_context,
            "registry_receipt",
            "reverse CSS delta preview group registry receipt",
            reasons,
        ),
        "reverse_css_map_receipt" => compare_required_str_or_null(
            preview,
            "reverse_css_map_receipt",
            group_context,
            "reverse_css_map_receipt",
            "reverse CSS delta preview reverse CSS map receipt",
            reasons,
        ),
        "reverse_css_map_status" => compare_required_str_or_null(
            preview,
            "reverse_css_map_status",
            group_context,
            "reverse_css_map_status",
            "reverse CSS delta preview reverse CSS map status",
            reasons,
        ),
        "group_source_state" => compare_required_str_or_null(
            preview,
            "group_source_state",
            group_context,
            "source_state",
            "reverse CSS delta preview group source state",
            reasons,
        ),
        "group_utility_count" => compare_required_u64_or_null(
            preview,
            "group_utility_count",
            group_context,
            "utility_count",
            "reverse CSS delta preview group utility count",
            reasons,
        ),
        unsupported => reasons.push(format!(
            "reverse CSS delta contract requires unsupported provenance field {unsupported}"
        )),
    }
}

fn compare_required_str_or_null(
    preview: &Value,
    preview_field: &str,
    context: &Value,
    context_field: &str,
    label: &str,
    reasons: &mut Vec<String>,
) {
    if preview.get(preview_field).is_none() {
        reasons.push(format!("{label} is missing from reverse CSS delta preview"));
        return;
    }
    if context.get(context_field).is_none() {
        reasons.push(format!("{label} is missing from active group context"));
        return;
    }
    let preview_value = preview.get(preview_field).and_then(Value::as_str);
    let context_value = context.get(context_field).and_then(Value::as_str);
    if preview_value != context_value {
        reasons.push(format!("{label} does not match active group context"));
    }
}

fn compare_required_u64_or_null(
    preview: &Value,
    preview_field: &str,
    context: &Value,
    context_field: &str,
    label: &str,
    reasons: &mut Vec<String>,
) {
    if preview.get(preview_field).is_none() {
        reasons.push(format!("{label} is missing from reverse CSS delta preview"));
        return;
    }
    if context.get(context_field).is_none() {
        reasons.push(format!("{label} is missing from active group context"));
        return;
    }
    let preview_value = preview.get(preview_field).and_then(Value::as_u64);
    let context_value = context.get(context_field).and_then(Value::as_u64);
    if preview_value != context_value {
        reasons.push(format!("{label} does not match active group context"));
    }
}

fn bounded_string<'a>(
    root: &'a Value,
    pointer: &str,
    label: &str,
    max_bytes: usize,
    reasons: &mut Vec<String>,
) -> Option<&'a str> {
    let Some(value) = root.pointer(pointer).and_then(Value::as_str) else {
        reasons.push(format!("{label} is missing"));
        return None;
    };
    if value.is_empty() {
        reasons.push(format!("{label} is empty"));
        return None;
    }
    if value.len() > max_bytes {
        reasons.push(format!("{label} exceeds {max_bytes} bytes"));
        return None;
    }
    Some(value)
}

fn bounded_optional_string<'a>(
    root: &'a Value,
    pointer: &str,
    label: &str,
    max_bytes: usize,
    reasons: &mut Vec<String>,
) -> Option<&'a str> {
    let Some(value) = root.pointer(pointer) else {
        return None;
    };
    let Some(value) = value.as_str() else {
        reasons.push(format!("{label} is not a string"));
        return None;
    };
    if value.len() > max_bytes {
        reasons.push(format!("{label} exceeds {max_bytes} bytes"));
        return None;
    }
    Some(value)
}

fn bounded_string_array(
    root: &Value,
    pointer: &str,
    label: &str,
    max_items: usize,
    max_item_bytes: usize,
    reasons: &mut Vec<String>,
) -> Vec<String> {
    let Some(values) = root.pointer(pointer).and_then(Value::as_array) else {
        reasons.push(format!("{label} is missing"));
        return Vec::new();
    };
    if values.is_empty() {
        reasons.push(format!("{label} is empty"));
        return Vec::new();
    }
    if values.len() > max_items {
        reasons.push(format!("{label} exceeds {max_items} item(s)"));
        return Vec::new();
    }

    let mut output = Vec::new();
    for value in values {
        let Some(value) = value.as_str() else {
            reasons.push(format!("{label} contains a non-string item"));
            continue;
        };
        if value.is_empty() {
            reasons.push(format!("{label} contains an empty item"));
            continue;
        }
        if value.len() > max_item_bytes {
            reasons.push(format!("{label} item exceeds {max_item_bytes} bytes"));
            continue;
        }
        output.push(value.to_string());
    }
    output
}

fn source_span_at(
    root: &Value,
    pointer: &str,
    label: &str,
    reasons: &mut Vec<String>,
) -> Option<SourceSpan> {
    let Some(span) = root.pointer(pointer) else {
        reasons.push(format!("{label} is missing"));
        return None;
    };
    let start = span.get("start_byte").and_then(Value::as_u64);
    let end = span.get("end_byte").and_then(Value::as_u64);
    let (Some(start), Some(end)) = (start, end) else {
        reasons.push(format!(
            "{label} must include integer start_byte and end_byte"
        ));
        return None;
    };
    if end < start {
        reasons.push(format!("{label} end_byte is before start_byte"));
        return None;
    }
    if end.saturating_sub(start) > MAX_SOURCE_SPAN_BYTES {
        reasons.push(format!("{label} exceeds {MAX_SOURCE_SPAN_BYTES} bytes"));
        return None;
    }
    Some(SourceSpan { start, end })
}

fn is_source_digest(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(SOURCE_DIGEST_PREFIX) else {
        return false;
    };
    digest.len() == 16 && digest.chars().all(|c| c.is_ascii_hexdigit())
}

fn span_json(span: Option<SourceSpan>) -> Option<Value> {
    span.map(|span| {
        json!({
            "start_byte": span.start,
            "end_byte": span.end,
        })
    })
}

fn bool_at(root: &Value, pointer: &str) -> Option<bool> {
    root.pointer(pointer).and_then(Value::as_bool)
}
