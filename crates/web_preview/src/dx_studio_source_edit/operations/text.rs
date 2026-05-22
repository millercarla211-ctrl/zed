use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use super::super::values::string_at;
use super::{
    SourceFileEdit,
    guards::{escape_jsx_text, normalize_text, validate_replacement_text},
};

pub(super) fn apply_text_operation(
    contents: &str,
    selection: &Value,
    payload: &Value,
) -> Result<SourceFileEdit> {
    let text_marker = string_at(
        selection,
        &["/text_marker", "/attributes/data-dx-editable-text"],
    )
    .ok_or_else(|| anyhow!("Selected DX surface has no data-dx-editable-text marker"))?;
    let previous_text = string_at(
        payload,
        &["/edit/previous_text", "/previous_text", "/selection/text"],
    )
    .unwrap_or_default();
    let replacement_text = string_at(payload, &["/edit/replacement_text", "/replacement_text"])
        .ok_or_else(|| anyhow!("DX Studio text edit is missing replacement text"))?;
    validate_replacement_text(&replacement_text)?;

    let edit = replace_text_marker(contents, &text_marker, &previous_text, &replacement_text)?;
    Ok(SourceFileEdit {
        updated: edit.updated,
        changed_bytes: edit.changed_bytes,
        details: serde_json::json!({
            "text_marker": text_marker,
            "previous_text": edit.previous_text,
            "replacement_text": replacement_text,
        }),
    })
}

#[derive(Debug)]
struct SourceTextEdit {
    updated: String,
    previous_text: String,
    changed_bytes: i64,
}

fn replace_text_marker(
    contents: &str,
    marker: &str,
    previous_text: &str,
    replacement_text: &str,
) -> Result<SourceTextEdit> {
    let marker_hits = marker_patterns(marker)
        .into_iter()
        .filter_map(|pattern| contents.find(&pattern))
        .collect::<Vec<_>>();

    if marker_hits.is_empty() {
        bail!("Source file does not contain data-dx-editable-text marker `{marker}`");
    }

    if marker_hits.len() > 1 {
        bail!("DX Studio refused ambiguous text edit: marker `{marker}` appears more than once");
    }

    let marker_start = marker_hits[0];
    let tag_end = contents[marker_start..]
        .find('>')
        .map(|offset| marker_start + offset + 1)
        .ok_or_else(|| anyhow!("Could not find the selected marker tag end"))?;
    let next_tag = contents[tag_end..]
        .find('<')
        .map(|offset| tag_end + offset)
        .ok_or_else(|| anyhow!("Could not find a direct text node for marker `{marker}`"))?;
    let direct_text = &contents[tag_end..next_tag];
    let direct_trimmed = direct_text.trim();

    if direct_trimmed.is_empty() || direct_trimmed.starts_with('{') {
        bail!(
            "DX Studio text marker `{marker}` is not a direct source literal; open the owning file for this edit"
        );
    }

    if !previous_text.trim().is_empty()
        && normalize_text(direct_trimmed) != normalize_text(previous_text)
    {
        bail!(
            "DX Studio refused stale text edit for `{marker}`: preview text no longer matches the source literal"
        );
    }

    let leading = direct_text
        .find(direct_trimmed)
        .map(|index| &direct_text[..index])
        .unwrap_or("");
    let trailing_start = direct_text
        .rfind(direct_trimmed)
        .map(|index| index + direct_trimmed.len())
        .unwrap_or(direct_text.len());
    let trailing = &direct_text[trailing_start..];
    let escaped = escape_jsx_text(replacement_text)?;

    let mut updated = String::with_capacity(contents.len() + escaped.len());
    updated.push_str(&contents[..tag_end]);
    updated.push_str(leading);
    updated.push_str(&escaped);
    updated.push_str(trailing);
    updated.push_str(&contents[next_tag..]);

    Ok(SourceTextEdit {
        changed_bytes: updated.len() as i64 - contents.len() as i64,
        updated,
        previous_text: direct_trimmed.to_string(),
    })
}

fn marker_patterns(marker: &str) -> Vec<String> {
    vec![
        format!("data-dx-editable-text=\"{marker}\""),
        format!("data-dx-editable-text='{marker}'"),
        format!("data-dx-editable-text={{\"{marker}\"}}"),
        format!("data-dx-editable-text={{'{marker}'}}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_direct_text_marker_literal() {
        let source = r#"<p data-dx-editable-text="launch-title">
  Original text
</p>"#;

        let edit = replace_text_marker(source, "launch-title", "Original text", "Better text")
            .expect("text edit");

        assert!(edit.updated.contains("Better text"));
        assert!(!edit.updated.contains("Original text"));
    }

    #[test]
    fn refuses_stale_text() {
        let source = r#"<p data-dx-editable-text="launch-title">Current</p>"#;
        let error = replace_text_marker(source, "launch-title", "Old", "New").unwrap_err();
        assert!(error.to_string().contains("stale text edit"));
    }
}
