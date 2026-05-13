# Metasearch Integration Guide

This guide is for embedding the `metasearch` workspace into another Rust host or for operating the HTTP service as part of a larger system.

## Crate roles

- `metasearch-core`
  Shared contracts: `SearchQuery`, `SearchResult`, `SearchResponse`, `SearchCategory`, `Settings`, and the `SearchEngine` trait.
- `metasearch-engine`
  `EngineRegistry` and concrete built-in engines.
- `metasearch-server`
  `SearchCache`, `EngineHealthTracker`, `SearchOrchestrator`, route modules, middleware, and template handling.
- `metasearch-cli`
  Operator binary for local launches and config inspection.

## The main embed path

For a Rust host, the core object is `SearchOrchestrator`.

```rust
use std::sync::Arc;

use metasearch_core::query::SearchQuery;
use metasearch_engine::EngineRegistry;
use metasearch_server::{
    cache::SearchCache,
    health::EngineHealthTracker,
    orchestrator::SearchOrchestrator,
};

async fn build_orchestrator() -> anyhow::Result<SearchOrchestrator> {
    let client = reqwest::Client::builder()
        .user_agent("MyHost/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let registry = Arc::new(EngineRegistry::with_defaults(client));
    let cache = SearchCache::new(10_000, 300);
    let health = Arc::new(EngineHealthTracker::new());

    Ok(SearchOrchestrator::new(registry, cache, health, 50))
}

async fn run_query(orchestrator: &SearchOrchestrator) {
    let query = SearchQuery::new("qwen3 rust integration");
    let cache_key = metasearch_server::cache::SearchCache::cache_key_for_query(&query);
    let response = orchestrator.search(&query, &cache_key).await;
    println!("{} results", response.number_of_results);
}
```

## Query features that now work end-to-end

- category routing through `SearchCategory`, including comma-separated multi-category requests
- language selection
- safe-search level
- page selection
- time-range filtering
- explicit engine targeting with `SearchQuery.engines`
 - bounded normalization for query text, language, time-range, categories, and engine lists

If `SearchQuery.engines` is non-empty, the orchestrator now uses those engines instead of the category default set. Otherwise it unions the registered engines for the full requested category set before filtering by health and concurrency limits.

## Cache behavior

Use `SearchCache::cache_key_for_query(&query)` when you build your own cache keys.

That key includes:

- query text
- full normalized category set
- language
- page
- safe-search level
- time-range
- explicit engine list

That is the correct key to use for hosts that expose engine selection or alternate filtering.

## HTTP server integration

The HTTP server is built from [crates/metasearch-server/src/app.rs](F:/flow/metasearch/crates/metasearch-server/src/app.rs). The mounted routes are:

- `/`
- `/search`
- `/about`
- `/status`
- `/preferences`
- `/api/v1/search`
- `/api/v1/engines`
- `/api/v1/config`
- `/api/v1/status`
- `/autocomplete`
- `/robots.txt`
- `/opensearch.xml`
- `/health`
- `/livez`
- `/readyz`
- `/static/*`

### Search API example

```text
GET /api/v1/search?q=rust&categories=it,science&language=en&page=1&safe_search=1&engines=github,searchcode_code
```

### Engines catalog

`/api/v1/engines` now returns structured metadata instead of only engine names:

- name
- display name
- homepage
- categories
- enabled flag
- timeout
- weight
- health snapshot when available

### Health endpoint

`/health` now reports:

- version
- registered engine count
- tracked engine count
- unhealthy engine count
- unhealthy engine names
- cache/rate-limit/bot-detection flags
- config warning count and warning messages

### Liveness and readiness probes

- `/livez` is the lightweight process-level liveness probe.
- `/readyz` is the deployment readiness probe and returns `503` when the service has no registered engines.
- The richer runtime picture still lives at `/health`, `/api/v1/status`, and `/status`.

### Operator status endpoint

`/api/v1/status` is the higher-detail operator surface. It adds:

- effective runtime posture
- structured config warnings
- full tracked engine health snapshots
- current cache/rate-limit/search/runtime settings

The browser-facing equivalent is `/status`, which renders the same deployment posture and warnings as an HTML page.
It now also breaks out asset-integrity issues for the configured template/static roots so operators can distinguish broken mounts from ordinary config posture warnings.

On the HTML side, normalized query state is now preserved through result-page transitions, including category changes, resubmits, empty-state recovery, and infinite scroll.

## Config loading

The CLI now loads [config.toml](F:/flow/metasearch/config.toml) automatically when present or from `--config <path>` when provided.

Use the config file for:

- bind address
- base URL
- template and static asset roots
- cache size and TTL
- search defaults
- rate limiting
- optional bot detection
- proxy-header trust
- response hardening and browser CORS posture
- explicit browser origin allowlisting

## Templates and static files

The UI expects:

- [templates](F:/flow/metasearch/templates)
- [static](F:/flow/metasearch/static)

The runtime roots are configurable through:

- `server.templates_dir`
- `server.static_dir`
- `metasearch serve --templates <dir>`
- `metasearch serve --static-dir <dir>`

If you embed the server into another workspace, point those roots at the copied asset directories in your host project instead of relying on the metasearch workspace root.

## Deployment guidance

- Put the server behind a reverse proxy for TLS termination and deployment-specific policy, but note that the app now emits its own baseline security headers.
- Leave bot detection disabled until you know which automated clients you need to allow.
- Leave `trust_forwarded_headers = false` unless your proxy is authoritative for client IP headers.
- Leave `permissive_cors = false` unless you intentionally want fully open cross-origin browser access.
- Prefer `allowed_origins = ["https://your-host.example"]` when you need controlled cross-origin browser access.
- When you do enable browser- or proxy-facing settings, make sure `server.base_url` is the real public URL; the runtime warning surfaces now flag common mistakes like leaving it on `localhost` or `http://`.
- Tune `max_concurrent_engines` for the machine you actually deploy on.
- Prefer explicit engine lists for narrow domains like code, maps, or science.
- The browser UI uses local recent-query suggestions by default; `/autocomplete` can remain disabled for privacy-sensitive deployments.
- Dynamic HTML and JSON responses now ship with `Cache-Control: no-store`, so browser or proxy caching needs to be an explicit operator decision if desired.
- Search/autocomplete/API/operator surfaces also emit `X-Robots-Tag: noindex, nofollow, noarchive`, and `/robots.txt` disallows indexing of result, API, and operator endpoints by default.
- Requests now emit lightweight `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers, and the container healthchecks should target `/readyz`.
- The security-header layer now also emits `Origin-Agent-Cluster`, `X-Permitted-Cross-Domain-Policies`, and `Strict-Transport-Security` when the configured public `base_url` is HTTPS.

## Files worth integrating against

- [crates/metasearch-core/src/query.rs](F:/flow/metasearch/crates/metasearch-core/src/query.rs)
- [crates/metasearch-core/src/result.rs](F:/flow/metasearch/crates/metasearch-core/src/result.rs)
- [crates/metasearch-core/src/config.rs](F:/flow/metasearch/crates/metasearch-core/src/config.rs)
- [crates/metasearch-engine/src/registry.rs](F:/flow/metasearch/crates/metasearch-engine/src/registry.rs)
- [crates/metasearch-server/src/orchestrator.rs](F:/flow/metasearch/crates/metasearch-server/src/orchestrator.rs)
- [crates/metasearch-server/src/routes/api.rs](F:/flow/metasearch/crates/metasearch-server/src/routes/api.rs)
- [crates/metasearch-server/src/routes/search.rs](F:/flow/metasearch/crates/metasearch-server/src/routes/search.rs)
