pub(super) const GROUP_CONTEXT_MAX_ALIAS_BYTES: usize = 128;
pub(super) const GROUP_CONTEXT_MAX_UTILITY_COUNT: usize = 32;
pub(super) const GROUP_CONTEXT_MAX_UTILITY_BYTES: usize = 256;
pub(super) const GROUP_CONTEXT_CANDIDATE_MIN_UTILITY_COUNT: usize = 4;

const ATOMIC_KEYWORDS: [&str; 6] = ["flex", "grid", "block", "inline", "hidden", "contents"];

pub(super) fn parse_group_call(token: &str) -> Option<(&str, &str, bool)> {
    let trimmed = token.trim();
    let source_declaration = trimmed.starts_with('@');
    let body_end = trimmed.strip_suffix(')')?;
    let open = body_end.find('(')?;
    let alias = body_end[..open]
        .trim()
        .strip_prefix('@')
        .unwrap_or(body_end[..open].trim());
    if alias.is_empty()
        || alias.len() > GROUP_CONTEXT_MAX_ALIAS_BYTES
        || !alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || !alias.as_bytes()[0].is_ascii_alphabetic()
    {
        return None;
    }
    Some((alias, &body_end[open + 1..], source_declaration))
}

pub(super) fn bounded_utilities<'a>(utilities: impl Iterator<Item = &'a str>) -> Vec<String> {
    utilities
        .filter(|utility| !utility.is_empty() && utility.len() <= GROUP_CONTEXT_MAX_UTILITY_BYTES)
        .take(GROUP_CONTEXT_MAX_UTILITY_COUNT)
        .map(str::to_string)
        .collect()
}

pub(super) fn looks_like_atomic_utility(utility: &str) -> bool {
    utility.contains('-')
        || utility.contains(':')
        || utility.contains('[')
        || ATOMIC_KEYWORDS.contains(&utility)
}
