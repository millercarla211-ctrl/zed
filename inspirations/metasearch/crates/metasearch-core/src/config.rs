//! Application settings.

use serde::{Deserialize, Serialize};

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub server: ServerSettings,
    pub search: SearchSettings,
    pub cache: CacheSettings,
    pub rate_limit: RateLimitSettings,
    pub bot_detection: BotDetectionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub templates_dir: String,
    pub static_dir: String,
    pub secret_key: String,
    pub image_proxy: bool,
    pub trust_forwarded_headers: bool,
    pub security_headers_enabled: bool,
    pub permissive_cors: bool,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    pub safe_search: u8,
    pub default_language: String,
    pub max_page: u32,
    pub request_timeout_ms: u64,
    pub max_concurrent_engines: usize,
    pub remote_autocomplete_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheSettings {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitSettings {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BotDetectionSettings {
    pub enabled: bool,
    pub block_missing_user_agent: bool,
    pub blocked_user_agent_keywords: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8888,
            base_url: "http://localhost:8888".to_string(),
            templates_dir: "templates".to_string(),
            static_dir: "static".to_string(),
            secret_key: "change-me-in-production".to_string(),
            image_proxy: true,
            trust_forwarded_headers: false,
            security_headers_enabled: true,
            permissive_cors: false,
            allowed_origins: Vec::new(),
        }
    }
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            safe_search: 1,
            default_language: "en".to_string(),
            max_page: 10,
            request_timeout_ms: 10_000,
            max_concurrent_engines: 50,
            remote_autocomplete_enabled: false,
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_secs: 300,
            max_entries: 10_000,
        }
    }
}

impl Default for RateLimitSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 10,
            burst_size: 30,
        }
    }
}

impl Default for BotDetectionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            block_missing_user_agent: false,
            blocked_user_agent_keywords: vec![
                "python-requests".to_string(),
                "scrapy".to_string(),
                "curl/".to_string(),
                "wget/".to_string(),
                "go-http-client".to_string(),
                "aiohttp".to_string(),
                "feedfetcher".to_string(),
                "crawler".to_string(),
                "spider".to_string(),
            ],
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings::default(),
            search: SearchSettings::default(),
            cache: CacheSettings::default(),
            rate_limit: RateLimitSettings::default(),
            bot_detection: BotDetectionSettings::default(),
        }
    }
}

impl ServerSettings {
    pub fn normalized_base_url(&self) -> String {
        self.base_url.trim().trim_end_matches('/').to_string()
    }

    pub fn runtime_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let base_url = self.base_url.trim();

        if base_url.is_empty() {
            warnings.push("server.base_url is empty; generated links and OpenSearch metadata may be incorrect.".to_string());
        } else if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            warnings.push("server.base_url should start with http:// or https://.".to_string());
        } else {
            let base_host = base_url_host(base_url);
            let external_browser_or_proxy_mode =
                self.permissive_cors || !self.allowed_origins.is_empty() || self.trust_forwarded_headers;

            if external_browser_or_proxy_mode && base_host.as_deref().is_some_and(is_local_host_label) {
                warnings.push("server.base_url still points to a local host while the deployment enables browser or proxy-facing settings.".to_string());
            }

            if external_browser_or_proxy_mode && base_url.starts_with("http://") {
                warnings.push("server.base_url uses http:// while the deployment enables browser or proxy-facing settings; production deployments usually want an https:// public URL.".to_string());
            }
        }

        if self.secret_key.trim().is_empty() {
            warnings.push("server.secret_key is empty.".to_string());
        } else if self.secret_key.trim() == "change-me-in-production" {
            warnings.push("server.secret_key still uses the default placeholder value.".to_string());
        }

        if self.templates_dir.trim().is_empty() {
            warnings.push("server.templates_dir is empty; HTML pages will not render correctly.".to_string());
        }

        if self.static_dir.trim().is_empty() {
            warnings.push("server.static_dir is empty; static assets will not be served correctly.".to_string());
        }

        if self.permissive_cors {
            warnings.push("server.permissive_cors is enabled; any browser origin can call the HTTP API.".to_string());
        }

        if self.permissive_cors && !self.allowed_origins.is_empty() {
            warnings.push("server.allowed_origins is ignored while permissive_cors is enabled.".to_string());
        }

        for origin in &self.allowed_origins {
            let trimmed = origin.trim();
            if trimmed.is_empty() {
                warnings.push("server.allowed_origins contains an empty origin entry.".to_string());
            } else if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
                warnings.push(format!(
                    "server.allowed_origins contains a non-HTTP origin: `{}`.",
                    trimmed
                ));
            }
        }

        if !self.security_headers_enabled {
            warnings.push("server.security_headers_enabled is disabled.".to_string());
        }

        if self.trust_forwarded_headers {
            warnings.push("server.trust_forwarded_headers is enabled; only use this behind a trusted reverse proxy.".to_string());
        }

        warnings
    }
}

fn base_url_host(base_url: &str) -> Option<String> {
    let without_scheme = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))?;
    let authority = without_scheme.split('/').next()?.trim();
    if authority.is_empty() {
        return None;
    }

    if authority.starts_with('[') {
        let end = authority.find(']')?;
        return Some(authority[1..end].to_string());
    }

    Some(authority.split(':').next()?.to_string())
}

fn is_local_host_label(host: &str) -> bool {
    matches!(
        host.trim().to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

impl Settings {
    pub fn runtime_warnings(&self) -> Vec<String> {
        let mut warnings = self.server.runtime_warnings();

        if !self.rate_limit.enabled {
            warnings.push("rate_limit.enabled is disabled; public deployments may be easier to abuse.".to_string());
        }

        if self.search.remote_autocomplete_enabled {
            warnings.push("search.remote_autocomplete_enabled is enabled; autocomplete queries will leave the local deployment.".to_string());
        }

        if self.search.max_concurrent_engines == 0 {
            warnings.push("search.max_concurrent_engines is 0; search fan-out is effectively disabled.".to_string());
        }

        if self.search.max_page == 0 {
            warnings.push("search.max_page is 0; paginated search navigation will not work correctly.".to_string());
        }

        if self.cache.enabled && self.cache.max_entries == 0 {
            warnings.push("cache.enabled is true but cache.max_entries is 0.".to_string());
        }

        if self.cache.enabled && self.cache.ttl_secs == 0 {
            warnings.push("cache.enabled is true but cache.ttl_secs is 0.".to_string());
        }

        warnings.sort();
        warnings.dedup();
        warnings
    }
}
