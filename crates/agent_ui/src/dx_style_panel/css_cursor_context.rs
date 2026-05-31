use super::css_hint_catalog::css_declaration_generator_hint;

pub(super) struct CssStyleHint {
    pub(super) ordinal: u64,
    pub(super) token: String,
    pub(super) generator_id: String,
    pub(super) property: String,
    pub(super) property_pattern: String,
    pub(super) property_match: String,
    pub(super) value_contains: Vec<String>,
    pub(super) source_edit_safety: String,
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn is_css_style_sheet_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    [".css", ".scss", ".sass", ".less", ".pcss"]
        .iter()
        .any(|extension| path.ends_with(extension))
}

pub(super) fn css_style_hint(source: &str, cursor: usize) -> Option<CssStyleHint> {
    let cursor = cursor.min(source.len());
    let start = declaration_start(source.as_bytes(), cursor);
    let end = declaration_end(source.as_bytes(), cursor);
    if start >= end {
        return None;
    }

    let declaration = &source[start..end];
    let colon = declaration.find(':')?;
    let property = declaration[..colon].trim();
    let value = declaration[colon + 1..].trim();
    if property.is_empty()
        || value.is_empty()
        || property.contains('{')
        || property.contains('}')
        || value.contains('{')
    {
        return None;
    }

    let hint = css_declaration_generator_hint(property, value)?;
    let property_offset = declaration.find(property).unwrap_or(0);
    Some(CssStyleHint {
        ordinal: hint.ordinal,
        token: hint.token,
        generator_id: hint.generator_id,
        property: property.to_ascii_lowercase(),
        property_pattern: hint.property_pattern,
        property_match: hint.property_match,
        value_contains: hint.value_contains,
        source_edit_safety: hint.source_edit_safety,
        start: start + property_offset,
        end,
    })
}

fn declaration_start(bytes: &[u8], cursor: usize) -> usize {
    let mut index = cursor;
    while index > 0 {
        let previous = bytes[index - 1];
        if matches!(previous, b';' | b'{' | b'}' | b'\n' | b'\r') {
            break;
        }
        index -= 1;
    }
    index
}

fn declaration_end(bytes: &[u8], cursor: usize) -> usize {
    let mut index = cursor;
    while index < bytes.len() {
        let byte = bytes[index];
        if matches!(byte, b';' | b'}' | b'\n' | b'\r') {
            break;
        }
        index += 1;
    }
    index
}
