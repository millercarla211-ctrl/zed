# Metasearch Status

## Current state

- source-complete for the current repository scope
- library-first crate structure is intact
- server runtime surface is coherent
- docs now match the actual implementation
- browser UI and server defaults are now privacy-hardened for deployment

## Major completed areas

- engine registry and adapter loading
- search orchestration
- request coalescing and caching
- health tracking
- HTML and JSON search surfaces
- informational and preference pages
- operator config loading
- explicit engine targeting
- multi-category engine routing and cache identity
- health and engine catalog endpoints
- operator status endpoint and runtime warnings
- browser status page for runtime inspection
- browser status page breaks out asset-integrity issues separately
- direct operator-status navigation from the public HTML surfaces
- dedicated liveness and readiness probe endpoints
- configurable template/static asset roots for embedded deployments
- startup validation of the configured template/static asset roots
- startup validation of required template/static files
- local recent-query autocomplete and self-hosted icon assets
- bounded input normalization and selector limits
- same-origin default browser policy and response security headers
- explicit origin allowlisting and privacy-oriented cache-control policy
- crawler controls for result/autocomplete/API/operator endpoints
- runtime warnings for local or non-HTTPS `base_url` values in browser/proxy-facing deployments
- normalized `base_url` reporting across the CLI/server operator surfaces
- conditional HSTS plus additional browser hardening headers
- graceful shutdown handling plus readiness-based container healthchecks

## Known follow-up areas

- stronger per-engine configuration management
- deeper scraper regression coverage for brittle upstream HTML engines
- more advanced public-edge traffic control if deployed at larger scale
