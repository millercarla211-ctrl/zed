use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use super::values::{string_at, unique_strings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ElementRange {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn attribute_patterns(attribute: &str, value: &str) -> Vec<String> {
    vec![
        format!("{attribute}=\"{value}\""),
        format!("{attribute}='{value}'"),
        format!("{attribute}={{\"{value}\"}}"),
        format!("{attribute}={{'{value}'}}"),
        format!("{attribute}=`{value}`"),
    ]
}

pub(super) fn element_range_for_selection(
    contents: &str,
    selection: &Value,
) -> Result<ElementRange> {
    let marker_start = unique_locator_position(contents, selection)?;
    element_range_around_marker(contents, marker_start)
}

pub(super) fn unique_locator_position(contents: &str, selection: &Value) -> Result<usize> {
    for pattern in locator_patterns(selection) {
        let hits = find_all(contents, &pattern);
        if hits.len() == 1 {
            return Ok(hits[0]);
        }
    }
    bail!("DX Studio could not locate a unique source marker for the selected surface");
}

pub(super) fn element_range_around_marker(
    contents: &str,
    marker_start: usize,
) -> Result<ElementRange> {
    let open_start = contents[..=marker_start]
        .rfind('<')
        .ok_or_else(|| anyhow!("Could not find selected source tag start"))?;
    let tag_name = tag_name_at(contents, open_start)?;
    let open_end = find_tag_end(contents, open_start)
        .ok_or_else(|| anyhow!("Could not find selected source tag end"))?;

    if is_self_closing_tag(contents, open_start, open_end) {
        return Ok(ElementRange {
            start: open_start,
            end: open_end + 1,
        });
    }

    let close_start = matching_close_tag_start(contents, open_end + 1, &tag_name)
        .ok_or_else(|| anyhow!("Could not find closing tag for `{tag_name}`"))?;
    let close_end = find_tag_end(contents, close_start)
        .ok_or_else(|| anyhow!("Could not find closing tag end for `{tag_name}`"))?;
    Ok(ElementRange {
        start: open_start,
        end: close_end + 1,
    })
}

pub(super) fn closing_tag_start(contents: &str, range: ElementRange) -> Option<usize> {
    contents[range.start..range.end]
        .rfind("</")
        .map(|offset| range.start + offset)
}

pub(super) fn find_all(contents: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut offset = 0usize;
    while let Some(index) = contents[offset..].find(needle) {
        let absolute = offset + index;
        hits.push(absolute);
        offset = absolute + needle.len();
    }
    hits
}

fn locator_patterns(selection: &Value) -> Vec<String> {
    let mut patterns = Vec::new();
    for (attribute, pointers) in [
        (
            "data-dx-editable-text",
            &["/text_marker", "/attributes/data-dx-editable-text"][..],
        ),
        (
            "data-dx-edit-id",
            &["/edit_id", "/attributes/data-dx-edit-id"][..],
        ),
        (
            "data-dx-design-token",
            &["/design_token", "/attributes/data-dx-design-token"][..],
        ),
        (
            "data-dx-insert-slot",
            &["/insert_slot", "/attributes/data-dx-insert-slot"][..],
        ),
        (
            "data-dx-media-slot",
            &["/media_slot", "/attributes/data-dx-media-slot"][..],
        ),
        (
            "data-dx-section",
            &["/section", "/attributes/data-dx-section"][..],
        ),
        (
            "data-dx-editable-section",
            &["/section", "/attributes/data-dx-editable-section"][..],
        ),
        (
            "data-dx-component",
            &["/component", "/attributes/data-dx-component"][..],
        ),
        (
            "data-dx-reorder-group",
            &["/reorder_group", "/attributes/data-dx-reorder-group"][..],
        ),
    ] {
        if let Some(value) = string_at(selection, pointers) {
            patterns.extend(attribute_patterns(attribute, &value));
        }
    }
    unique_strings(patterns)
}

fn tag_name_at(contents: &str, open_start: usize) -> Result<String> {
    let bytes = contents.as_bytes();
    if bytes.get(open_start) != Some(&b'<') || bytes.get(open_start + 1) == Some(&b'/') {
        bail!("Selected marker is not inside an opening tag");
    }
    let mut index = open_start + 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let name_start = index;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.') {
            index += 1;
        } else {
            break;
        }
    }
    if index == name_start {
        bail!("Could not parse selected tag name");
    }
    Ok(contents[name_start..index].to_string())
}

fn find_tag_end(contents: &str, open_start: usize) -> Option<usize> {
    let bytes = contents.as_bytes();
    let mut quote: Option<u8> = None;
    let mut index = open_start + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote && bytes.get(index.wrapping_sub(1)) != Some(&b'\\') {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
        } else if byte == b'>' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_self_closing_tag(contents: &str, open_start: usize, open_end: usize) -> bool {
    contents[open_start..open_end].trim_end().ends_with('/')
}

fn matching_close_tag_start(contents: &str, from: usize, tag_name: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut index = from;
    while index < contents.len() {
        let relative = contents[index..].find('<')?;
        let tag_start = index + relative;
        if contents[tag_start..].starts_with("</") {
            if tag_boundary_matches(contents, tag_start + 2, tag_name) {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(tag_start);
                }
            }
        } else if tag_boundary_matches(contents, tag_start + 1, tag_name)
            && let Some(tag_end) = find_tag_end(contents, tag_start)
            && !is_self_closing_tag(contents, tag_start, tag_end)
        {
            depth += 1;
        }
        index = find_tag_end(contents, tag_start)
            .map(|tag_end| tag_end + 1)
            .unwrap_or(tag_start + 1);
    }
    None
}

fn tag_boundary_matches(contents: &str, name_start: usize, tag_name: &str) -> bool {
    contents[name_start..]
        .strip_prefix(tag_name)
        .and_then(|rest| rest.as_bytes().first().copied())
        .map(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/' | b'{'))
        .unwrap_or(false)
}
