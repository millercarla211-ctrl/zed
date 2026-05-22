use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use super::super::{
    source_ranges::{
        ElementRange, attribute_patterns, element_range_around_marker, find_all,
        unique_locator_position,
    },
    values::string_at,
};
use super::{SourceFileEdit, guards::validate_token_reference};

pub(super) fn apply_reorder_operation(
    contents: &str,
    selection: &Value,
    payload: &Value,
) -> Result<SourceFileEdit> {
    let group = string_at(
        payload,
        &[
            "/edit/reorder_group",
            "/reorder_group",
            "/selection/reorder_group",
        ],
    )
    .or_else(|| {
        string_at(
            selection,
            &["/reorder_group", "/attributes/data-dx-reorder-group"],
        )
    })
    .ok_or_else(|| anyhow!("DX Studio reorder edit is missing data-dx-reorder-group"))?;
    validate_token_reference(&group)?;

    let direction = string_at(payload, &["/edit/direction", "/direction"])
        .unwrap_or_else(|| "down".to_string());
    let direction = match direction.trim().to_ascii_lowercase().as_str() {
        "up" | "previous" | "before" => ReorderDirection::Up,
        "down" | "next" | "after" => ReorderDirection::Down,
        _ => bail!("DX Studio reorder direction must be `up` or `down`"),
    };

    let edit = reorder_group_section(contents, selection, &group, direction)?;
    Ok(SourceFileEdit {
        updated: edit.updated,
        changed_bytes: edit.changed_bytes,
        details: serde_json::json!({
            "reorder_group": group,
            "direction": direction.as_str(),
            "moved_marker": edit.moved_marker,
        }),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReorderDirection {
    Up,
    Down,
}

impl ReorderDirection {
    fn as_str(self) -> &'static str {
        match self {
            ReorderDirection::Up => "up",
            ReorderDirection::Down => "down",
        }
    }
}

#[derive(Debug)]
struct SourceReorderEdit {
    updated: String,
    changed_bytes: i64,
    moved_marker: Option<String>,
}

fn reorder_group_section(
    contents: &str,
    selection: &Value,
    group: &str,
    direction: ReorderDirection,
) -> Result<SourceReorderEdit> {
    let group_patterns = attribute_patterns("data-dx-reorder-group", group);
    let mut ranges = Vec::<ElementRange>::new();
    for pattern in &group_patterns {
        for marker_start in find_all(contents, pattern) {
            let range = element_range_around_marker(contents, marker_start)?;
            if !ranges
                .iter()
                .any(|candidate| candidate.start == range.start)
            {
                ranges.push(range);
            }
        }
    }
    ranges.sort_by_key(|range| range.start);

    if ranges.len() < 2 {
        bail!("DX Studio reorder group `{group}` needs at least two source sections");
    }

    let selected_anchor = unique_locator_position(contents, selection)?;
    let selected_index = ranges
        .iter()
        .position(|range| selected_anchor >= range.start && selected_anchor < range.end)
        .ok_or_else(|| anyhow!("Selected DX surface is not inside reorder group `{group}`"))?;

    let target_index = match direction {
        ReorderDirection::Up if selected_index > 0 => selected_index - 1,
        ReorderDirection::Down if selected_index + 1 < ranges.len() => selected_index + 1,
        ReorderDirection::Up => {
            bail!("Selected DX surface is already first in reorder group `{group}`")
        }
        ReorderDirection::Down => {
            bail!("Selected DX surface is already last in reorder group `{group}`")
        }
    };

    let (first_index, second_index) = if selected_index < target_index {
        (selected_index, target_index)
    } else {
        (target_index, selected_index)
    };
    let first = ranges[first_index];
    let second = ranges[second_index];
    if first.end > second.start {
        bail!("DX Studio refused overlapping reorder ranges in group `{group}`");
    }

    let first_text = &contents[first.start..first.end];
    let between = &contents[first.end..second.start];
    let second_text = &contents[second.start..second.end];
    let mut updated = String::with_capacity(contents.len());
    updated.push_str(&contents[..first.start]);
    updated.push_str(second_text);
    updated.push_str(between);
    updated.push_str(first_text);
    updated.push_str(&contents[second.end..]);

    Ok(SourceReorderEdit {
        changed_bytes: updated.len() as i64 - contents.len() as i64,
        updated,
        moved_marker: string_at(
            selection,
            &[
                "/edit_id",
                "/section",
                "/component",
                "/attributes/data-dx-edit-id",
                "/attributes/data-dx-section",
                "/attributes/data-dx-component",
            ],
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_selected_reorder_group_section_down() {
        let source = r#"<main>
  <section data-dx-edit-id="one" data-dx-reorder-group="launch-main">One</section>
  <section data-dx-edit-id="two" data-dx-reorder-group="launch-main">Two</section>
</main>"#;
        let selection = serde_json::json!({
            "edit_id": "one",
            "reorder_group": "launch-main",
            "operations": ["move_reorder_section"],
        });

        let edit = reorder_group_section(source, &selection, "launch-main", ReorderDirection::Down)
            .expect("reorder edit");

        let one = edit.updated.find(r#"data-dx-edit-id="one""#).unwrap();
        let two = edit.updated.find(r#"data-dx-edit-id="two""#).unwrap();
        assert!(two < one);
    }
}
