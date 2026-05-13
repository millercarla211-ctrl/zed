# Production Ready

`metasearch` is now in a production-ready source state for this repository scope.

## What is complete

- reusable Rust core types in `metasearch-core`
- large built-in registry in `metasearch-engine`
- cache + coalescing + health-aware orchestration in `metasearch-server`
- HTML UI, JSON API, autocomplete, OpenSearch, and health routes
- operator status endpoint with runtime warnings and health snapshots
- browser status page for manual deployment checks
- browser status page now separates asset-integrity issues from general runtime warnings
- direct status-page navigation across the HTML app surfaces
- dedicated `/livez` and `/readyz` probe endpoints for deployment healthchecks
- configurable template/static asset roots for embedded or non-root deployments
- startup validation of the effective template/static asset roots
- startup validation of the required template/static files inside those roots
- bounded request normalization across HTML search, JSON API, and autocomplete
- working `/about` and `/preferences` pages with local preference carry-through
- homepage category selection now drives a real search parameter instead of dead links
- CLI config loading from a TOML file
- container files aligned with the real config/layout
- normalized `base_url` reporting across CLI-derived and server-reported operator surfaces
- explicit engine selection support end-to-end
- cache keys aligned with the full search query, including normalized multi-category requests
- rate limiting and optional bot detection
- basic `X-RateLimit-*` response headers for operators and clients
- same-origin-by-default CORS with explicit opt-in permissive mode
- explicit origin allowlisting for controlled cross-origin browser clients
- built-in response hardening headers
- conditional HSTS plus additional browser hardening headers
- privacy-oriented response cache headers
- crawler controls for search/autocomplete/API/operator surfaces
- runtime warnings for mismatched public `base_url` settings in browser/proxy-facing deployments
- local recent-query autocomplete with no passive third-party browser asset dependencies
- operator docs aligned with the actual code layout
- debug scrape artifacts removed from the committed source tree
- non-root runtime container and built-in healthchecks
- readiness-based container healthchecks and graceful shutdown handling
- explicit `SIGTERM`/grace-period container stop behavior

## What operators still own

- reverse proxy and TLS
- environment-specific rate-limit tuning
- upstream API keys or instance URLs for engines that need them
- any deployment-specific CORS allowlisting if they do not want the default same-origin posture

## Current deployment assumptions

- `server.templates_dir` and `server.static_dir` are set correctly for the deployment layout
- `config.toml` is the source of runtime defaults unless CLI flags override them
- the engine registry count reflects adapter coverage, not guaranteed upstream stability
- `trust_forwarded_headers` stays off unless the deployment is behind a trusted reverse proxy
- `allowed_origins` is empty unless the operator explicitly wants controlled cross-origin browser access
- result/API/operator surfaces are intentionally non-indexable by default
