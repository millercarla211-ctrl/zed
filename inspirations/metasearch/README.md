# metasearch

Rust metasearch workspace with a reusable core crate, a large engine registry, an Axum server, and a CLI launcher.

## What is in this workspace

- `crates/metasearch-core`
  Shared query, result, category, config, and engine trait types.
- `crates/metasearch-engine`
  The built-in engine registry and concrete engine adapters.
- `crates/metasearch-server`
  The web UI, JSON API, OpenSearch endpoint, cache, orchestrator, health tracker, and middleware.
- `crates/metasearch-cli`
  The binary that loads config, initializes the registry, and starts the server.

## Current scope

- the default registry loads a large built-in engine catalog through `EngineRegistry::with_defaults(...)`
- the exact runtime adapter count is exposed by `/api/v1/engines`, `/api/v1/config`, and the homepage/about pages
- The server supports:
  - HTML search UI at `/` and `/search`
  - informational pages at `/about`, `/preferences`, and `/status`
  - JSON API at `/api/v1/search`
  - engine catalog at `/api/v1/engines`
  - runtime config summary at `/api/v1/config`
  - operator status at `/api/v1/status`
  - autocomplete at `/autocomplete`
  - crawler policy at `/robots.txt`
  - OpenSearch descriptor at `/opensearch.xml`
  - health endpoint at `/health`
  - liveness probe at `/livez`
  - readiness probe at `/readyz`
- Search orchestration includes:
  - full-response caching
  - in-flight request coalescing
  - engine health tracking
  - adaptive per-engine timeouts
  - explicit engine targeting through `SearchQuery.engines`
  - bounded input normalization for queries, categories, engines, language, and time-range filters

## Production-facing behavior

- The CLI now loads `config.toml` automatically when present.
- Search cache keys now include:
  - full normalized category set
  - language
  - page
  - safe-search level
  - time-range
  - explicit engine list
- The JSON API now honors:
  - `categories` including comma-separated multi-category requests
  - `language`
  - `page`
  - `safe_search`
  - `time_range`
  - `engines`
- The server now mounts the OpenSearch route and includes lightweight:
  - per-client rate limiting
  - optional bot detection
- Rate-limited responses and successful requests now emit basic `X-RateLimit-*` headers for clients and operators.
- The server now ships same-origin by default, with opt-in permissive CORS and built-in response hardening headers.
- The server now supports explicit CORS origin allowlisting when you need browser access from specific external hosts.
- The UI now includes working `/about` and `/preferences` pages, the visible search preferences carry into real searches, and homepage category selection is part of the actual search form.
- The HTML results flow now preserves normalized query state across resubmits, category switches, empty-state recovery, and infinite scroll, including `time_range` when the client provides it.
- The browser UI now uses local recent-query autocomplete and self-hosted icon rendering, so it no longer depends on passive third-party assets for normal operation.
- Dynamic HTML and JSON responses now default to `Cache-Control: no-store`, while static assets and the OpenSearch descriptor get bounded cache headers.
- Search, autocomplete, API, and operator responses now emit `X-Robots-Tag: noindex, nofollow, noarchive`, and the app serves an explicit `robots.txt`.
- `/health` now reports engine counts and unhealthy-engine state instead of a bare `"ok"`, and returns `503` when the service is fundamentally misconfigured.
- `/livez` and `/readyz` now provide explicit liveness and readiness probes, and the container healthchecks use `/readyz`.
- `/api/v1/status` now exposes operator-facing runtime warnings, engine health snapshots, and the effective deployment posture in one place.
- `/status` now provides the same operator view in HTML for browser-based deployments and manual handoff checks, including asset-integrity visibility for the configured template/static roots.
- Server startup now logs runtime warnings and shuts down gracefully on termination signals.
- Server startup now validates the effective template/static asset roots before binding the HTTP port.
- Server startup now also validates the required HTML and static asset files inside those roots before binding the HTTP port.

## Config

The root [config.toml](F:/flow/metasearch/config.toml) is now aligned with the current `Settings` model:

```toml
[server]
host = "0.0.0.0"
port = 8888
base_url = "http://localhost:8888"
templates_dir = "templates"
static_dir = "static"
secret_key = "change-me-in-production"
image_proxy = true
trust_forwarded_headers = false
security_headers_enabled = true
permissive_cors = false
allowed_origins = []

[search]
safe_search = 1
default_language = "en"
max_page = 10
request_timeout_ms = 10000
max_concurrent_engines = 50
remote_autocomplete_enabled = false

[cache]
enabled = true
ttl_secs = 300
max_entries = 10000

[rate_limit]
enabled = true
requests_per_second = 10
burst_size = 30

[bot_detection]
enabled = false
block_missing_user_agent = false
```

## CLI

Typical commands:

```powershell
metasearch serve
metasearch serve --config config.toml
metasearch engines
metasearch config
```

Important notes:

- `--host` and `--port` override the loaded config.
- `--templates` and `--static-dir` override the configured asset roots when you need to embed the server outside the workspace root.
- The generated `base_url` prefers `localhost` when the bind address is `0.0.0.0`.
- `trust_forwarded_headers` should stay `false` unless the service is behind a trusted reverse proxy that rewrites client IP headers correctly.
- `permissive_cors` is off by default so the HTTP surface behaves as same-origin unless you explicitly relax it.
- `allowed_origins` can be used for a narrower browser allowlist when you want cross-origin access without opening the API to every origin.
- the runtime warning surfaces now flag obviously mismatched `base_url` settings, such as `localhost` or `http://` when the deployment enables external browser/proxy-facing behavior.

## Containers

The container files now follow the same runtime layout as the CLI:

- [Dockerfile](F:/flow/metasearch/Dockerfile) copies the root `config.toml`
- [docker-compose.yml](F:/flow/metasearch/docker-compose.yml) mounts `config.toml`, `templates`, and `static`
- the runtime container now runs as a non-root user and exposes a built-in healthcheck
- the built-in container healthcheck now uses `/readyz`
- the container now stops with `SIGTERM` and a grace period so the server can shut down cleanly
- the container starts with `metasearch serve --config config.toml`

## Library integration

Use the workspace as a Rust library when you need metasearch inside another host:

```rust
use std::sync::Arc;

use metasearch_core::query::SearchQuery;
use metasearch_engine::EngineRegistry;
use metasearch_server::{
    cache::SearchCache,
    health::EngineHealthTracker,
    orchestrator::SearchOrchestrator,
};

let client = reqwest::Client::new();
let registry = Arc::new(EngineRegistry::with_defaults(client));
let cache = SearchCache::new(10_000, 300);
let health = Arc::new(EngineHealthTracker::new());
let orchestrator = SearchOrchestrator::new(registry, cache, health, 50);

let query = SearchQuery::new("rust metasearch");
```

See [INTEGRATION_GUIDE.md](F:/flow/metasearch/INTEGRATION_GUIDE.md) for the integration details.

## Operator notes

- Set `server.templates_dir` and `server.static_dir` or pass `--templates` / `--static-dir` when the app is launched outside the workspace root.
- Put it behind a reverse proxy for TLS termination and any deployment-specific allowlist logic, but the server now ships with its own baseline security headers and same-origin browser policy.
- If you need browser access from another host, prefer `allowed_origins = [...]` over `permissive_cors = true`.
- Remote autocomplete is disabled by default and the browser UI does not require it; when left disabled, autocomplete stays local to the browser's recent-query storage.
- Treat the built-in engine count as adapter coverage, not a guarantee that every upstream site is always reachable or stable.

## Project docs

- [INTEGRATION_GUIDE.md](F:/flow/metasearch/INTEGRATION_GUIDE.md)
- [TODO.md](F:/flow/metasearch/TODO.md)
- [CHANGELOG.md](F:/flow/metasearch/CHANGELOG.md)
- [docs/PRODUCTION_READY.md](F:/flow/metasearch/docs/PRODUCTION_READY.md)
- [docs/METASEARCH_STATUS.md](F:/flow/metasearch/docs/METASEARCH_STATUS.md)
