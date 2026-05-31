use serde_json::{Value, json};

const MAX_DX_STYLE_DRY_RUN_EDIT_REPLAY_PREVIEWS: usize = 3;
const MAX_DX_STYLE_DRY_RUN_REPLACEMENT_TEXT_BYTES: usize = 4096;

pub(crate) fn native_writer_dry_run_replay(
    request: &Value,
    source: &str,
    source_path: &str,
    source_digest_before: &str,
    source_len_bytes_before: u64,
    request_source_span: (u64, u64),
) -> Value {
    let Some(edit_previews) = request
        .pointer("/context/apply_gate/receipt_summary/edit_previews")
        .and_then(Value::as_array)
    else {
        return blocked(
            "structured_edit_previews_missing",
            "DX Style native writer dry-run replay needs structured edit previews from a trusted dry-run receipt.",
            source_path,
            source_digest_before,
            source_len_bytes_before,
            request_source_span,
        );
    };
    if edit_previews.is_empty() {
        return blocked(
            "structured_edit_previews_empty",
            "DX Style native writer dry-run replay found no structured edit previews.",
            source_path,
            source_digest_before,
            source_len_bytes_before,
            request_source_span,
        );
    }
    if edit_previews.len() > MAX_DX_STYLE_DRY_RUN_EDIT_REPLAY_PREVIEWS {
        return blocked(
            "structured_edit_preview_count_exceeded",
            "DX Style native writer dry-run replay refuses oversized edit preview batches.",
            source_path,
            source_digest_before,
            source_len_bytes_before,
            request_source_span,
        );
    }

    for preview in edit_previews {
        let preview_source_path = preview.get("source_path").and_then(Value::as_str);
        if !preview_source_path.is_some_and(|path| source_paths_match(path, source_path)) {
            continue;
        }
        let Some((edit_start_byte, edit_end_byte)) = source_span_from_json(preview) else {
            return blocked(
                "structured_edit_preview_span_invalid",
                "DX Style native writer dry-run replay found a matching edit preview without a valid byte span.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        };
        if edit_start_byte > request_source_span.0 || request_source_span.1 > edit_end_byte {
            continue;
        }
        let Some(replacement_text) = preview.get("replacement_text").and_then(Value::as_str) else {
            return blocked(
                "replacement_text_missing",
                "DX Style native writer dry-run replay found a matching edit preview without replacement text.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        };
        if replacement_text.is_empty() {
            return blocked(
                "replacement_text_empty",
                "DX Style native writer dry-run replay refuses empty replacement text.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        }
        if replacement_text.len() > MAX_DX_STYLE_DRY_RUN_REPLACEMENT_TEXT_BYTES {
            return blocked(
                "replacement_text_too_large",
                "DX Style native writer dry-run replay refuses oversized replacement text.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        }
        let (Some(edit_start), Some(edit_end)) = (
            usize::try_from(edit_start_byte).ok(),
            usize::try_from(edit_end_byte).ok(),
        ) else {
            return blocked(
                "edit_span_out_of_range",
                "DX Style native writer dry-run replay edit span is too large for this platform.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        };
        if edit_start > edit_end
            || edit_end > source.len()
            || !source.is_char_boundary(edit_start)
            || !source.is_char_boundary(edit_end)
        {
            return blocked(
                "edit_span_not_replayable",
                "DX Style native writer dry-run replay edit span does not align with the live source text.",
                source_path,
                source_digest_before,
                source_len_bytes_before,
                request_source_span,
            );
        }

        let edited_source_len = source.len() - (edit_end - edit_start) + replacement_text.len();
        let mut edited_source = String::with_capacity(edited_source_len);
        edited_source.push_str(&source[..edit_start]);
        edited_source.push_str(replacement_text);
        edited_source.push_str(&source[edit_end..]);
        let source_digest_after =
            crate::dx_style_source_apply::active_source_digest(&edited_source);
        let replaced_source_digest =
            crate::dx_style_source_apply::active_source_digest(&source[edit_start..edit_end]);

        return json!({
            "schema": crate::dx_style_source_apply::DX_STYLE_NATIVE_WRITER_DRY_RUN_REPLAY_SCHEMA,
            "status": "matched",
            "reason": "Trusted dry-run edit preview replayed against live editor source in memory only.",
            "mutation_performed": false,
            "source_path": source_path,
            "source_digest_before": source_digest_before,
            "source_digest_after": source_digest_after,
            "source_len_bytes_before": source_len_bytes_before,
            "source_len_bytes_after": edited_source_len,
            "request_source_span": span_json(request_source_span),
            "edit_span": span_json((edit_start_byte, edit_end_byte)),
            "replacement_text_bytes": replacement_text.len(),
            "replaced_source_digest": replaced_source_digest,
            "replayed_edit_count": 1,
        });
    }

    blocked(
        "cursor_scoped_edit_preview_missing",
        "DX Style native writer dry-run replay found no trusted edit preview covering the requested source span.",
        source_path,
        source_digest_before,
        source_len_bytes_before,
        request_source_span,
    )
}

fn blocked(
    status: &'static str,
    reason: &'static str,
    source_path: &str,
    source_digest_before: &str,
    source_len_bytes_before: u64,
    request_source_span: (u64, u64),
) -> Value {
    json!({
        "schema": crate::dx_style_source_apply::DX_STYLE_NATIVE_WRITER_DRY_RUN_REPLAY_SCHEMA,
        "status": status,
        "reason": reason,
        "mutation_performed": false,
        "source_path": source_path,
        "source_digest_before": source_digest_before,
        "source_len_bytes_before": source_len_bytes_before,
        "request_source_span": span_json(request_source_span),
        "replayed_edit_count": 0,
    })
}

fn source_span_from_json(value: &Value) -> Option<(u64, u64)> {
    let start = value.get("start_byte").and_then(Value::as_u64)?;
    let end = value.get("end_byte").and_then(Value::as_u64)?;
    Some((start, end))
}

fn source_paths_match(left: &str, right: &str) -> bool {
    let left = left.trim().replace('\\', "/");
    let right = right.trim().replace('\\', "/");
    if cfg!(target_os = "windows") {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn span_json(span: (u64, u64)) -> Value {
    json!({
        "start_byte": span.0,
        "end_byte": span.1,
    })
}
