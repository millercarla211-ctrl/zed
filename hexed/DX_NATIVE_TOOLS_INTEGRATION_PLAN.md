# DX Native Tools Integration Plan

## Goal

DX/Zed should become the fastest native editor for coding, agent work, local models, remote models, search, project health, deployment, and multi-media version control. The product should keep Zed-class editor performance while exposing the missing DX surfaces as real panels and tools, not decorative UI.

This plan connects the existing G-drive DX tools into the F-drive Zed fork in a maintainable order.

## Executive Product Thesis

DX/Zed should be a native, Rust-first development environment that combines the speed of Zed, the agent management of modern AI workspaces, the provider freedom of a universal AI router, the source grounding of NotebookLM, the task clarity of Kiro, and the shipping controls of a professional engineering console.

The product bet is simple:

- Keep the editor fast and full-width by default.
- Make local models first-class through direct llama.cpp and Rust integration.
- Make remote models first-class through the broadest provider/auth catalog in any code editor.
- Give agents native tools for metasearch, project health, version control, deployment, sources, and long-context reduction.
- Keep every panel real, typed, permission-aware, and connected to actual project state.

This is not a plan to turn Zed into an Electron-style AI shell. It is a plan to make a native editor that can absorb the best workflows from Cursor, Codex, Claude Code, Kiro, NotebookLM, Perplexity, and deployment dashboards while staying faster and more developer-friendly.

## Full Product Vision Backup

The long-term product should include these major lanes:

- Provider catalog: unify local models, premium-account models, free-tier models, LiteLLM-style provider aliases, models.dev metadata, OpenRouter/Ollama-style public model lists, provider auth profiles, context limits, capabilities, pricing, free-quota hints, and routing roles.
- Fast catalog loading: store provider/model catalogs as versioned `rkyv` archives loaded with `memmap2`, so model menus and routing can start from a binary artifact instead of parsing JSON on the hot path.
- Metasearch: expose the G-drive metasearch workspace as an AI tool in the Agent panel, searching many engines and vertical sources at once with citations, engine health, cancellation, and compact result cards.
- Forge: integrate Forge as a version-control and sync system for code, media, datasets, databases, project files, and multi-remote backups, including GitHub, GitLab, Bitbucket, object storage, media remotes, and local/network mirrors.
- Drive: build a source and task workspace where markdown specs, docs, web pages, PDFs, code references, prompts, media metadata, and project notes can become durable sources for agents.
- DCP: make DCP the native DX capability protocol while bridging MCP, ACP, Codex/Claude Code style tools, local Rust tools, and browser tools through one permission and receipt model.
- Serializer and RLM: use serializer for compact tool catalogs, packed conversations, provider catalogs, and project summaries; use RLM for long-context reduction, recursive summarization, and token-saving source preparation.
- Check panel: add a project-quality surface for code smell, oversized files, folder structure, formatting/lint/typecheck status imports, test freshness, visual proof receipts, security posture, and deploy readiness.
- Deploy panel: add a shipping surface for CI/CD, Vercel, Cloudflare, GitHub Actions, Docker, local scripts, custom deploy targets, logs, previews, production URLs, rollback, env status, and release receipts.
- Workspace layout: keep the center editor/browser/terminal/media screens full-width by default; add a left Sources rail and a right project details/tasks/summary/quick links rail inspired by Codex Desktop and NotebookLM.
- Top panels: make AI, Forge, Drive, Check, Deploy, and the existing screen dock visible as real product surfaces, not placeholder buttons.
- Screen workflow: keep the Screen Dock Carousel as the main full-width screen navigation model, with Editor as the default coding screen and Browser/Terminal/media screens available through smooth edge reveal, keyboard commands, and click-to-activate slivers.
- Internationalization and additional DX crates: inspect and integrate `i18`, `driven`, `dcp`, `dx-check`, and other G-drive Rust crates as adapter-backed capabilities when they are mature enough, without forcing unstable crates into editor startup.

## Prioritized Next-Build Order

1. `dx_catalog`: create the archived provider/model catalog loader and generator.
2. AI model picker/routing: connect `dx_catalog` to the Agent panel provider/model selection surfaces.
3. Metasearch AI tool: wire `metasearch-core` and `metasearch-engine` into a cancellable background tool with compact cited results.
4. Serializer/RLM prep pipeline: compact tool catalogs, conversations, source packs, and search results before model calls.
5. Forge panel: surface code/media snapshot status, remotes, sync plans, jobs, restore points, and conflict warnings.
6. Drive/Sources rail: add durable sources, markdown task specs, source sets, and agent context packs.
7. Check panel: show project score, code smell, folder structure, status imports, visual-test receipts, and recommended repairs.
8. Deploy panel: add deploy targets, env readiness, CI/CD receipts, preview URLs, production status, and rollback.
9. DCP bridge: unify DCP/MCP/ACP/local tools under one schema, permission model, and receipt system.
10. Codex-style rails: add optional left Sources and right project/task rails without slowing the full-width editor.

## Local Sources Inspected

- `G:\Workspaces\flow`: Rust-first local AI runtime with `ZedFlowAdapter`, `CodexFlowAdapter`, local model roles, provider-catalog planning, Forge bridge planning, metasearch, serializer, and RLM.
- `G:\Workspaces\flow\metasearch`: reusable metasearch workspace with `metasearch-core`, `metasearch-engine`, `metasearch-server`, and `metasearch-cli`.
- `G:\Workspaces\flow\metasearch\crates\metasearch-engine\src\registry.rs`: built-in engine registry with many search engines and category-specific adapters.
- `G:\Workspaces\flow\forge`: media-aware version-control layer with chunking, content addressing, `rkyv`, `memmap2`, multi-remote sync, restore, jobs, and transport work already underway.
- `G:\Workspaces\flow\serializer`: TOON-compatible and dx-serializer crate for compact tool catalogs, packed conversations, and schema-efficient agent payloads.
- `G:\Workspaces\flow\rlm`: embeddable long-context reduction runtime for search, summarization, and agent context preparation.
- `G:\Workspaces\flow\src\provider_catalog`: starter plan types for local registry, models.dev, LiteLLM-style normalization, and native provider scanning.
- `G:\Workspaces\flow\src\forge_bridge`: starter plan types for code, audio, video, image, 3D, documents, datasets, and multi-remote sync.
- `G:\Dx\agent\crates\zeroclaw-providers`: stronger provider implementation source with auth profiles, OAuth/token storage, OpenAI-compatible providers, Codex, Claude Code, Gemini, Qwen, OpenRouter, Ollama, llama.cpp, models.dev lookup, routing, and reliability wrappers.

## Architecture Rule

Do not copy these crates blindly into Zed. Build thin, typed adapter crates first, then wire panels to those adapters. The editor hot path must not parse large JSON, scan provider folders, fetch network catalogs, or instantiate search engines during normal startup.

The integration shape should be:

- `dx_catalog`: archived provider/model catalog using `rkyv` plus `memmap2`.
- `dx_ai_tools`: Zed agent tool bridge for metasearch, RLM, serializer, and provider routing.
- `dx_forge`: host adapter around Forge repository, snapshot, restore, job, and multi-remote APIs.
- `dx_drive`: source-library and task-document model backed by Forge snapshots and serializer summaries.
- `dx_check`: project health scoring, file/folder review, formatting/lint status import, and visual-test evidence.
- `dx_deploy`: deployment and CI/CD provider registry, run history, receipts, rollback, and environment checks.
- `dx_dcp`: capability protocol bridge for DCP, MCP, ACP, and local native tools with one permission model.

## Feature Set 1: Provider And Model Catalog

Build this first because every AI surface needs it.

- Ingest local Flow provider roles, `zeroclaw-providers`, models.dev, LiteLLM-style provider naming, local llama.cpp models, OpenRouter/Ollama public model lists where available, and user auth profiles.
- Normalize provider IDs, model IDs, aliases, capability flags, context limits, pricing, free-tier signals, OAuth/token support, local/remote/offline status, and preferred routing roles.
- Generate a versioned binary catalog artifact at build or refresh time.
- Load the artifact with `memmap2`, validate with `rkyv` bytecheck, and expose immutable zero-copy views to menus, agent routing, and settings.
- Keep JSON fetch/parse out of app startup. JSON belongs in refresh jobs, not the editor hot path.
- Add fallback behavior: if remote catalogs fail, keep the last good archived catalog and local models.

Acceptance:

- AI model picker opens without network access.
- Local model routes and remote provider routes appear from the same catalog.
- Free-tier and premium-account signals are visible to the router.
- Auth state is linked by profile ID, not raw secret strings.

## Feature Set 2: Metasearch As An AI Tool

Wire metasearch into the AI panel as a real tool.

- Embed `metasearch-core` query/result types first.
- Use `metasearch-engine` registry through a background runtime, not on the UI thread.
- Add a Zed agent tool named `web_search` or `dx_metasearch`.
- Support categories: web, code, docs, news, images, video, packages, academic, security, and local project sources.
- Return compact, cited result cards using serializer/RLM summaries when needed.
- Add rate limits, cancellation, safe-search defaults, source allowlists/denylists, and per-engine health.

Acceptance:

- Agent panel can call metasearch without a sidecar server.
- Search results include source, title, URL, snippet, rank, engine, timestamp, and confidence.
- Failed engines do not fail the whole search.

## Feature Set 3: Forge Panel

Forge should become the version-control surface for code plus media.

- Add a Forge panel beside existing editor/product panels.
- Show repository status, chunk/storage health, snapshots, remotes, mirror jobs, restore points, and conflict warnings.
- Support GitHub, GitLab, Bitbucket, R2/S3-compatible storage, Google Drive, Dropbox, YouTube, Sketchfab, SoundCloud, and local/network mirrors as capability-backed remote adapters.
- Treat media files as first-class assets instead of pushing every large file through Git-LFS.
- Keep destructive restore operations behind explicit confirmation and preview.

Acceptance:

- Current project can show code/media snapshot status.
- Multi-remote sync plans can be previewed before execution.
- Restore actions are auditable and reversible where possible.

## Feature Set 4: Drive And Task UI

Build a markdown-backed task/source surface like Kiro and NotebookLM, but local-first.

- Add a Drive panel for sources: repo docs, files, PDFs, web pages, notes, media metadata, prompts, task specs, and imported references.
- Add markdown task documents that can become agent tasks, checklists, implementation plans, or saved workspace memory.
- Back sources with Forge snapshot IDs and serializer summaries so the AI can cite exact source versions.
- Let users pin sources to the left-side Sources rail and task/project summary to the right-side rail.

Acceptance:

- A user can attach sources to an agent task without leaving the editor.
- Agent context can be rebuilt from source IDs, not copied giant prompts.
- Markdown tasks are editable, versioned, and linked to code changes.

## Feature Set 5: Check Panel

The Check panel should make project quality visible.

- Integrate `dx-check` as a scored project health surface.
- Score code smell, oversized files, folder structure, formatting state, lint/typecheck/build status imports, test freshness, dependency risk, security posture, visual evidence, and deployment readiness.
- Use read-only analysis first. Mutating fixes should be explicit actions.
- Render actionable cards with owner, severity, command, file path, and expected improvement.
- Support visual-test receipts for Browser/WebPreview flows.

Acceptance:

- The current project has a clear score and blocker list.
- Users can see what raises the score before running heavy commands.
- Visual and runtime proof receipts can be attached to quality gates.

## Feature Set 6: Deploy And CI/CD Panel

Add a deployment surface for project shipping.

- Track Vercel, Cloudflare, GitHub Actions, local scripts, Docker, and custom CI/CD commands as deploy targets.
- Show env readiness, branch, commit, latest run, logs, preview URLs, production URL, rollback options, and blocker status.
- Keep deploy actions permission-gated and auditable.
- Reuse Forge receipts and Check panel results for release confidence.

Acceptance:

- User can see deploy readiness from inside DX/Zed.
- A deploy action writes a receipt that AI panels can read later.

## Feature Set 7: DCP Protocol Bridge

DCP should be the native DX capability protocol, while MCP and ACP remain supported bridges.

- Define one capability schema for tools, prompts, resources, UI cards, permissions, secrets, sessions, and receipts.
- Build adapters for MCP servers, ACP agents, Codex/Claude Code style tools, local Rust tools, and browser tools.
- Use serializer packed catalogs for tool schemas to cut prompt/token overhead.
- Apply one permission policy across local filesystem, network, shell, browser, model calls, and media actions.

Acceptance:

- Existing MCP/ACP tools can be represented in DCP without losing permissions.
- AI panels can list, run, and audit tools through one protocol.

## Feature Set 8: Serializer And RLM Token Budget Engine

Use serializer and RLM to reduce prompt cost and latency.

- Use serializer for compact tool catalogs, packed conversations, tool results, project summaries, and provider catalogs.
- Use RLM for long files, docs, source packs, and task histories before sending to local or remote models.
- Store reusable summaries in Forge/Drive with content hashes and invalidation rules.
- Prefer local models for summary/reduction passes when latency and quality are acceptable.

Acceptance:

- Large source/task contexts are summarized with stable citations.
- Tool catalogs can be passed in compact serializer form.
- Repeated agent work reuses cached reductions when source hashes match.

## Feature Set 9: Codex-Style Workspace Layout

Keep the editor full-width by default, then add optional rails.

- Center: full-width active screen loop for Editor, Browser, Terminal, media preview, and future screens.
- Left rail: Sources, Drive, pinned docs, local/remote context, active source sets.
- Right rail: project details, task summary, quick links, Check score, Forge state, Deploy state, and agent action receipts.
- Top panels: AI, Forge, Drive, Check, Deploy, plus existing screen dock.
- The Screen Dock Carousel remains the main way to move among full-width screens.

Acceptance:

- Coding stays the default first screen.
- Rails are useful when open and cheap when closed.
- No panel blocks typing or editor repaint performance.

## Screen Dock Follow-Up

The current carousel fix preserves the pane layout contract by wrapping the center pane with full-size flex behavior. The next UI pass should add visual polish only after native runtime testing confirms the main screens remain visible:

- subtle spring easing for reveal/commit animation,
- edge dwell affordance that appears only after the delay,
- keyboard command discoverability,
- wraparound visual cue at first/last screen,
- reduced-motion behavior,
- and persistence of last active screen order when custom screens exist.

Rainbow cursor remains deferred until it can be measured as opt-in and GPU-cheap.

## Verification Strategy

This repo is too large for repeated full builds. Use this cadence:

- Source review and narrow search for every slice.
- `rustfmt` only on touched Rust files.
- `git diff --check` for whitespace/conflict hygiene.
- One final `just run` at coherent milestones when runtime verification is needed.
- Never put catalog JSON parsing, provider fetching, metasearch startup, or Forge scans on editor startup without cached artifacts and background scheduling.

## Next Target

Implement `dx_catalog` first:

- define archived provider/model structs,
- add an artifact loader using `memmap2` plus `rkyv` validation,
- write a generator that can ingest the local Flow plan, `zeroclaw-providers` metadata, models.dev JSON, and LiteLLM-style aliases,
- expose a read-only catalog service to the AI panel model picker and routing layer,
- then use that service to power provider menus and agent model selection.

## Raw Vision Backup

This section preserves the original product prompt for future recovery and comparison. The main-screen visibility issue mentioned here was already fixed before this backup pass.

```text
In our g drive you can find providers, metasearch, forge, serializer, i18, driven, dcp, and other dx tools rust crates now please like make a plan that's way we can use those in our dx/zed forge code editor like we have integrate all providers or free models, lite-llm providers list, models.dev providers list as a rkyv and memmap2 so that we can read it fast not slow like json and also we can implememnt the most providers in code editor in the world, then we have forge which is like git for not only code but also everything and in there you can have many database or remove backups not only one you get with github, gitlab, and also you can do version controls of medias that github needs git-lfs which is not that good like forge - so please kindly add new metasearch  in our ai so that our ai panel can use that when they do web search or just use that as tool to search so much online places and search engines at onces beating perplexity correctly!!! and also then like kiro code editor we can make markdown based task ui with drive and use dcp to beat current claude mcp and google acp and also use serializer for speed where we can do to make our zed code editor even faster than actual zed and then like the 5 top panels we will add new forge, drive , check that uses dx-check to like give points to current project about code smell, files and folder sturcuture and also lint and formatting status and also create another by what users can deploy or many ci cd so please add these new panel so that by using check panel users can mananger current project states and also do visual test using our check panel and we also in our ai panel we added metasearch and also rlm and other token saving rust crate to save token and serializer for saving more tokens and also like the image of the codex app I gave you that from now on by defaut our zed code editor to have full width and also can be used like a panel to but the new things is that like codex desktop app in the right it will have project details and other tasks for a summary and quick links in the right and in the left there will be one sources so like the google notbooklm we can configure sources there correctly in that our dx/zed code editor fork will be the best in the world and also currnet our main screens are not showing for some reason after you implememnt the screens swipe funtionaliity so please fix that first and then please create a plan to add the features I told you!!!
```
