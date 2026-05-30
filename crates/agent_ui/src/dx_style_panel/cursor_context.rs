use super::cursor_context_tokens::tokens_in_value;

pub(super) enum CursorStyleToken {
    Token {
        token: String,
        start: usize,
        end: usize,
        attribute_tokens: Vec<String>,
    },
    StaticAttribute {
        attribute_tokens: Vec<String>,
    },
    DynamicAttribute,
    NonLiteralAttribute,
    UnterminatedAttribute,
    Outside,
}

pub(super) fn cursor_style_token(source: &str, cursor: usize) -> CursorStyleToken {
    let bytes = source.as_bytes();
    let mut pos = 0usize;

    while pos < bytes.len() {
        if !bytes[pos..].starts_with(b"class") || (pos > 0 && is_identifier_part(bytes[pos - 1])) {
            pos += 1;
            continue;
        }
        if !has_open_tag_before(bytes, pos) {
            pos += 5;
            continue;
        }

        let name_len =
            if pos + 9 <= bytes.len() && bytes[pos + 5..pos + 9].eq_ignore_ascii_case(b"Name") {
                9usize
            } else {
                5usize
            };
        let mut scan = pos + name_len;
        if scan < bytes.len() && is_identifier_part(bytes[scan]) {
            pos += name_len;
            continue;
        }

        skip_ascii_whitespace(bytes, &mut scan);
        if scan >= bytes.len() || bytes[scan] != b'=' {
            pos += name_len;
            continue;
        }
        scan += 1;
        skip_ascii_whitespace(bytes, &mut scan);
        if scan >= bytes.len() {
            return CursorStyleToken::UnterminatedAttribute;
        }

        match bytes[scan] {
            b'"' | b'\'' => {
                let value_start = scan + 1;
                let Some(value_end) = find_quoted_literal_end(bytes, value_start, bytes[scan])
                else {
                    return CursorStyleToken::UnterminatedAttribute;
                };
                if cursor >= pos && cursor <= value_end + 1 {
                    let value = &source[value_start..value_end];
                    let attribute_tokens = tokens_in_value(value);
                    if cursor >= value_start && cursor <= value_end {
                        return token_in_value(value, value_start, cursor, &attribute_tokens)
                            .unwrap_or(CursorStyleToken::StaticAttribute { attribute_tokens });
                    }
                    return CursorStyleToken::StaticAttribute { attribute_tokens };
                }
                pos = value_end + 1;
            }
            b'{' => {
                let expression_end = find_jsx_expression_end(bytes, scan).unwrap_or(bytes.len());
                if cursor >= pos && cursor <= expression_end {
                    return CursorStyleToken::DynamicAttribute;
                }
                pos = expression_end;
            }
            _ => {
                let value_end = find_unquoted_value_end(bytes, scan);
                if cursor >= pos && cursor <= value_end {
                    return CursorStyleToken::NonLiteralAttribute;
                }
                pos = value_end;
            }
        }
    }

    CursorStyleToken::Outside
}

pub(super) fn is_style_bearing_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    [
        ".css", ".scss", ".sass", ".less", ".pcss", ".html", ".htm", ".tsx", ".jsx", ".vue",
        ".svelte", ".astro",
    ]
    .iter()
    .any(|extension| path.ends_with(extension))
}

fn token_in_value(
    value: &str,
    value_start: usize,
    cursor: usize,
    attribute_tokens: &[String],
) -> Option<CursorStyleToken> {
    let mut token_start = None;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    for (offset, ch) in value.char_indices() {
        let is_separator = ch.is_whitespace() && bracket_depth == 0 && paren_depth == 0;
        if is_separator {
            if let Some(token) = close_token(
                value,
                value_start,
                token_start.take(),
                offset,
                cursor,
                attribute_tokens,
            ) {
                return Some(token);
            }
            continue;
        }

        token_start.get_or_insert(offset);
        match ch {
            '[' => bracket_depth = bracket_depth.saturating_add(1),
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' if bracket_depth == 0 => paren_depth = paren_depth.saturating_add(1),
            ')' if bracket_depth == 0 => paren_depth = paren_depth.saturating_sub(1),
            _ => {}
        }
    }

    close_token(
        value,
        value_start,
        token_start,
        value.len(),
        cursor,
        attribute_tokens,
    )
}

fn close_token(
    value: &str,
    value_start: usize,
    token_start: Option<usize>,
    token_end: usize,
    cursor: usize,
    attribute_tokens: &[String],
) -> Option<CursorStyleToken> {
    let start = token_start?;
    let source_start = value_start + start;
    let source_end = value_start + token_end;
    (source_start <= cursor && cursor <= source_end).then(|| CursorStyleToken::Token {
        token: value[start..token_end].to_string(),
        start: source_start,
        end: source_end,
        attribute_tokens: attribute_tokens.to_vec(),
    })
}

fn has_open_tag_before(bytes: &[u8], attr_start: usize) -> bool {
    let mut cursor = attr_start;
    let mut scanned = 0usize;
    while cursor > 0 && scanned < 512 {
        cursor -= 1;
        match bytes[cursor] {
            b'<' => return true,
            b'>' | b';' | b'{' | b'}' => return false,
            _ => {}
        }
        scanned += 1;
    }
    false
}

fn skip_ascii_whitespace(bytes: &[u8], cursor: &mut usize) {
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
}

fn find_quoted_literal_end(bytes: &[u8], value_start: usize, quote: u8) -> Option<usize> {
    let mut cursor = value_start;
    let mut escaped = false;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn find_jsx_expression_end(bytes: &[u8], expression_start: usize) -> Option<usize> {
    let mut cursor = expression_start;
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }

        match byte {
            b'\'' | b'"' | b'`' => quote = Some(byte),
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor + 1);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn find_unquoted_value_end(bytes: &[u8], value_start: usize) -> usize {
    let mut cursor = value_start;
    while cursor < bytes.len()
        && !bytes[cursor].is_ascii_whitespace()
        && !matches!(bytes[cursor], b'>' | b'/')
    {
        cursor += 1;
    }
    cursor
}

fn is_identifier_part(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}
