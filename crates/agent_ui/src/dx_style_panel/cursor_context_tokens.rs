const CURSOR_ATTRIBUTE_TOKEN_LIMIT: usize = 32;
const CURSOR_ATTRIBUTE_TOKEN_MAX_BYTES: usize = 256;

pub(super) fn tokens_in_value(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    for (offset, ch) in value.char_indices() {
        let is_separator = ch.is_whitespace() && bracket_depth == 0 && paren_depth == 0;
        if is_separator {
            push_attribute_token(value, &mut token_start, offset, &mut tokens);
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
    push_attribute_token(value, &mut token_start, value.len(), &mut tokens);
    tokens
}

fn push_attribute_token(
    value: &str,
    token_start: &mut Option<usize>,
    token_end: usize,
    tokens: &mut Vec<String>,
) {
    if tokens.len() >= CURSOR_ATTRIBUTE_TOKEN_LIMIT {
        return;
    }
    let Some(start) = token_start.take() else {
        return;
    };
    if start >= token_end {
        return;
    }
    let token = &value[start..token_end];
    if token.len() <= CURSOR_ATTRIBUTE_TOKEN_MAX_BYTES {
        tokens.push(token.to_string());
    }
}
