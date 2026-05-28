use std::{
    fs::{self, File, Metadata},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::{Map, Value};

use super::{DX_STUDIO_MAX_SOURCE_FILE_BYTES, values::string_at};

pub(super) fn source_file_snapshot(source: &Path, selection: &Value) -> Option<Value> {
    let metadata = fs::metadata(source).ok()?;
    let content_digest = source_content_digest(source)?;
    Some(serde_json::json!({
        "source_file": source.display().to_string(),
        "len": metadata.len(),
        "modified_ms": metadata.modified().ok().and_then(system_time_ms),
        "readonly": metadata.permissions().readonly(),
        "content_digest": content_digest,
        "selection_identity": selection_snapshot_identity(selection),
        "selection_identity_required": true,
    }))
}

pub(super) fn validate_expected_source_snapshot(
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

    validate_expected_selection_identity(expected, payload)?;

    let expected_digest = string_at(expected, &["/content_digest"]).ok_or_else(|| {
        anyhow!(
            "DX Studio refused source edit without a trusted source snapshot content identity for {}",
            source.display()
        )
    })?;
    let current_digest = source_content_digest(source).ok_or_else(|| {
        anyhow!(
            "DX Studio refused source edit because the trusted source snapshot content identity could not be read for {}",
            source.display()
        )
    })?;
    if expected_digest != current_digest {
        bail!(
            "DX Studio refused stale source file {}: content changed after selection",
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

pub(super) fn validate_expected_source_contents(
    source: &Path,
    contents: &str,
    payload: &Value,
) -> Result<()> {
    let Some(expected) = payload.get("expected_source_snapshot") else {
        bail!(
            "DX Studio refused source edit without a trusted Zed source snapshot for {}",
            source.display()
        );
    };
    let expected_digest = string_at(expected, &["/content_digest"]).ok_or_else(|| {
        anyhow!(
            "DX Studio refused source edit without a trusted source snapshot content identity for {}",
            source.display()
        )
    })?;
    let current_digest = content_digest(contents.as_bytes());
    if expected_digest != current_digest {
        bail!(
            "DX Studio refused stale source file {}: source content changed before write",
            source.display()
        );
    }
    Ok(())
}

pub(super) fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}

fn validate_expected_selection_identity(expected: &Value, payload: &Value) -> Result<()> {
    let Some(expected_identity) = expected
        .get("selection_identity")
        .and_then(Value::as_object)
    else {
        bail!("DX Studio refused source edit without a trusted source snapshot selection identity");
    };
    let Some(selection) = payload.get("selection") else {
        bail!("DX Studio refused source edit without a current selection identity");
    };
    let current_identity = selection_snapshot_identity(selection);
    let Some(current_identity) = current_identity.as_object() else {
        bail!("DX Studio refused source edit without a current selection identity");
    };

    let mut strong_identity_count = 0usize;
    for (key, expected_value) in expected_identity {
        let Some(expected_value) = expected_value.as_str().filter(|value| !value.is_empty()) else {
            continue;
        };
        let current_value = current_identity.get(key).and_then(Value::as_str);
        if current_value != Some(expected_value) {
            bail!(
                "DX Studio refused source edit because the trusted source snapshot selection identity does not match"
            );
        }
        if is_strong_selection_identity_key(key) {
            strong_identity_count += 1;
        }
    }

    if strong_identity_count == 0 {
        bail!(
            "DX Studio refused source edit without a narrow trusted source snapshot selection identity"
        );
    }

    Ok(())
}

fn selection_snapshot_identity(selection: &Value) -> Value {
    let mut identity = Map::new();
    insert_identity(
        &mut identity,
        "edit_id",
        selection,
        &["/edit_id", "/attributes/data-dx-edit-id"],
    );
    insert_identity(
        &mut identity,
        "text_marker",
        selection,
        &["/text_marker", "/attributes/data-dx-editable-text"],
    );
    insert_identity(
        &mut identity,
        "component",
        selection,
        &["/component", "/attributes/data-dx-component"],
    );
    insert_identity(
        &mut identity,
        "section",
        selection,
        &[
            "/section",
            "/attributes/data-dx-section",
            "/attributes/data-dx-editable-section",
        ],
    );
    insert_identity(
        &mut identity,
        "insert_slot",
        selection,
        &["/insert_slot", "/attributes/data-dx-insert-slot"],
    );
    insert_identity(
        &mut identity,
        "media_slot",
        selection,
        &["/media_slot", "/attributes/data-dx-media-slot"],
    );
    insert_identity(
        &mut identity,
        "reorder_group",
        selection,
        &["/reorder_group", "/attributes/data-dx-reorder-group"],
    );
    insert_identity(
        &mut identity,
        "design_token",
        selection,
        &["/design_token", "/attributes/data-dx-design-token"],
    );
    insert_identity(
        &mut identity,
        "token_scope",
        selection,
        &["/token_scope", "/attributes/data-dx-token-scope"],
    );
    insert_identity(
        &mut identity,
        "style_surface",
        selection,
        &["/style_surface", "/attributes/data-dx-style-surface"],
    );
    insert_identity(
        &mut identity,
        "route",
        selection,
        &["/route", "/attributes/data-dx-route"],
    );
    insert_identity(
        &mut identity,
        "source_file",
        selection,
        &[
            "/source_file",
            "/attributes/data-dx-source",
            "/attributes/data-dx-source-file",
        ],
    );
    Value::Object(identity)
}

fn insert_identity(
    identity: &mut Map<String, Value>,
    key: &str,
    selection: &Value,
    pointers: &[&str],
) {
    if let Some(value) = string_at(selection, pointers) {
        identity.insert(key.to_string(), Value::String(value));
    }
}

fn is_strong_selection_identity_key(key: &str) -> bool {
    !matches!(key, "route" | "source_file")
}

fn read_source_file_for_digest(source: &Path) -> Option<Vec<u8>> {
    let file = File::open(source).ok()?;
    let mut bytes = Vec::new();
    let mut limited = file.take(DX_STUDIO_MAX_SOURCE_FILE_BYTES + 1);
    limited.read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > DX_STUDIO_MAX_SOURCE_FILE_BYTES {
        return None;
    }

    Some(bytes)
}

fn source_content_digest(source: &Path) -> Option<String> {
    let contents = read_source_file_for_digest(source)?;
    Some(content_digest(&contents))
}

fn content_digest(contents: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in contents {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}
