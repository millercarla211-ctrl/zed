use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use super::super::{
    source_ranges::{attribute_patterns, element_range_for_selection, find_all},
    values::string_at,
};
use super::{
    SourceFileEdit,
    guards::{validate_responsive_token_pair, validate_token_reference},
};

pub(super) fn apply_design_token_operation(
    contents: &str,
    selection: &Value,
    payload: &Value,
) -> Result<SourceFileEdit> {
    let old_token = string_at(
        payload,
        &["/edit/old_token", "/old_token", "/edit/current_token"],
    )
    .or_else(|| {
        string_at(
            selection,
            &["/design_token", "/attributes/data-dx-design-token"],
        )
    })
    .ok_or_else(|| anyhow!("DX Studio design-token edit is missing the existing token"))?;
    let new_token = string_at(
        payload,
        &[
            "/edit/new_token",
            "/new_token",
            "/edit/replacement_token",
            "/replacement_token",
        ],
    )
    .ok_or_else(|| anyhow!("DX Studio design-token edit is missing the replacement token"))?;
    validate_token_reference(&old_token)?;
    validate_token_reference(&new_token)?;
    if payload.pointer("/edit/responsive_layout").is_some()
        || is_responsive_token(&old_token)
        || is_responsive_token(&new_token)
    {
        validate_responsive_token_pair(&old_token, &new_token)?;
    }

    let edit = replace_design_token_reference(contents, selection, &old_token, &new_token)?;
    Ok(SourceFileEdit {
        updated: edit.updated,
        changed_bytes: edit.changed_bytes,
        details: serde_json::json!({
            "old_token": old_token,
            "new_token": new_token,
            "strategy": edit.strategy,
            "responsive_layout": payload.pointer("/edit/responsive_layout").cloned(),
        }),
    })
}

#[derive(Debug)]
struct SourceTokenEdit {
    updated: String,
    changed_bytes: i64,
    strategy: &'static str,
}

fn replace_design_token_reference(
    contents: &str,
    selection: &Value,
    old_token: &str,
    new_token: &str,
) -> Result<SourceTokenEdit> {
    if let Ok(updated) = replace_selected_attribute_value(
        contents,
        selection,
        "data-dx-design-token",
        old_token,
        new_token,
    ) {
        return Ok(SourceTokenEdit {
            changed_bytes: updated.len() as i64 - contents.len() as i64,
            updated,
            strategy: "data-dx-design-token",
        });
    }

    let range = element_range_for_selection(contents, selection)?;
    let segment = &contents[range.start..range.end];
    if !(segment.contains("className=") || segment.contains("class=")) {
        bail!(
            "DX Studio token edit found no class/className source literal on the selected surface"
        );
    }

    let occurrences = segment.match_indices(old_token).collect::<Vec<_>>();
    if occurrences.is_empty() {
        bail!("DX Studio token `{old_token}` was not found in the selected source surface");
    }
    if occurrences.len() > 1 {
        bail!(
            "DX Studio refused ambiguous token edit: `{old_token}` appears more than once in the selected source surface"
        );
    }

    let token_start = range.start + occurrences[0].0;
    let token_end = token_start + old_token.len();
    let mut updated = String::with_capacity(contents.len() + new_token.len());
    updated.push_str(&contents[..token_start]);
    updated.push_str(new_token);
    updated.push_str(&contents[token_end..]);

    Ok(SourceTokenEdit {
        changed_bytes: updated.len() as i64 - contents.len() as i64,
        updated,
        strategy: "class-token",
    })
}

fn replace_selected_attribute_value(
    contents: &str,
    selection: &Value,
    attribute: &str,
    old_value: &str,
    new_value: &str,
) -> Result<String> {
    let patterns = attribute_patterns(attribute, old_value);
    let exact_hits = patterns
        .iter()
        .flat_map(|pattern| {
            find_all(contents, pattern)
                .into_iter()
                .map(move |offset| (pattern, offset))
        })
        .collect::<Vec<_>>();

    if exact_hits.len() == 1 {
        let (pattern, start) = exact_hits[0];
        let old_start = start + pattern.find(old_value).unwrap_or(0);
        let old_end = old_start + old_value.len();
        let mut updated = String::with_capacity(contents.len() + new_value.len());
        updated.push_str(&contents[..old_start]);
        updated.push_str(new_value);
        updated.push_str(&contents[old_end..]);
        return Ok(updated);
    }

    let range = element_range_for_selection(contents, selection)?;
    let segment = &contents[range.start..range.end];
    let local_hits = patterns
        .iter()
        .flat_map(|pattern| {
            find_all(segment, pattern)
                .into_iter()
                .map(move |offset| (pattern, offset))
        })
        .collect::<Vec<_>>();
    if local_hits.len() != 1 {
        bail!("DX Studio refused ambiguous {attribute} edit for `{old_value}`");
    }

    let (pattern, local_start) = local_hits[0];
    let absolute_start = range.start + local_start + pattern.find(old_value).unwrap_or(0);
    let absolute_end = absolute_start + old_value.len();
    let mut updated = String::with_capacity(contents.len() + new_value.len());
    updated.push_str(&contents[..absolute_start]);
    updated.push_str(new_value);
    updated.push_str(&contents[absolute_end..]);
    Ok(updated)
}

fn is_responsive_token(token: &str) -> bool {
    matches!(
        token.split_once(':').map(|(prefix, _)| prefix),
        Some("xs" | "sm" | "md" | "lg" | "xl" | "2xl")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_selected_design_token_attribute() {
        let source = r#"<section data-dx-edit-id="launch.hero" data-dx-design-token="launch-hero-panel">
  Hero
</section>"#;
        let selection = serde_json::json!({
            "edit_id": "launch.hero",
            "design_token": "launch-hero-panel",
            "operations": ["update_design_token"],
        });

        let edit = replace_design_token_reference(
            source,
            &selection,
            "launch-hero-panel",
            "launch-hero-soft",
        )
        .expect("token edit");

        assert!(
            edit.updated
                .contains(r#"data-dx-design-token="launch-hero-soft""#)
        );
        assert!(
            !edit
                .updated
                .contains(r#"data-dx-design-token="launch-hero-panel""#)
        );
    }

    #[test]
    fn preserves_responsive_breakpoint_prefix() {
        let source = r#"<section data-dx-edit-id="launch.hero" className="grid md:grid-cols-2">Hero</section>"#;
        let selection = serde_json::json!({
            "edit_id": "launch.hero",
            "operations": ["update_design_token"],
        });

        let edit =
            replace_design_token_reference(source, &selection, "md:grid-cols-2", "md:grid-cols-3")
                .expect("responsive token edit");

        assert!(edit.updated.contains("md:grid-cols-3"));
        assert!(!edit.updated.contains("md:grid-cols-2"));
    }

    #[test]
    fn refuses_responsive_breakpoint_prefix_change() {
        let source = r#"<section data-dx-edit-id="launch.hero" className="grid md:grid-cols-2">Hero</section>"#;
        let selection = serde_json::json!({
            "edit_id": "launch.hero",
            "operations": ["update_design_token"],
        });
        let payload = serde_json::json!({
            "edit": {
                "old_token": "md:grid-cols-2",
                "new_token": "lg:grid-cols-3",
                "responsive_layout": {
                    "active_breakpoint": "md",
                    "responsive_policy": "use-existing-grid-and-design-tokens"
                }
            }
        });

        let error = apply_design_token_operation(source, &selection, &payload)
            .expect_err("prefix mismatch");

        assert!(error.to_string().contains("preserve the breakpoint prefix"));
    }
}
