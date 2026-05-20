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
const DEFAULT_ENGINE_STATUS_LIMIT: usize = 40;
const MAX_ENGINE_STATUS_LIMIT: usize = 200;
const SOURCE_PACK_TOTAL_CHAR_BUDGET: usize = 2600;
const SOURCE_PACK_TITLE_CHAR_LIMIT: usize = 120;
const SOURCE_PACK_EXCERPT_CHAR_LIMIT: usize = 280;
const SOURCE_PACK_MIN_EXCERPT_CHAR_LIMIT: usize = 96;
const SOURCE_PACK_APPROX_CHARS_PER_TOKEN: usize = 4;
const DEFAULT_SOURCE_EXTRACT_CHAR_LIMIT: usize = 4_000;
const MIN_SOURCE_EXTRACT_CHAR_LIMIT: usize = 500;
const MAX_SOURCE_EXTRACT_CHAR_LIMIT: usize = 12_000;
const MAX_SOURCE_EXTRACT_FETCH_BYTES: usize = 1_500_000;

pub(crate) const DX_METASEARCH_RESULT_SCHEMA: &str = "zed.dx.metasearch.result.v1";
pub(crate) const DX_METASEARCH_STATUS_SCHEMA: &str = "zed.dx.metasearch.status.v1";
pub(crate) const DX_METASEARCH_SOURCE_PACK_SCHEMA: &str = "zed.dx.metasearch.source_pack.v1";
pub(crate) const DX_METASEARCH_SOURCE_PACK_RECEIPT_SCHEMA: &str =
    "zed.dx.metasearch.source_pack_receipt.v1";
pub(crate) const DX_METASEARCH_SOURCE_EXTRACT_SCHEMA: &str = "zed.dx.metasearch.source_extract.v1";

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

#[derive(Clone, Debug)]
pub(crate) struct DxMetasearchStatusRequest {
    pub base_url: Option<String>,
    pub include_engines: bool,
    pub engine_limit: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct DxMetasearchSourceExtractRequest {
    pub url: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub engine: Option<String>,
    pub category: Option<String>,
    pub max_chars: Option<usize>,
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
    pub source_pack: DxMetasearchSourcePack,
    pub source_pack_receipt: Option<DxMetasearchSourcePackReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchStatusResponse {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub source: DxMetasearchSource,
    pub request: DxMetasearchStatusRequestSummary,
    pub service: DxMetasearchServiceStatus,
    pub engine_summary: DxMetasearchEngineSummary,
    pub engines: Vec<DxMetasearchEngineInfo>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourceExtractResponse {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub source: DxMetasearchSource,
    pub request: DxMetasearchSourceExtractRequestSummary,
    pub fetch: DxMetasearchSourceExtractFetch,
    pub content: DxMetasearchSourceExtractContent,
    pub compression: DxMetasearchSourceExtractCompression,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourceExtractRequestSummary {
    pub url: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub engine: Option<String>,
    pub category: Option<String>,
    pub max_chars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourceExtractFetch {
    pub status_code: u16,
    pub content_type: Option<String>,
    pub fetched_bytes: usize,
    pub body_truncated: bool,
    pub max_fetch_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourceExtractContent {
    pub kind: &'static str,
    pub extracted_chars: usize,
    pub estimated_tokens: usize,
    pub output_char_limit: usize,
    pub output_truncated: bool,
    pub text: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourceExtractCompression {
    pub profile: &'static str,
    pub machine_format: &'static str,
    pub approx_chars_per_token: usize,
    pub serializer_ready: bool,
    pub rlm_ready: bool,
    pub loss_policy: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchStatusRequestSummary {
    pub base_url: String,
    pub status_endpoint: String,
    pub engines_endpoint: String,
    pub include_engines: bool,
    pub engine_limit: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchServiceStatus {
    pub status: String,
    pub version: Option<String>,
    pub engine_count: usize,
    pub tracked_engine_count: usize,
    pub unhealthy_engine_count: usize,
    pub unhealthy_engines: Vec<String>,
    pub warning_count: usize,
    pub warnings: Vec<String>,
    pub asset_warning_count: usize,
    pub runtime: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchEngineSummary {
    pub catalog_count: usize,
    pub returned_count: usize,
    pub truncated_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub category_count: usize,
    pub categories: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchEngineInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub homepage: Option<String>,
    pub categories: Vec<String>,
    pub enabled: bool,
    pub timeout_ms: Option<u64>,
    pub weight: Option<f64>,
    pub health_status: Option<String>,
    pub failure_count: Option<u64>,
    pub last_error: Option<String>,
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

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourcePack {
    pub schema: &'static str,
    pub query: String,
    pub mode: &'static str,
    pub item_count: usize,
    pub omitted_result_count: usize,
    pub estimated_chars: usize,
    pub estimated_tokens: usize,
    pub char_budget: usize,
    pub token_budget: usize,
    pub budget_exceeded: bool,
    pub compression: DxMetasearchSourcePackCompression,
    pub handoff: DxMetasearchSourcePackHandoff,
    pub items: Vec<DxMetasearchSourcePackItem>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourcePackCompression {
    pub profile: &'static str,
    pub title_char_limit: usize,
    pub excerpt_char_limit: usize,
    pub approx_chars_per_token: usize,
    pub serializer_ready: bool,
    pub rlm_ready: bool,
    pub loss_policy: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourcePackHandoff {
    pub source_id_style: &'static str,
    pub citation_style: &'static str,
    pub machine_format: &'static str,
    pub recommended_next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourcePackItem {
    pub source_id: String,
    pub citation_id: usize,
    pub title: String,
    pub url: String,
    pub engine: String,
    pub category: String,
    pub score: f64,
    pub excerpt: String,
    pub published_date: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMetasearchSourcePackReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub source_pack_schema: &'static str,
    pub item_count: usize,
    pub estimated_tokens: usize,
    pub next_action: String,
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

pub(crate) async fn inspect_metasearch_status(
    http_client: Arc<HttpClientWithUrl>,
    request: DxMetasearchStatusRequest,
) -> Result<DxMetasearchStatusResponse, String> {
    let base_url = resolve_base_url(request.base_url);
    let status_endpoint = build_endpoint(&base_url, "/api/v1/status")?;
    let engines_endpoint = build_endpoint(&base_url, "/api/v1/engines")?;
    let engine_limit = request
        .engine_limit
        .unwrap_or(DEFAULT_ENGINE_STATUS_LIMIT)
        .clamp(1, MAX_ENGINE_STATUS_LIMIT);
    let status_json = fetch_json(http_client.clone(), &status_endpoint).await?;
    let engines_json = if request.include_engines {
        Some(fetch_json(http_client, &engines_endpoint).await?)
    } else {
        None
    };

    Ok(compact_status_response(
        base_url,
        status_endpoint,
        engines_endpoint,
        request.include_engines,
        engine_limit,
        status_json,
        engines_json,
    ))
}

pub(crate) async fn extract_metasearch_source(
    http_client: Arc<HttpClientWithUrl>,
    request: DxMetasearchSourceExtractRequest,
) -> Result<DxMetasearchSourceExtractResponse, String> {
    let request = normalize_source_extract_request(request)?;
    let mut response = http_client
        .get(&request.url, AsyncBody::default(), true)
        .await
        .map_err(|error| {
            format!(
                "DX metasearch source extract failed for {}. Is the source reachable? {error}",
                request.url
            )
        })?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let status = response.status();
    let mut body = Vec::new();
    response
        .body_mut()
        .read_to_end(&mut body)
        .await
        .map_err(|error| format!("Failed to read DX metasearch source body: {error}"))?;

    if status.as_u16() >= 400 {
        let text = String::from_utf8_lossy(&body);
        return Err(format!(
            "DX metasearch source extract returned HTTP {} for {}: {}",
            status.as_u16(),
            request.url,
            truncate_text(&text, SNIPPET_CHAR_LIMIT)
        ));
    }

    let fetched_bytes = body.len();
    let body_truncated = fetched_bytes > MAX_SOURCE_EXTRACT_FETCH_BYTES;
    if body_truncated {
        body.truncate(MAX_SOURCE_EXTRACT_FETCH_BYTES);
    }

    Ok(compact_source_extract_response(
        request,
        status.as_u16(),
        content_type,
        fetched_bytes,
        body_truncated,
        body,
    ))
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
    let base_url = resolve_base_url(request.base_url);
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

fn normalize_source_extract_request(
    request: DxMetasearchSourceExtractRequest,
) -> Result<DxMetasearchSourceExtractRequestSummary, String> {
    let url = request.url.trim();
    if url.is_empty() {
        return Err("DX metasearch source URL is required.".to_string());
    }
    let parsed_url = Url::parse(url)
        .map_err(|error| format!("Invalid DX metasearch source URL `{url}`: {error}"))?;
    if !matches!(parsed_url.scheme(), "http" | "https") {
        return Err(format!(
            "DX metasearch source URL must use http or https, got `{}`.",
            parsed_url.scheme()
        ));
    }

    Ok(DxMetasearchSourceExtractRequestSummary {
        url: parsed_url.to_string(),
        source_id: clean_optional_text(request.source_id, SOURCE_PACK_TITLE_CHAR_LIMIT),
        title: clean_optional_text(request.title, SOURCE_PACK_TITLE_CHAR_LIMIT),
        engine: clean_optional_text(request.engine, SOURCE_PACK_TITLE_CHAR_LIMIT),
        category: clean_optional_text(request.category, SOURCE_PACK_TITLE_CHAR_LIMIT),
        max_chars: request
            .max_chars
            .unwrap_or(DEFAULT_SOURCE_EXTRACT_CHAR_LIMIT)
            .clamp(MIN_SOURCE_EXTRACT_CHAR_LIMIT, MAX_SOURCE_EXTRACT_CHAR_LIMIT),
    })
}

async fn fetch_json(
    http_client: Arc<HttpClientWithUrl>,
    endpoint: &str,
) -> Result<serde_json::Value, String> {
    let mut response = http_client
        .get(endpoint, AsyncBody::default(), true)
        .await
        .map_err(|error| {
            format!(
                "DX metasearch request failed for {endpoint}. Is the metasearch service running? {error}"
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
            "DX metasearch returned HTTP {} for {endpoint}: {}",
            status.as_u16(),
            truncate_text(&text, SNIPPET_CHAR_LIMIT)
        ));
    }

    serde_json::from_slice::<serde_json::Value>(&body)
        .map_err(|error| format!("Failed to parse DX metasearch JSON response: {error}"))
}

fn resolve_base_url(base_url: Option<String>) -> String {
    base_url
        .filter(|base_url| !base_url.trim().is_empty())
        .or_else(|| env::var(DX_METASEARCH_BASE_URL_ENV).ok())
        .unwrap_or_else(|| DEFAULT_METASEARCH_BASE_URL.to_string())
}

fn build_endpoint(base_url: &str, path: &str) -> Result<String, String> {
    let path = path.trim_start_matches('/');
    Url::parse(&format!("{}/{}", base_url.trim_end_matches('/'), path))
        .map(|url| url.to_string())
        .map_err(|error| format!("Invalid DX metasearch base URL `{base_url}`: {error}"))
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

fn compact_status_response(
    base_url: String,
    status_endpoint: String,
    engines_endpoint: String,
    include_engines: bool,
    engine_limit: usize,
    status_json: serde_json::Value,
    engines_json: Option<serde_json::Value>,
) -> DxMetasearchStatusResponse {
    let service_status =
        string_field(&status_json, "status").unwrap_or_else(|| "unknown".to_string());
    let engine_count = usize_field(&status_json, "engine_count");
    let tracked_engine_count = usize_field(&status_json, "tracked_engines")
        .or_else(|| usize_field(&status_json, "tracked_engine_count"))
        .unwrap_or_default();
    let unhealthy_engines = string_array_field(&status_json, "unhealthy_engines");
    let unhealthy_engine_count =
        usize_field(&status_json, "unhealthy_engine_count").unwrap_or(unhealthy_engines.len());
    let warnings = string_array_field(&status_json, "warnings");
    let warning_count = usize_field(&status_json, "warning_count").unwrap_or(warnings.len());
    let asset_warning_count = usize_field(&status_json, "asset_warning_count").unwrap_or_default();
    let runtime = status_json
        .get("runtime")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let all_engines = engines_json
        .as_ref()
        .and_then(|json| json.get("engines"))
        .and_then(|engines| engines.as_array())
        .map(|engines| engines.iter().map(compact_engine_info).collect::<Vec<_>>())
        .unwrap_or_default();
    let catalog_count = engines_json
        .as_ref()
        .and_then(|json| usize_field(json, "count"))
        .or(engine_count)
        .unwrap_or(all_engines.len());
    let enabled_count = all_engines.iter().filter(|engine| engine.enabled).count();
    let disabled_count = all_engines.len().saturating_sub(enabled_count);
    let mut categories = all_engines
        .iter()
        .flat_map(|engine| engine.categories.iter().cloned())
        .collect::<Vec<_>>();
    categories.sort();
    categories.dedup();
    let returned_engines = all_engines
        .into_iter()
        .take(engine_limit)
        .collect::<Vec<_>>();
    let next_action = metasearch_status_next_action(
        &service_status,
        include_engines,
        catalog_count,
        unhealthy_engine_count,
        warning_count,
    );

    DxMetasearchStatusResponse {
        schema: DX_METASEARCH_STATUS_SCHEMA,
        generated_at_ms: current_unix_ms(),
        source: metasearch_source(),
        request: DxMetasearchStatusRequestSummary {
            base_url,
            status_endpoint,
            engines_endpoint,
            include_engines,
            engine_limit,
        },
        service: DxMetasearchServiceStatus {
            status: service_status,
            version: string_field(&status_json, "version"),
            engine_count: engine_count.unwrap_or(catalog_count),
            tracked_engine_count,
            unhealthy_engine_count,
            unhealthy_engines,
            warning_count,
            warnings,
            asset_warning_count,
            runtime,
        },
        engine_summary: DxMetasearchEngineSummary {
            catalog_count,
            returned_count: returned_engines.len(),
            truncated_count: catalog_count.saturating_sub(returned_engines.len()),
            enabled_count,
            disabled_count,
            category_count: categories.len(),
            categories,
        },
        engines: returned_engines,
        next_action,
    }
}

fn compact_source_extract_response(
    request: DxMetasearchSourceExtractRequestSummary,
    status_code: u16,
    content_type: Option<String>,
    fetched_bytes: usize,
    body_truncated: bool,
    body: Vec<u8>,
) -> DxMetasearchSourceExtractResponse {
    let raw_text = String::from_utf8_lossy(&body);
    let content_kind = source_extract_kind(content_type.as_deref(), &raw_text);
    let mut warnings = Vec::new();
    if body_truncated {
        warnings.push(format!(
            "Fetched body exceeded {} bytes and was truncated before text extraction.",
            MAX_SOURCE_EXTRACT_FETCH_BYTES
        ));
    }

    let readable_text = match content_kind {
        "html" => html_to_compact_text(&raw_text),
        "json" | "xml" | "text" | "unknown_text" => compact_source_pack_text(&raw_text, usize::MAX),
        _ => compact_source_pack_text(&raw_text, usize::MAX),
    };
    let output_truncated = readable_text.chars().count() > request.max_chars;
    if output_truncated {
        warnings.push(format!(
            "Extracted text exceeded {} characters and was truncated for Agent context.",
            request.max_chars
        ));
    }
    if readable_text.trim().is_empty() {
        warnings.push("No readable text was extracted from the fetched source.".to_string());
    }

    let output_char_limit = request.max_chars;
    let text = truncate_source_extract_text(&readable_text, output_char_limit);
    let extracted_chars = text.chars().count();
    let next_action = if extracted_chars == 0 {
        "Try a different source-pack item, fetch the page through the browser preview, or inspect the metasearch result snippet only.".to_string()
    } else if output_truncated || body_truncated {
        "Use this bounded extract for immediate Agent context, then pass the same source metadata to serializer/RLM compaction before expanding more text.".to_string()
    } else {
        "Use content.text as cited Agent context now, and pass this schema to serializer/RLM adapters when that lane is enabled.".to_string()
    };

    DxMetasearchSourceExtractResponse {
        schema: DX_METASEARCH_SOURCE_EXTRACT_SCHEMA,
        generated_at_ms: current_unix_ms(),
        source: metasearch_source(),
        request,
        fetch: DxMetasearchSourceExtractFetch {
            status_code,
            content_type,
            fetched_bytes,
            body_truncated,
            max_fetch_bytes: MAX_SOURCE_EXTRACT_FETCH_BYTES,
        },
        content: DxMetasearchSourceExtractContent {
            kind: content_kind,
            extracted_chars,
            estimated_tokens: estimate_token_count(extracted_chars),
            output_char_limit,
            output_truncated,
            text,
            warnings,
        },
        compression: DxMetasearchSourceExtractCompression {
            profile: "dx_metasearch.deep_source_extract.compact.v1",
            machine_format: "json:zed.dx.metasearch.source_extract.v1",
            approx_chars_per_token: SOURCE_PACK_APPROX_CHARS_PER_TOKEN,
            serializer_ready: true,
            rlm_ready: true,
            loss_policy: "preserve URL, source ID, title, engine, category, content type, fetch status, and bounded readable text; truncate before expanding to full page text",
        },
        next_action,
    }
}

fn compact_engine_info(engine: &serde_json::Value) -> DxMetasearchEngineInfo {
    let health = engine.get("health").unwrap_or(&serde_json::Value::Null);
    DxMetasearchEngineInfo {
        name: string_field(engine, "name").unwrap_or_else(|| "unknown".to_string()),
        display_name: string_field(engine, "display_name"),
        homepage: string_field(engine, "homepage"),
        categories: string_array_field(engine, "categories"),
        enabled: bool_field(engine, "enabled").unwrap_or(true),
        timeout_ms: u64_field(engine, "timeout_ms"),
        weight: f64_field(engine, "weight"),
        health_status: string_field(health, "status"),
        failure_count: u64_field(health, "failure_count"),
        last_error: string_field(health, "last_error"),
    }
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
    let source_pack = build_source_pack(&response.query, &returned_results);
    let next_action = if returned_results.is_empty() {
        "No compact citations returned. Try a broader category set or inspect metasearch engine health.".to_string()
    } else if !response.engines_failed.is_empty() {
        "Use the returned citations, then inspect failed engines before relying on this as exhaustive.".to_string()
    } else {
        "Use source_pack.items for token-aware cited Agent context, then hand the same schema to serializer/RLM compaction when that lane is enabled.".to_string()
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
        source_pack,
        source_pack_receipt: None,
        next_action,
    }
}

fn build_source_pack(query: &str, results: &[DxMetasearchCompactResult]) -> DxMetasearchSourcePack {
    let mut items = Vec::new();
    let mut estimated_chars = 0usize;

    for result in results {
        let title = compact_source_pack_text(&result.title, SOURCE_PACK_TITLE_CHAR_LIMIT);
        let mut excerpt = compact_source_pack_text(&result.snippet, SOURCE_PACK_EXCERPT_CHAR_LIMIT);
        let source_id = format!("S{}", result.citation_id);
        let mut candidate_chars = source_pack_item_chars(
            &source_id,
            &title,
            &result.url,
            &result.engine,
            &result.category,
            &excerpt,
        );

        if estimated_chars + candidate_chars > SOURCE_PACK_TOTAL_CHAR_BUDGET {
            excerpt = compact_source_pack_text(&result.snippet, SOURCE_PACK_MIN_EXCERPT_CHAR_LIMIT);
            candidate_chars = source_pack_item_chars(
                &source_id,
                &title,
                &result.url,
                &result.engine,
                &result.category,
                &excerpt,
            );
        }

        if !items.is_empty() && estimated_chars + candidate_chars > SOURCE_PACK_TOTAL_CHAR_BUDGET {
            break;
        }

        estimated_chars += candidate_chars;
        items.push(DxMetasearchSourcePackItem {
            source_id,
            citation_id: result.citation_id,
            title,
            url: result.url.clone(),
            engine: result.engine.clone(),
            category: result.category.clone(),
            score: result.score,
            excerpt,
            published_date: result.published_date.clone(),
        });
    }

    let item_count = items.len();
    let omitted_result_count = results.len().saturating_sub(item_count);
    let estimated_tokens = estimate_token_count(estimated_chars);
    let token_budget = estimate_token_count(SOURCE_PACK_TOTAL_CHAR_BUDGET);
    let recommended_next_action = if item_count == 0 {
        "No source-pack items are ready; broaden the search or inspect metasearch engine health."
            .to_string()
    } else if omitted_result_count > 0 {
        format!(
            "Use the {} compact source-pack item(s), then rerun with narrower engines/categories if omitted sources matter.",
            item_count
        )
    } else {
        "Use these source-pack items directly as cited Agent context, or pass them to serializer/RLM compaction for the next call."
            .to_string()
    };

    DxMetasearchSourcePack {
        schema: DX_METASEARCH_SOURCE_PACK_SCHEMA,
        query: compact_source_pack_text(query, SOURCE_PACK_TITLE_CHAR_LIMIT),
        mode: "token_aware_cited_handoff",
        item_count,
        omitted_result_count,
        estimated_chars,
        estimated_tokens,
        char_budget: SOURCE_PACK_TOTAL_CHAR_BUDGET,
        token_budget,
        budget_exceeded: omitted_result_count > 0
            || estimated_chars > SOURCE_PACK_TOTAL_CHAR_BUDGET,
        compression: DxMetasearchSourcePackCompression {
            profile: "dx_metasearch.cited_source_pack.compact.v1",
            title_char_limit: SOURCE_PACK_TITLE_CHAR_LIMIT,
            excerpt_char_limit: SOURCE_PACK_EXCERPT_CHAR_LIMIT,
            approx_chars_per_token: SOURCE_PACK_APPROX_CHARS_PER_TOKEN,
            serializer_ready: true,
            rlm_ready: true,
            loss_policy: "preserve source id, url, engine, category, score, date, and one compact excerpt; omit overflow results before expanding snippets",
        },
        handoff: DxMetasearchSourcePackHandoff {
            source_id_style: "S{citation_id}",
            citation_style: "cite as [S1], [S2], ... and preserve URLs in final attribution when needed",
            machine_format: "json:zed.dx.metasearch.source_pack.v1",
            recommended_next_action,
        },
        items,
    }
}

fn source_pack_item_chars(
    source_id: &str,
    title: &str,
    url: &str,
    engine: &str,
    category: &str,
    excerpt: &str,
) -> usize {
    source_id.chars().count()
        + title.chars().count()
        + url.chars().count()
        + engine.chars().count()
        + category.chars().count()
        + excerpt.chars().count()
        + 24
}

fn metasearch_status_next_action(
    status: &str,
    include_engines: bool,
    catalog_count: usize,
    unhealthy_engine_count: usize,
    warning_count: usize,
) -> String {
    if !include_engines {
        return "Run again with include_engines=true before choosing exact engine filters for Agent searches."
            .to_string();
    }

    if catalog_count == 0 {
        return "Start or repair the DX metasearch service before routing Agent searches through it."
            .to_string();
    }

    if status != "ok" || unhealthy_engine_count > 0 || warning_count > 0 {
        return "Review warnings and unhealthy engines, then prefer healthy exact-engine filters for critical Agent searches."
            .to_string();
    }

    "Metasearch service is ready for cited Agent searches; choose categories or exact engines for focused source packs."
        .to_string()
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

fn string_field(json: &serde_json::Value, field: &str) -> Option<String> {
    json.get(field)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn string_array_field(json: &serde_json::Value, field: &str) -> Vec<String> {
    json.get(field)
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn usize_field(json: &serde_json::Value, field: &str) -> Option<usize> {
    u64_field(json, field).and_then(|value| usize::try_from(value).ok())
}

fn u64_field(json: &serde_json::Value, field: &str) -> Option<u64> {
    json.get(field).and_then(|value| value.as_u64())
}

fn f64_field(json: &serde_json::Value, field: &str) -> Option<f64> {
    json.get(field).and_then(|value| value.as_f64())
}

fn bool_field(json: &serde_json::Value, field: &str) -> Option<bool> {
    json.get(field).and_then(|value| value.as_bool())
}

fn clean_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| compact_source_pack_text(&value, max_chars))
        .filter(|value| !value.is_empty())
}

fn source_extract_kind(content_type: Option<&str>, text: &str) -> &'static str {
    let content_type = content_type
        .map(|content_type| content_type.to_ascii_lowercase())
        .unwrap_or_default();

    if content_type.contains("html") || text.trim_start().starts_with('<') {
        "html"
    } else if content_type.contains("json") {
        "json"
    } else if content_type.contains("xml") {
        "xml"
    } else if content_type.starts_with("text/") {
        "text"
    } else if content_type.is_empty() {
        "unknown_text"
    } else {
        "unknown_binary_or_text"
    }
}

fn html_to_compact_text(html: &str) -> String {
    let without_blocks = strip_html_blocks(html, &["script", "style", "noscript", "svg", "head"]);
    let mut text = String::with_capacity(without_blocks.len().min(32_000));
    let mut in_tag = false;

    for character in without_blocks.chars() {
        match character {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }

    compact_source_pack_text(&decode_basic_html_entities(&text), usize::MAX)
}

fn strip_html_blocks(input: &str, tags: &[&str]) -> String {
    let mut output = input.to_string();

    for tag in tags {
        let open_tag = format!("<{tag}");
        let close_tag = format!("</{tag}>");
        loop {
            let lower = output.to_ascii_lowercase();
            let Some(start) = lower.find(&open_tag) else {
                break;
            };
            let after_open = lower[start..]
                .find('>')
                .map(|offset| start + offset + 1)
                .unwrap_or(output.len());
            let end = lower[after_open..]
                .find(&close_tag)
                .map(|offset| after_open + offset + close_tag.len())
                .unwrap_or(after_open);
            output.replace_range(start..end, " ");
        }
    }

    output
}

fn decode_basic_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
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

fn truncate_source_extract_text(text: &str, max_chars: usize) -> String {
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

fn compact_source_pack_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_text(&normalized, max_chars)
}

fn estimate_token_count(chars: usize) -> usize {
    (chars + SOURCE_PACK_APPROX_CHARS_PER_TOKEN - 1) / SOURCE_PACK_APPROX_CHARS_PER_TOKEN
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
