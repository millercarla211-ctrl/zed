use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_SERIALIZER_ROOT_ENV: &str = "DX_SERIALIZER_ROOT";
const DX_RLM_ROOT_ENV: &str = "DX_RLM_ROOT";
const DEFAULT_SERIALIZER_ROOT: &str = r"G:\Workspaces\flow\serializer";
const DEFAULT_RLM_ROOT: &str = r"G:\Workspaces\flow\rlm";
const SOURCE_PACK_SCHEMA: &str = "zed.dx.metasearch.source_pack.v1";
const SOURCE_EXTRACT_SCHEMA: &str = "zed.dx.metasearch.source_extract.v1";
const DEFAULT_CONTEXT_TOKEN_BUDGET: usize = 1_600;
const MIN_CONTEXT_TOKEN_BUDGET: usize = 200;
const MAX_CONTEXT_TOKEN_BUDGET: usize = 8_000;
const APPROX_CHARS_PER_TOKEN: usize = 4;
const MIN_SOURCE_CONTEXT_CHARS: usize = 160;

pub(crate) const DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA: &str =
    "zed.dx.serializer_rlm.context_bundle.v1";
pub(crate) const DX_METASEARCH_CONTEXT_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.context_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxMetasearchContextAdapterRequest {
    pub source_pack: Option<Value>,
    pub source_extracts: Vec<Value>,
    pub question: Option<String>,
    pub token_budget: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextBundle {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxMetasearchContextRequestSummary,
    pub adapter: DxMetasearchContextAdapterStatus,
    pub summary: DxMetasearchContextSummary,
    pub compression: DxMetasearchContextCompression,
    pub context_text: String,
    pub items: Vec<DxMetasearchContextItem>,
    pub context_receipt: Option<DxMetasearchContextReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextRequestSummary {
    pub question: Option<String>,
    pub token_budget: usize,
    pub char_budget: usize,
    pub source_pack_provided: bool,
    pub source_extract_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextAdapterStatus {
    pub mode: &'static str,
    pub external_execution: bool,
    pub serializer: DxMetasearchContextExternalRoot,
    pub rlm: DxMetasearchContextExternalRoot,
    pub rlm_document_contract: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextExternalRoot {
    pub root: String,
    pub root_exists: bool,
    pub integration_file: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextSummary {
    pub input_source_count: usize,
    pub included_source_count: usize,
    pub omitted_source_count: usize,
    pub estimated_chars: usize,
    pub estimated_tokens: usize,
    pub budget_exceeded: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextCompression {
    pub profile: &'static str,
    pub machine_format: &'static str,
    pub approx_chars_per_token: usize,
    pub serializer_ready: bool,
    pub rlm_ready: bool,
    pub loss_policy: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextItem {
    pub source_id: String,
    pub title: String,
    pub url: String,
    pub engine: Option<String>,
    pub category: Option<String>,
    pub content_kind: Option<String>,
    pub source_kind: &'static str,
    pub estimated_chars: usize,
    pub estimated_tokens: usize,
    pub truncated: bool,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchContextReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub context_bundle_schema: &'static str,
    pub item_count: usize,
    pub estimated_tokens: usize,
    pub next_action: String,
}

#[derive(Clone, Debug)]
struct SourceCandidate {
    source_id: String,
    title: String,
    url: String,
    engine: Option<String>,
    category: Option<String>,
    content_kind: Option<String>,
    source_kind: &'static str,
    text: String,
}

pub(crate) fn build_metasearch_context_bundle(
    request: DxMetasearchContextAdapterRequest,
) -> Result<DxMetasearchContextBundle, String> {
    let token_budget = request
        .token_budget
        .unwrap_or(DEFAULT_CONTEXT_TOKEN_BUDGET)
        .clamp(MIN_CONTEXT_TOKEN_BUDGET, MAX_CONTEXT_TOKEN_BUDGET);
    let char_budget = token_budget.saturating_mul(APPROX_CHARS_PER_TOKEN);
    let mut warnings = Vec::new();
    let mut candidates = Vec::new();
    let mut candidate_index = BTreeMap::new();

    if let Some(source_pack) = &request.source_pack {
        let source_pack = decode_json_string(source_pack)?;
        if let Some(pack) = locate_schema_object(&source_pack, SOURCE_PACK_SCHEMA, "source_pack") {
            collect_source_pack_items(pack, &mut candidates, &mut candidate_index);
        } else {
            warnings.push(
                "source_pack was provided but did not contain zed.dx.metasearch.source_pack.v1."
                    .to_string(),
            );
        }
    }

    for source_extract in &request.source_extracts {
        let source_extract = decode_json_string(source_extract)?;
        if let Some(extract) =
            locate_schema_object(&source_extract, SOURCE_EXTRACT_SCHEMA, "source_extract")
        {
            collect_source_extract(extract, &mut candidates, &mut candidate_index);
        } else {
            warnings.push(
                "A source_extract entry did not contain zed.dx.metasearch.source_extract.v1."
                    .to_string(),
            );
        }
    }

    if candidates.is_empty() {
        return Err(
            "DX metasearch context adapter needs at least one source-pack item or source extract."
                .to_string(),
        );
    }

    let (items, context_text, estimated_chars) = pack_context_items(&candidates, char_budget);
    let omitted_source_count = candidates.len().saturating_sub(items.len());
    let estimated_tokens = estimate_token_count(estimated_chars);
    let budget_exceeded = omitted_source_count > 0
        || candidates.iter().any(|candidate| {
            let included = items
                .iter()
                .find(|item| item.source_id == candidate.source_id && item.url == candidate.url);
            included.is_some_and(|item| item.truncated)
        });
    if omitted_source_count > 0 {
        warnings.push(format!(
            "{omitted_source_count} source(s) were omitted to stay inside the context budget."
        ));
    }

    let next_action = if budget_exceeded {
        "Use this compact context receipt for the next Agent call, then call plan_dx_serializer_rlm_execution when a larger source set must be reduced further.".to_string()
    } else {
        "Use context_text directly for cited Agent context, or call plan_dx_serializer_rlm_execution to create the approved external reducer handoff.".to_string()
    };

    Ok(DxMetasearchContextBundle {
        schema: DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMetasearchContextRequestSummary {
            question: clean_optional_text(request.question, 240),
            token_budget,
            char_budget,
            source_pack_provided: request.source_pack.is_some(),
            source_extract_count: request.source_extracts.len(),
        },
        adapter: DxMetasearchContextAdapterStatus {
            mode: "source_only_contract",
            external_execution: false,
            serializer: external_root(
                DX_SERIALIZER_ROOT_ENV,
                DEFAULT_SERIALIZER_ROOT,
                &["src", "llm", "packed.rs"],
            ),
            rlm: external_root(DX_RLM_ROOT_ENV, DEFAULT_RLM_ROOT, &["src", "rlm.rs"]),
            rlm_document_contract: "RLMDocument { id, title, content, source_path, mime_type, tags }",
        },
        summary: DxMetasearchContextSummary {
            input_source_count: candidates.len(),
            included_source_count: items.len(),
            omitted_source_count,
            estimated_chars,
            estimated_tokens,
            budget_exceeded,
            warnings,
        },
        compression: DxMetasearchContextCompression {
            profile: "dx_metasearch.serializer_rlm.source_only_context.v1",
            machine_format: "json:zed.dx.serializer_rlm.context_bundle.v1",
            approx_chars_per_token: APPROX_CHARS_PER_TOKEN,
            serializer_ready: true,
            rlm_ready: true,
            loss_policy: "preserve source id, title, URL, engine, category, content kind, and bounded cited text; omit lower-priority overflow sources before dropping metadata",
        },
        context_text,
        items,
        context_receipt: None,
        next_action,
    })
}

fn collect_source_pack_items(
    pack: &Value,
    candidates: &mut Vec<SourceCandidate>,
    candidate_index: &mut BTreeMap<String, usize>,
) {
    let Some(items) = pack.get("items").and_then(Value::as_array) else {
        return;
    };

    for (index, item) in items.iter().enumerate() {
        let source_id =
            string_field(item, "source_id").unwrap_or_else(|| format!("S{}", index + 1));
        let candidate = SourceCandidate {
            source_id,
            title: string_field(item, "title").unwrap_or_else(|| "Untitled source".to_string()),
            url: string_field(item, "url").unwrap_or_default(),
            engine: string_field(item, "engine"),
            category: string_field(item, "category"),
            content_kind: None,
            source_kind: "source_pack_excerpt",
            text: string_field(item, "excerpt").unwrap_or_default(),
        };
        merge_candidate(candidate, candidates, candidate_index);
    }
}

fn collect_source_extract(
    extract: &Value,
    candidates: &mut Vec<SourceCandidate>,
    candidate_index: &mut BTreeMap<String, usize>,
) {
    let request = extract.get("request").unwrap_or(&Value::Null);
    let content = extract.get("content").unwrap_or(&Value::Null);
    let source_id = string_field(request, "source_id")
        .filter(|source_id| !source_id.trim().is_empty())
        .unwrap_or_else(|| format!("E{}", candidates.len() + 1));
    let candidate = SourceCandidate {
        source_id,
        title: string_field(request, "title").unwrap_or_else(|| "Extracted source".to_string()),
        url: string_field(request, "url").unwrap_or_default(),
        engine: string_field(request, "engine"),
        category: string_field(request, "category"),
        content_kind: string_field(content, "kind"),
        source_kind: "source_extract_text",
        text: string_field(content, "text").unwrap_or_default(),
    };
    merge_candidate(candidate, candidates, candidate_index);
}

fn merge_candidate(
    candidate: SourceCandidate,
    candidates: &mut Vec<SourceCandidate>,
    candidate_index: &mut BTreeMap<String, usize>,
) {
    if candidate.url.trim().is_empty() && candidate.text.trim().is_empty() {
        return;
    }

    let key = if !candidate.source_id.trim().is_empty() {
        format!("id:{}", candidate.source_id)
    } else {
        format!("url:{}", candidate.url)
    };

    if let Some(index) = candidate_index.get(&key).copied() {
        let existing = &mut candidates[index];
        if candidate.text.chars().count() > existing.text.chars().count() {
            existing.text = candidate.text;
            existing.source_kind = candidate.source_kind;
            existing.content_kind = candidate.content_kind;
        }
        if existing.title == "Untitled source" || existing.title == "Extracted source" {
            existing.title = candidate.title;
        }
        if existing.url.is_empty() {
            existing.url = candidate.url;
        }
        if existing.engine.is_none() {
            existing.engine = candidate.engine;
        }
        if existing.category.is_none() {
            existing.category = candidate.category;
        }
    } else {
        candidate_index.insert(key, candidates.len());
        candidates.push(candidate);
    }
}

fn pack_context_items(
    candidates: &[SourceCandidate],
    char_budget: usize,
) -> (Vec<DxMetasearchContextItem>, String, usize) {
    let mut items = Vec::new();
    let mut context_sections = Vec::new();
    let mut used_chars = 0usize;

    for candidate in candidates {
        let metadata_chars = source_metadata_chars(candidate);
        if used_chars + metadata_chars + MIN_SOURCE_CONTEXT_CHARS > char_budget && !items.is_empty()
        {
            break;
        }

        let remaining_chars = char_budget.saturating_sub(used_chars + metadata_chars);
        if remaining_chars == 0 {
            break;
        }
        let text_budget = remaining_chars.min(candidate.text.chars().count());
        let text = truncate_for_char_budget(&compact_text(&candidate.text), text_budget);
        let truncated = candidate.text.chars().count() > text.chars().count();
        let item_chars = metadata_chars + text.chars().count();
        if used_chars + item_chars > char_budget && !items.is_empty() {
            break;
        }

        let item = DxMetasearchContextItem {
            source_id: candidate.source_id.clone(),
            title: compact_text(&candidate.title),
            url: candidate.url.clone(),
            engine: candidate.engine.clone(),
            category: candidate.category.clone(),
            content_kind: candidate.content_kind.clone(),
            source_kind: candidate.source_kind,
            estimated_chars: item_chars,
            estimated_tokens: estimate_token_count(item_chars),
            truncated,
            text,
        };
        used_chars += item.estimated_chars;
        context_sections.push(render_context_section(&item));
        items.push(item);
    }

    (items, context_sections.join("\n\n"), used_chars)
}

fn render_context_section(item: &DxMetasearchContextItem) -> String {
    let mut lines = vec![
        format!("[{}] {}", item.source_id, item.title),
        format!("url: {}", item.url),
    ];
    if let Some(engine) = &item.engine {
        lines.push(format!("engine: {engine}"));
    }
    if let Some(category) = &item.category {
        lines.push(format!("category: {category}"));
    }
    if let Some(content_kind) = &item.content_kind {
        lines.push(format!("content_kind: {content_kind}"));
    }
    lines.push(format!("source_kind: {}", item.source_kind));
    lines.push(format!("text: {}", item.text));
    lines.join("\n")
}

fn source_metadata_chars(candidate: &SourceCandidate) -> usize {
    candidate.source_id.chars().count()
        + candidate.title.chars().count()
        + candidate.url.chars().count()
        + candidate
            .engine
            .as_ref()
            .map_or(0, |engine| engine.chars().count())
        + candidate
            .category
            .as_ref()
            .map_or(0, |category| category.chars().count())
        + 64
}

fn locate_schema_object<'a>(
    value: &'a Value,
    schema: &str,
    nested_field: &str,
) -> Option<&'a Value> {
    if value.get("schema").and_then(Value::as_str) == Some(schema) {
        return Some(value);
    }

    value
        .get(nested_field)
        .filter(|nested| nested.get("schema").and_then(Value::as_str) == Some(schema))
        .or_else(|| {
            value
                .get("source_pack")
                .filter(|nested| nested.get("schema").and_then(Value::as_str) == Some(schema))
        })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text)
                .map_err(|error| format!("Failed to parse stringified DX context JSON: {error}"));
        }
    }

    Ok(value.clone())
}

fn external_root(
    env_var: &str,
    default_root: &str,
    integration_path: &[&str],
) -> DxMetasearchContextExternalRoot {
    let root = env::var_os(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_root));
    let integration_file = integration_path
        .iter()
        .fold(root.clone(), |path, segment| path.join(*segment));

    DxMetasearchContextExternalRoot {
        root: root.display().to_string(),
        root_exists: root.exists(),
        integration_file: integration_file
            .is_file()
            .then(|| integration_file.display().to_string()),
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn clean_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| truncate_for_char_budget(&compact_text(&value), max_chars))
        .filter(|value| !value.is_empty())
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_for_char_budget(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let suffix = "...";
    if max_chars <= suffix.len() {
        return text.chars().take(max_chars).collect();
    }

    let mut truncated = text
        .chars()
        .take(max_chars - suffix.len())
        .collect::<String>();
    truncated.push_str(suffix);
    truncated
}

fn estimate_token_count(chars: usize) -> usize {
    (chars + APPROX_CHARS_PER_TOKEN - 1) / APPROX_CHARS_PER_TOKEN
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
