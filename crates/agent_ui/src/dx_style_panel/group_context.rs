use serde_json::{Value, json};

use super::cursor_context_tokens::tokens_in_value;
use super::group_registry::registry_group_entry;
use super::reverse_css_map::reverse_css_map_summary;

const GROUP_CONTEXT_SCHEMA: &str = "zed.dx_style.group_context.v1";
const GROUP_CONTEXT_MAX_ALIAS_BYTES: usize = 128;
const GROUP_CONTEXT_MAX_UTILITY_COUNT: usize = 32;
const GROUP_CONTEXT_MAX_UTILITY_BYTES: usize = 256;
const GROUP_CONTEXT_CANDIDATE_MIN_UTILITY_COUNT: usize = 4;
const ATOMIC_KEYWORDS: [&str; 6] = ["flex", "grid", "block", "inline", "hidden", "contents"];

#[derive(Clone)]
pub(super) struct ActiveGroupContext {
    pub(super) status: String,
    pub(super) alias: Option<String>,
    pub(super) syntax: String,
    pub(super) utilities: Vec<String>,
    pub(super) expansion_status: String,
    pub(super) candidate_token_count: Option<usize>,
    pub(super) source_state: String,
    pub(super) registry_receipt: Option<String>,
    pub(super) reverse_css_map_receipt: Option<String>,
    pub(super) reverse_css_map_status: Option<String>,
}

impl ActiveGroupContext {
    pub(super) fn none() -> Self {
        Self {
            status: "none".to_string(),
            alias: None,
            syntax: "not_grouped".to_string(),
            utilities: Vec::new(),
            expansion_status: "not_available".to_string(),
            candidate_token_count: None,
            source_state: "no grouped class context at cursor".to_string(),
            registry_receipt: None,
            reverse_css_map_receipt: None,
            reverse_css_map_status: None,
        }
    }

    pub(super) fn from_tokens(
        token: Option<&str>,
        attribute_tokens: &[String],
        source_path: Option<&str>,
        workspace_root: Option<&str>,
    ) -> Self {
        if let Some(context) =
            token.and_then(|token| group_call_context(token, source_path, workspace_root))
        {
            return context;
        }
        if let Some(context) = attribute_tokens
            .iter()
            .find_map(|token| group_call_context(token, source_path, workspace_root))
        {
            return context;
        }
        if attribute_tokens.len() >= GROUP_CONTEXT_CANDIDATE_MIN_UTILITY_COUNT {
            return Self {
                status: "atomic_list_candidate".to_string(),
                alias: None,
                syntax: "static_atomic_utility_list".to_string(),
                utilities: bounded_utilities(attribute_tokens.iter().map(String::as_str)),
                expansion_status: "candidate_requires_project_repetition_scan".to_string(),
                candidate_token_count: Some(attribute_tokens.len()),
                source_state: "static class list is eligible for grouping analysis".to_string(),
                registry_receipt: None,
                reverse_css_map_receipt: None,
                reverse_css_map_status: None,
            };
        }
        Self::none()
    }

    pub(super) fn to_json(&self) -> Value {
        json!({
            "schema": GROUP_CONTEXT_SCHEMA,
            "status": self.status,
            "alias": self.alias,
            "syntax": self.syntax,
            "utilities": self.utilities,
            "utility_count": self.utilities.len(),
            "expansion_status": self.expansion_status,
            "candidate_token_count": self.candidate_token_count,
            "source_state": self.source_state,
            "requires_registry_receipt": self.status == "alias_reference" && self.registry_receipt.is_none(),
            "source_owned": self.syntax != "not_grouped" && (self.status != "alias_reference" || self.registry_receipt.is_some()),
            "can_expand_inline": !self.utilities.is_empty(),
            "registry_receipt": self.registry_receipt,
            "reverse_css_map_receipt": self.reverse_css_map_receipt,
            "reverse_css_map_status": self.reverse_css_map_status,
        })
    }

    pub(super) fn summary(&self) -> Option<String> {
        match (self.alias.as_ref(), self.candidate_token_count) {
            (Some(alias), _) => Some(format!("{alias} ({})", self.expansion_status)),
            (None, Some(count)) => Some(format!("{count} token grouping candidate")),
            _ => None,
        }
    }
}

fn group_call_context(
    token: &str,
    source_path: Option<&str>,
    workspace_root: Option<&str>,
) -> Option<ActiveGroupContext> {
    let (alias, body, source_declaration) = parse_group_call(token)?;
    if body.trim().is_empty() {
        if let Some(entry) = registry_group_entry(alias, source_path, workspace_root) {
            let reverse_css_map =
                reverse_css_map_summary(alias, entry.reverse_css_map_receipt.as_deref());
            return Some(ActiveGroupContext {
                status: "alias_reference_expanded".to_string(),
                alias: Some(alias.to_string()),
                syntax: "alias_reference".to_string(),
                utilities: entry.utilities,
                expansion_status: "registry_receipt_expansion_available".to_string(),
                candidate_token_count: None,
                source_state: "expanded from trusted DX Style registry receipt".to_string(),
                registry_receipt: Some(entry.receipt_path.display().to_string()),
                reverse_css_map_receipt: reverse_css_map
                    .as_ref()
                    .map(|summary| summary.receipt_path.display().to_string()),
                reverse_css_map_status: reverse_css_map.map(|summary| summary.status),
            });
        }
        return Some(ActiveGroupContext {
            status: "alias_reference".to_string(),
            alias: Some(alias.to_string()),
            syntax: "alias_reference".to_string(),
            utilities: Vec::new(),
            expansion_status: "needs_project_group_contract".to_string(),
            candidate_token_count: None,
            source_state: "alias needs a DX Style registry receipt before expansion".to_string(),
            registry_receipt: None,
            reverse_css_map_receipt: None,
            reverse_css_map_status: None,
        });
    }
    let utilities = bounded_utilities(tokens_in_value(body).iter().map(String::as_str));
    if !utilities
        .iter()
        .any(|utility| looks_like_atomic_utility(utility))
    {
        return None;
    }

    let (status, syntax) = if source_declaration {
        ("source_group_declaration", "source_declaration")
    } else {
        ("inline_group_declaration", "inline_utilities")
    };

    Some(ActiveGroupContext {
        status: status.to_string(),
        alias: Some(alias.to_string()),
        syntax: syntax.to_string(),
        utilities,
        expansion_status: "inline_utilities_available".to_string(),
        candidate_token_count: None,
        source_state: "group call carries inline atomics for Web Preview review".to_string(),
        registry_receipt: None,
        reverse_css_map_receipt: None,
        reverse_css_map_status: None,
    })
}

fn parse_group_call(token: &str) -> Option<(&str, &str, bool)> {
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

fn bounded_utilities<'a>(utilities: impl Iterator<Item = &'a str>) -> Vec<String> {
    utilities
        .filter(|utility| !utility.is_empty() && utility.len() <= GROUP_CONTEXT_MAX_UTILITY_BYTES)
        .take(GROUP_CONTEXT_MAX_UTILITY_COUNT)
        .map(str::to_string)
        .collect()
}

fn looks_like_atomic_utility(utility: &str) -> bool {
    utility.contains('-')
        || utility.contains(':')
        || utility.contains('[')
        || ATOMIC_KEYWORDS.contains(&utility)
}
