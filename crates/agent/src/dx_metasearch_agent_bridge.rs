use futures::AsyncReadExt as _;
use http_client::{AsyncBody, HttpClientWithUrl};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

const DX_METASEARCH_BASE_URL_ENV: &str = "DX_METASEARCH_BASE_URL";
const DX_METASEARCH_ROOT_ENV: &str = "DX_METASEARCH_ROOT";
const DEFAULT_METASEARCH_BASE_URL: &str = "http://127.0.0.1:8888";
const DEFAULT_MAX_RESULTS: usize = 8;
const MAX_RESULTS_LIMIT: usize = 20;
const MAX_CATEGORY_COUNT: usize = 6;
const MAX_ENGINE_COUNT: usize = 24;
const SNIPPET_CHAR_LIMIT: usize = 420;

pub(crate) const DX_METASEARCH_RESULT_SCHEMA: &str = "zed.dx.metasearch.result.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxMetasearchRequest {
    pub query: String,
    pub categories: Vec<String>,
    pub engines: Vec<String>,
    pub language: Option<String>,
    pub safe_search: Option<u8>,
    pub page: Option<u32>,
    pub time_range: Option<String>,
    pub max_results: Option<usize>,
    pub base_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchCompactResponse {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub query: String,
    pub request: DxMetasearchCompactRequest,
    pub source: DxMetasearchSource,
    pub summary: DxMetasearchSummary,
    pub results: Vec<DxMetasearchCompactResult>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchCompactRequest {
    pub base_url: String,
    pub endpoint: String,
    pub categories: Vec<String>,
    pub engines: Vec<String>,
    pub language: Option<String>,
    pub safe_search: u8,
    pub page: u32,
    pub time_range: Option<String>,
    pub max_results: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSource {
    pub root: Option<String>,
    pub root_exists: bool,
    pub integration_guide: Option<String>,
    pub mode: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSummary {
    pub returned_result_count: usize,
    pub reported_result_count: usize,
    pub truncated_result_count: usize,
    pub engines_used: Vec<String>,
    pub engines_failed: Vec<String>,
    pub cached: bool,
    pub search_time_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchCompactResult {
    pub citation_id: usize,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engine: String,
    pub engine_rank: u32,
    pub score: f64,
    pub category: String,
    pub thumbnail: Option<String>,
    pub published_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetasearchApiResponse {
    query: String,
    results: Vec<MetasearchApiResult>,
    number_of_results: usize,
    engines_used: Vec<String>,
    engines_failed: Vec<String>,
    search_time_ms: u64,
    cached: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MetasearchApiResult {
    title: String,
    url: String,
    content: String,
    engine: String,
    engine_rank: u32,
    score: f64,
    thumbnail: Option<String>,
    published_date: Option<String>,
    #[serde(default)]
    category: String,
}

pub(crate) async fn search_metasearch(
    http_client: Arc<HttpClientWithUrl>,
    request: DxMetasearchRequest,
) -> Result<DxMetasearchCompactResponse, String> {
    let normalized = normalize_request(request)?;
    let mut response = http_client
        .get(&normalized.endpoint, AsyncBody::default(), true)
        .await
        .map_err(|error| {
            format!(
                "DX metasearch request failed for {}. Is the metasearch service running? {error}",
                normalized.base_url
            )
        })?;

    let mut body = Vec::new();
    response
        .body_mut()
        .read_to_end(&mut body)
        .await
        .map_err(|error| format!("Failed to read DX metasearch response body: {error}"))?;

    let status = response.status();
    if status.as_u16() >= 400 {
        let text = String::from_utf8_lossy(&body);
        return Err(format!(
            "DX metasearch returned HTTP {}: {}",
            status.as_u16(),
            truncate_text(&text, SNIPPET_CHAR_LIMIT)
        ));
    }

    let api_response = serde_json::from_slice::<MetasearchApiResponse>(&body)
        .map_err(|error| format!("Failed to parse DX metasearch JSON response: {error}"))?;
    Ok(compact_response(api_response, normalized))
}

fn normalize_request(request: DxMetasearchRequest) -> Result<DxMetasearchCompactRequest, String> {
    let query = request.query.trim();
    if query.is_empty() {
        return Err("DX metasearch query is required.".to_string());
    }

    let categories = clean_list(request.categories, MAX_CATEGORY_COUNT);
    let engines = clean_list(request.engines, MAX_ENGINE_COUNT);
    let language = request
        .language
        .map(|language| language.trim().to_ascii_lowercase())
        .filter(|language| !language.is_empty());
    let time_range = request
        .time_range
        .map(|time_range| time_range.trim().to_ascii_lowercase())
        .filter(|time_range| !time_range.is_empty());
    let safe_search = request.safe_search.unwrap_or(1).min(2);
    let page = request.page.unwrap_or(1).max(1);
    let max_results = request
        .max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .clamp(1, MAX_RESULTS_LIMIT);
    let base_url = request
        .base_url
        .filter(|base_url| !base_url.trim().is_empty())
        .or_else(|| env::var(DX_METASEARCH_BASE_URL_ENV).ok())
        .unwrap_or_else(|| DEFAULT_METASEARCH_BASE_URL.to_string());
    let endpoint = build_search_endpoint(
        &base_url,
        query,
        &categories,
        &engines,
        language.as_deref(),
        safe_search,
        page,
        time_range.as_deref(),
    )?;

    Ok(DxMetasearchCompactRequest {
        base_url,
        endpoint,
        categories,
        engines,
        language,
        safe_search,
        page,
        time_range,
        max_results,
    })
}

fn build_search_endpoint(
    base_url: &str,
    query: &str,
    categories: &[String],
    engines: &[String],
    language: Option<&str>,
    safe_search: u8,
    page: u32,
    time_range: Option<&str>,
) -> Result<String, String> {
    let mut endpoint = Url::parse(&format!("{}/api/v1/search", base_url.trim_end_matches('/')))
        .map_err(|error| format!("Invalid DX metasearch base URL `{base_url}`: {error}"))?;
    {
        let mut pairs = endpoint.query_pairs_mut();
        pairs.append_pair("q", query);
        pairs.append_pair("format", "json");
        pairs.append_pair("safe_search", &safe_search.to_string());
        pairs.append_pair("page", &page.to_string());
        if !categories.is_empty() {
            pairs.append_pair("categories", &categories.join(","));
        }
        if !engines.is_empty() {
            pairs.append_pair("engines", &engines.join(","));
        }
        if let Some(language) = language {
            pairs.append_pair("language", language);
        }
        if let Some(time_range) = time_range {
            pairs.append_pair("time_range", time_range);
        }
    }

    Ok(endpoint.to_string())
}

fn compact_response(
    response: MetasearchApiResponse,
    request: DxMetasearchCompactRequest,
) -> DxMetasearchCompactResponse {
    let source = metasearch_source();
    let reported_result_count = response.number_of_results;
    let returned_results = response
        .results
        .into_iter()
        .take(request.max_results)
        .enumerate()
        .map(|(index, result)| DxMetasearchCompactResult {
            citation_id: index + 1,
            title: truncate_text(&result.title, 180),
            url: result.url,
            snippet: truncate_text(&result.content, SNIPPET_CHAR_LIMIT),
            engine: result.engine,
            engine_rank: result.engine_rank,
            score: result.score,
            category: if result.category.trim().is_empty() {
                "general".to_string()
            } else {
                result.category
            },
            thumbnail: result.thumbnail,
            published_date: result.published_date,
        })
        .collect::<Vec<_>>();
    let truncated_result_count = reported_result_count.saturating_sub(returned_results.len());
    let next_action = if returned_results.is_empty() {
        "No compact citations returned. Try a broader category set or inspect metasearch engine health.".to_string()
    } else if !response.engines_failed.is_empty() {
        "Use the returned citations, then inspect failed engines before relying on this as exhaustive.".to_string()
    } else {
        "Use the numbered citations directly in the Agent answer or source pack.".to_string()
    };

    DxMetasearchCompactResponse {
        schema: DX_METASEARCH_RESULT_SCHEMA,
        generated_at_ms: current_unix_ms(),
        query: response.query,
        request,
        source,
        summary: DxMetasearchSummary {
            returned_result_count: returned_results.len(),
            reported_result_count,
            truncated_result_count,
            engines_used: response.engines_used,
            engines_failed: response.engines_failed,
            cached: response.cached.unwrap_or(false),
            search_time_ms: response.search_time_ms,
        },
        results: returned_results,
        next_action,
    }
}

fn metasearch_source() -> DxMetasearchSource {
    let root = env::var_os(DX_METASEARCH_ROOT_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            let default_root = PathBuf::from(r"G:\Workspaces\flow\metasearch");
            default_root.exists().then_some(default_root)
        });
    let root_exists = root.as_ref().is_some_and(|root| root.exists());
    let integration_guide = root
        .as_ref()
        .map(|root| root.join("INTEGRATION_GUIDE.md"))
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string());

    DxMetasearchSource {
        root: root.map(|root| root.display().to_string()),
        root_exists,
        integration_guide,
        mode: "http_api",
    }
}

fn clean_list(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut cleaned = Vec::new();

    for value in values {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }

        cleaned.push(value);
        if cleaned.len() >= limit {
            break;
        }
    }

    cleaned
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
