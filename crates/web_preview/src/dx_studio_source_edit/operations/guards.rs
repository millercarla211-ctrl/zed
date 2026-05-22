use anyhow::{Result, bail};

pub(super) fn validate_replacement_text(text: &str) -> Result<()> {
    if text.contains('{') || text.contains('}') {
        bail!("DX Studio text edit does not write JSX expression braces from the preview");
    }
    Ok(())
}

pub(super) fn validate_token_reference(token: &str) -> Result<()> {
    if token.trim().is_empty()
        || token.chars().any(|character| {
            matches!(
                character,
                '<' | '>' | '"' | '\'' | '`' | '{' | '}' | ';' | '\n' | '\r'
            )
        })
    {
        bail!("DX Studio token references cannot contain markup, quotes, braces, or newlines");
    }
    Ok(())
}

pub(super) fn validate_responsive_token_pair(old_token: &str, new_token: &str) -> Result<()> {
    let old_breakpoint = breakpoint_prefix(old_token);
    let new_breakpoint = breakpoint_prefix(new_token);
    if old_breakpoint != new_breakpoint {
        bail!(
            "DX Studio responsive layout edits must preserve the breakpoint prefix for `{old_token}`"
        );
    }
    Ok(())
}

pub(super) fn validate_insert_snippet(snippet: &str) -> Result<()> {
    let lower = snippet.to_ascii_lowercase();
    if snippet.trim().is_empty()
        || lower.contains("<script")
        || lower.contains("dangerouslysetinnerhtml")
        || lower.contains("style=")
        || lower.contains("position:absolute")
        || lower.contains("position:fixed")
    {
        bail!("DX Studio refused unsafe insert template");
    }
    if !(snippet.contains("data-dx-component")
        || snippet.contains("data-dx-edit-id")
        || snippet.contains("data-dx-media-slot"))
    {
        bail!("DX Studio insert templates must carry stable data-dx ownership markers");
    }
    Ok(())
}

pub(super) fn escape_jsx_text(text: &str) -> Result<String> {
    validate_replacement_text(text)?;
    Ok(text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;"))
}

pub(super) fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn breakpoint_prefix(token: &str) -> Option<&str> {
    ["xs:", "sm:", "md:", "lg:", "xl:", "2xl:"]
        .into_iter()
        .find(|prefix| token.starts_with(*prefix))
}
