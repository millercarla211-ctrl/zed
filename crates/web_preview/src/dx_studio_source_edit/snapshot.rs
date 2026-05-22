use std::{
    fs::{self, Metadata},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;

use super::values::string_at;

pub(super) fn source_file_snapshot(source: &Path) -> Option<Value> {
    let metadata = fs::metadata(source).ok()?;
    Some(serde_json::json!({
        "source_file": source.display().to_string(),
        "len": metadata.len(),
        "modified_ms": metadata.modified().ok().and_then(system_time_ms),
        "readonly": metadata.permissions().readonly(),
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

pub(super) fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
