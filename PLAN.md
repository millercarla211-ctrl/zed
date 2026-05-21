# DX/Zed Launch Plan

Date: 2026-05-21
Launch target: 2026-05-22
Primary repo: `G:\Zed`
Canonical DX hub: `G:\Dx`
Supporting DX tools: `G:\Dx\www`, `G:\Dx\cli`, `G:\Dx\dx-agents`, `G:\WWW`, `G:\Workspaces\flow`

## May 22 Launch Sprint Orchestration

Tomorrow's launch goal is to make DX feel like one coherent product: a native Zed-based code editor, DX-WWW and Forge for fast app creation, DX Agents for automation and social workflows, and token-efficient local/remote AI tooling that feels faster and more responsible than the current Electron-heavy editor market.

The sprint should be split across parallel Codex Desktop GPT-5.5 Extra High chats. Each worker should write real code first, inspect the obvious integration errors by reading, and only run lightweight checks at coherent milestones. Repeated full builds are not allowed during the sprint because the product needs more implementation time than rebuild time.

### Canonical Folder Strategy

`G:\Dx` is the launch hub. For today, the safest structure is to gather every launch repo under that hub using stable links and a workspace manifest, not by physically moving active repos during launch crunch.

Physical moves are risky today because:

- `G:\WWW\www\target\debug\dx-www.exe` is currently running a dev server.
- `G:\Zed` is the active editor checkout and already has target/cache paths configured for the G drive.
- `G:\Workspaces\flow` has an untracked `tools/` folder that should be preserved before any relocation.

The launch hub should therefore expose these paths:

- `G:\Dx\zed` -> `G:\Zed`
- `G:\Dx\www-inspirations` -> `G:\WWW`
- `G:\Dx\flow` -> `G:\Workspaces\flow`
- `G:\Dx\token` -> `G:\Dx\inspirations\agent-archive\cursed\token`

After launch, if the product needs a real physical monorepo layout, move repos in a dedicated migration window with clean git status, stopped processes, backups, and one repo at a time.

### Launch Work Split

#### Chat A: WWW + Forge React Ecosystem

Build Forge-native package slices from the cloned React ecosystem mirrors in `G:\WWW\inspirations`.

Priority package families:

- State: `zustand`, `tanstack-store`
- Query: `tanstack-query`
- Auth: `better-auth`
- Database/backend: `supabase`, `drizzle-orm`, `tanstack-db`, `trpc`
- Framework/router: `nextjs`, `tanstack-router`, `react-router`, `next-safe-action`, `hono`, `honox`
- UI/components: `shadcn-ui`, `radix-primitives`, `react-aria`, `lucide`
- i18n: `next-intl`
- Forms/validation: `react-hook-form`, `zod`
- Animation: `motion` and its `framer-motion` package
- Content/docs: `react-markdown`, `mdx`, `fumadocs`
- 3D/media: `three.js`, `react-three-fiber`, `drei`, `xr`, `react-three-rapier`
- Payments: `stripe-js`, `stripe-react-stripe-js`
- AI: `vercel-ai`
- WebAssembly bridge: `wasm-bindgen`

The goal is not fake mini wrappers. The goal is small, source-owned, front-facing Forge packages that expose real APIs and can be used by DX-WWW templates.

#### Chat B: Token / RLM / Serializer

Use the token sources already found on G drive:

- Primary: `G:\Dx\inspirations\agent-archive\cursed\token`
- Secondary comparison copy: `G:\Workspaces\flow\trash\token`
- Optional inspiration: `G:\Dx\inspirations\openclaw\extensions\tokenjuice`

Extract useful token-budget and live-prune ideas into DX token tooling. The Zed-facing output should be simple receipt data first: prompt tokens, output tokens, tool tokens, RLM savings estimate, serializer savings estimate, and source-pack bytes. Use `rkyv`/`memmap2` where it is already isolated and fast to wire, but do not rewrite broad Zed JSON parsing before launch.

#### Chat C: DX Agents + Zed GPUI Bridge

Keep the ZeroClaw-derived `dx-agents` runtime CLI-first, but connect it to Zed GPUI.

The launch UI should add an `Agent` action next to the existing AI modes such as `Write` and `Ask`. It should expose QR/connect UI, social account status, automation entrypoints, and background agent task receipts. Zed should call CLI JSON commands and render their status; it should not store social passwords or provider secrets.

#### Chat D: Zed AI Panel Full-Width UI

Upgrade the Zed AI panel into the DX launch surface:

- left rail: Sources, like NotebookLM,
- center: full-width chat/task/agent workspace by default,
- right rail: Progress, Git, and Background Tasks,
- sidebar actions: New Chat, Search, Plugins, Automations,
- workspace chat groups: Pinned and All Chats.

This must preserve current AI behavior. The work should add small focused GPUI modules instead of replacing the existing panel or creating dummy UI.

#### Chat E: Launch Verification + Status

Create or update a 100-point DX launch status file that tracks WWW+Forge, token/RLM/serializer, agents, Zed AI panel, sidebar, sources rail, right rail, web preview, provider catalog, metasearch, Check, Drive/Forge, and Deploy.

The integrator should use `git status`, `git diff --check`, conflict-marker search, and targeted checks. Only run `just run` when the assembled launch candidate needs runtime validation.

### Copy-Paste Worker Prompts

#### Chat A Prompt: WWW + Forge React Ecosystem

```text
You are Codex GPT-5.5 Extra High working as one launch worker for DX. Work mainly in G:\WWW, G:\Dx\www, and G:\Dx\cli. Do not waste time on repeated full builds. Write real code first, inspect by reading, and only run lightweight checks after coherent milestones.

Goal: create Forge-native, DX-WWW-ready versions of the most valuable React ecosystem packages from existing mirrors in G:\WWW\inspirations. These must not be fake toy wrappers. They should be small, source-owned package slices that expose real useful APIs for DX-WWW templates.

Priority packages:
State: zustand, tanstack-store
Query: tanstack-query
Auth: better-auth
Forms/validation: react-hook-form, zod
i18n: next-intl
UI: shadcn-ui, radix-primitives, react-aria, lucide
Animation: motion / framer-motion
Content: react-markdown, mdx, fumadocs
Payments: stripe-js, stripe-react-stripe-js
AI: vercel-ai
Routing/backend: tanstack-router, react-router, hono, honox, trpc, next-safe-action
3D/media demo: three.js, react-three-fiber, drei, xr, react-three-rapier

Implementation:
1. Inspect existing DX-WWW package/template structure.
2. Create a Forge package layout for these ecosystem slices.
3. Start with a launch template that proves auth, state, query, form, i18n, markdown, payment placeholder, AI action, and UI components can coexist.
4. Add CLI/template metadata so Zed can later list and create these templates.
5. Keep files small and professional. No giant generated blobs. No dummy APIs.
6. Update TODO/status/changelog docs if the repo already has them.

Checks:
- Use git diff --check.
- Use targeted cargo check or package-specific checks only when a milestone is coherent.
- Do not run expensive full builds repeatedly.
- Commit coherent completed changes with a professional message.
```

#### Chat B Prompt: Token / RLM / Serializer

```text
You are Codex GPT-5.5 Extra High working as the DX token-efficiency worker. Work in G:\Dx, G:\Workspaces\flow, and G:\Zed only where needed. Do not mutate unrelated source. Write real code first, then run lightweight checks at milestones.

Important source paths:
Primary token source: G:\Dx\inspirations\agent-archive\cursed\token
Secondary comparison copy: G:\Workspaces\flow\trash\token
Optional inspiration: G:\Dx\inspirations\openclaw\extensions\tokenjuice
Serializer/RLM roots: G:\Workspaces\flow\serializer and G:\Workspaces\flow\rlm

Goal: make a launch-ready DX token system that can feed Zed's AI panel meters. It should support token budgeting, live token pruning, serializer/RLM savings estimates, and receipt files that Zed can read quickly.

Implementation:
1. Inspect the primary token source and compare it with the Flow trash copy. Do not blindly copy trash code.
2. Extract only useful token-budget/live-prune concepts into a clean DX token module or CLI contract.
3. Add receipt output under G:\Dx\.dx\receipts\tokens with JSON first if fastest, and rkyv/memmap2 where already easy and isolated.
4. Define simple CLI surfaces such as dx token estimate --json, dx token budget --json, and dx token prune --json if the DX CLI structure supports it.
5. Prepare Zed-facing fields: prompt_tokens, output_tokens, tool_tokens, saved_by_rlm_estimate, saved_by_serializer_estimate, source_pack_bytes.
6. Keep this practical for tomorrow's demo. Do not rewrite all Zed JSON parsing.

Checks:
- git diff --check.
- Targeted cargo check for touched crates only.
- No repeated full builds.
- Commit coherent completed changes.
```

#### Chat C Prompt: DX Agents + Zed GPUI Bridge

```text
You are Codex GPT-5.5 Extra High working as the DX Agents integration worker. Work in G:\Dx\dx-agents, G:\Dx\cli, and G:\Zed. Write real code first. Use lightweight checks only after coherent changes.

Goal: connect the ZeroClaw-derived dx-agents runtime to Zed's GPUI. The agent stays CLI-first, but Zed gets a professional GUI bridge.

Required Zed UX:
- Add an Agent entry next to existing AI modes like Write and Ask.
- Show QR/connect UI for agent/social connection.
- Show social account connection status.
- Show Automations entrypoint.
- Show background agent task status/receipts.
- Preserve all current Zed AI behavior.

Required CLI bridge:
- Provide or wire JSON commands Zed can call:
  dx agents status --json
  dx agents social list --json
  dx agents automate list --json
  dx agents run --json
- Store receipts under G:\Dx\.dx\receipts\agents.
- Do not store passwords or social secrets in Zed. Zed should trigger/view CLI-managed connection state.

Checks:
- Prefer code review and targeted checks.
- Use git diff --check.
- Use targeted cargo check for touched crates when the bridge compiles as a milestone.
- Do not run repeated full builds.
- Commit coherent completed changes.
```

#### Chat D Prompt: Zed AI Panel Full-Width UI

```text
You are Codex GPT-5.5 Extra High working as the Zed GPUI launch UX worker. Work in G:\Zed on branch dev. Do not remove existing Zed AI features. Write real GPUI code, not dummy UI. Avoid repeated full builds.

Goal: make the Zed AI panel feel like DX: full-width by default, Codex Desktop-inspired on the right, NotebookLM-inspired sources on the left, with smooth professional organization.

Required UI:
- AI panel full width by default when no file/editor is open.
- When an editor/file is open, AI can behave like the normal panel.
- Left side rail: Sources.
- Main area: chat/task/agent workspace.
- Right rail: Progress, Git, Background Tasks.
- Sidebar actions: New Chat, Search, Plugins, Automations.
- Workspace chat groups: Pinned and All Chats.
- Add subtle smooth animation only if GPUI patterns make it safe and cheap.
- Preserve current Write, Ask, existing thread behavior, model picker, token usage, and background task functionality.

Implementation:
1. Inspect crates/agent_ui and existing sidebar/workspace patterns.
2. Add small focused modules/components instead of bloating one file.
3. Wire real data where available; use graceful empty states only when the backend is missing.
4. Add hooks for DX CLI receipts but do not block UI on missing tools.

Checks:
- git diff --check.
- Targeted cargo check for touched Zed crates at milestone.
- No repeated full builds or just run until final integration.
- Commit coherent completed changes.
```

#### Chat E Prompt: Launch Status + Final Integration

```text
You are Codex GPT-5.5 Extra High working as the launch integrator. Work across G:\Zed and G:\Dx after the feature workers make progress. Your job is to make the repo launch-trackable, not to rebuild everything from scratch.

Goal:
- Create or update a 100-point DX launch status file.
- Track WWW+Forge, token/RLM/serializer, agents, Zed AI panel, sidebar, sources rail, right progress rail, web preview, provider catalog, metasearch, Check, Drive/Forge, Deploy.
- Keep the status honest: complete, partial, blocked, next action.
- Sync git correctly once coherent changes are healthy.

Rules:
- Do not waste hours running full builds after every change.
- Use git status, git diff --check, conflict-marker search, and targeted checks.
- Run final just run only when the launch candidate is assembled and the user wants runtime validation.
- Commit coherent changes with professional messages.
- Push dev when clean and appropriate.
```

### Orchestrator Notes For Every Worker

- Ship fast, but do not create throwaway slop.
- Prefer real integration over mock UI.
- Preserve existing working features.
- Keep files small and maintainable.
- Do not delete source, models, inspirations, or user work.
- Do not spend the day proving the whole universe compiles after every tiny edit.
- Code first, inspect carefully, then run lightweight checks at milestones.
- The launch demo matters most: Zed should visibly connect DX-WWW, Forge, Agents, Sources, Progress, Token meters, and the new sidebar experience.

## Executive Thesis

DX/Zed should be the fastest native AI code editor and agent workspace. The core bet is not only "better chat in an editor." The bet is that local models, remote models, deterministic tools, project intelligence, source grounding, and native editor performance can be combined into one product that makes smaller models feel dramatically more capable.

The product advantage comes from four compounding strengths:

- Native Zed/GPUI speed instead of Electron-heavy editing.
- Direct llama.cpp/Rust local model execution instead of relying only on Ollama or LM Studio style universal layers.
- The broadest provider catalog in an editor, including local models, premium accounts, free monthly/daily AI offers, models.dev metadata, LiteLLM-style aliases, and OpenAI-compatible services.
- Powerful native tools that let smaller models succeed by delegating exact work to deterministic capabilities: media tools, web search, project checks, version control, deploy tools, file/source management, and long-context compression.

This can become a venture-scale product if the execution is excellent: fast onboarding, reliable model routing, polished panels, trustworthy permissions, strong demos, and a clear launch story. "Billions" is not guaranteed by the idea alone, but the wedge is real: a native editor plus fast local AI plus powerful tools plus provider freedom is a differentiated opportunity.

## Performance And Responsibility Thesis

The most important DX idea is that AI should become faster, cheaper, more capable, and more responsible at the same time. Most products treat those as tradeoffs. DX should make them reinforce each other.

The performance thesis:

- Direct llama.cpp integration can remove avoidable sidecar/RPC overhead from latency-critical local-agent loops. In the best-fit workflows, the target is up to 90% faster task feel versus heavier universal local-model stacks.
- RLM can reduce long-context prompts by up to 90% on workloads where recursive search, reduction, and citation-preserving summaries replace raw document stuffing.
- dx-serializer can reduce verbose JSON/tool/schema/conversation payloads by up to 70% in suitable agent formats, with packed machine-oriented forms for repeated sessions.
- `rkyv` plus `memmap2` should make provider catalogs, tool catalogs, source summaries, and other large read-mostly data far faster than reparsing JSON on editor startup.
- The point is not a synthetic benchmark trophy. The point is time-to-useful-action: small local models should feel dramatically more capable because DX gives them fast runtime, compact context, and precise tools.

The responsibility thesis:

- DX should make AI powerful without making it reckless.
- Forge should become the default safety layer for risky project operations.
- Delete should not mean permanent loss by default. Destructive actions should create a local, compressed, restorable backup or quarantine first.
- For space saving, prefer zstd-compressed archives or content-addressed Forge snapshots over irreversible deletion.
- Tool calls should leave receipts: what changed, where the backup is, how to restore, and which source state was used.
- This is how DX makes AI useful to serious developers: powerful enough to act, responsible enough to trust.

The developer-positive thesis:

- The goal is not to make software developers obsolete.
- The goal is to stop the chaos where developers are threatened by expensive black-box AI tools that are slow, locked down, and provider-controlled.
- DX should make AI a developer-owned tool again: local when possible, remote when useful, transparent, reversible, fast, and integrated with real project state.
- If a local model can do practical work freely, repeatedly, and safely through strong tools, developers win. They keep agency, spend less money, and move faster.

Internal rally line:

DX wins by playing smart, not by pretending a small model is a frontier model. Give the model speed, memory, sources, tools, Forge safety, and compact context, and suddenly it can beat much larger systems on real workflows. That is the moment worth building toward.

## Launch Narrative

The public launch story for the May 22 sprint launch should be:

- DX/Zed is a native AI editor, not a slow AI shell.
- It runs local models directly and fast, while still supporting frontier remote models.
- It gives small local models powerful tools, so the model does not need to reason through every low-level command.
- It can use free and premium providers together, routing intelligently instead of forcing users to manage scattered accounts.
- It includes real developer surfaces: web preview, media, icons, fonts, shadcn/ui components, sources, project checks, deploy state, Forge, Drive, and task workflows.

Example demo story:

- A frontier model can extract audio from video by reasoning through terminal commands.
- DX can give a smaller local model a direct media tool backed by ffmpeg, so the model only has to understand the user's intent and select the right tool.
- This makes local AI feel smarter because the product carries the mechanical expertise.
- DX can make that action safe by storing a Forge/zstd backup or receipt before risky file operations.

## Product Pillars

### 1. Native Editor First

- Keep editor typing, navigation, panes, and screen switching fast.
- Keep the coding screen full-width by default.
- Use optional left/right rails only when they help the current task.
- Do not add decorative panels that are disconnected from real commands or data.

### 2. Direct Local Model Runtime

- Use direct llama.cpp integration where it avoids unnecessary RPC and sidecar overhead.
- Auto-select model size and quantization based on the user's OS, RAM, VRAM, and hardware.
- Keep small, fast models for tool routing and quick edits.
- Use stronger local models on capable Mac, Linux, and high-end Windows machines.
- Fall back to remote models only when the local model is not the best fit.

### 3. Universal Provider Catalog

- Integrate providers from Flow, `zeroclaw-providers`, models.dev, LiteLLM-style aliases, OpenRouter, Ollama-compatible sources, local llama.cpp models, and user auth profiles.
- Store the provider/model catalog as a versioned `rkyv` archive loaded with `memmap2`.
- Keep JSON parsing and network catalog refreshes off the editor startup path.
- Show free-tier, premium-account, local, remote, context-window, tool-use, image, audio, video, coding, and cost hints.

### 4. Tool-Boosted Small Models

- Give local models deterministic tools for daily work.
- Examples: ffmpeg media manipulation, project search, metasearch, code health checks, source ingestion, deploy status, Git/Forge actions, file transforms, OCR, transcription, screenshots, web preview, and structured editing.
- Use tool schemas compacted by serializer so the model gets more useful capability with fewer tokens.
- Keep every tool permissioned, auditable, and reversible where possible.
- Prefer reversible Forge-backed actions over permanent deletion or silent mutation.

### 5. Source-Grounded Agent Workspace

- Add a left Sources rail inspired by NotebookLM.
- Let users attach docs, files, URLs, PDFs, notes, prompts, screenshots, media metadata, and project references to agent tasks.
- Store source summaries with content hashes and Forge snapshot IDs.
- Use RLM and serializer to reduce context without losing citations.

### 6. Real Project Operations

- Add Forge, Drive, Check, and Deploy as real panels.
- Forge handles code/media versioning and multi-remote sync.
- Drive handles sources, markdown specs, task docs, and agent context packs.
- Check scores project quality and shows concrete blockers.
- Deploy shows CI/CD readiness, env status, preview URLs, production status, logs, rollback, and receipts.

## Architecture Map

### `dx_catalog`

Purpose: zero-copy provider/model catalog.

Inputs:

- Flow local model roles and local runtime data.
- `G:\Dx\dx-agents\crates\zeroclaw-providers`.
- models.dev metadata.
- LiteLLM-style provider aliases.
- OpenRouter/Ollama-compatible public model lists where available.
- Local llama.cpp model scans.
- Auth profile state without raw secret exposure.

Output:

- Versioned binary catalog artifact.
- `rkyv` archived structs validated with bytecheck.
- `memmap2` read path for editor startup.
- Read-only catalog service for model picker and routing.

### `dx_ai_tools`

Purpose: AI panel tool bridge.

Capabilities:

- Metasearch.
- RLM reduction.
- Serializer packing.
- Provider routing.
- Media manipulation.
- Project inspection.
- Forge/Drive/Check/Deploy tool calls.

### `dx_forge`

Purpose: code and media version control.

Capabilities:

- Source code, audio, video, images, 3D assets, documents, datasets, databases, project files.
- Content-addressed chunks.
- Multi-remote sync.
- GitHub, GitLab, Bitbucket, R2/S3, Google Drive, Dropbox, YouTube, Sketchfab, SoundCloud, local/network mirrors.
- Restore plans, conflict warnings, receipts, and sync jobs.

### `dx_drive`

Purpose: source library and markdown task UI.

Capabilities:

- Sources rail.
- Markdown specs and task documents.
- Source sets for agents.
- Durable source snapshots through Forge.
- Serializer/RLM summaries.

### `dx_check`

Purpose: project quality and readiness scoring.

Capabilities:

- Code smell.
- Oversized files.
- Folder structure.
- Formatting/lint/typecheck/build status imports.
- Test freshness.
- Dependency/security posture.
- Visual proof receipts.
- Deploy readiness.

### `dx_deploy`

Purpose: CI/CD and shipping panel.

Capabilities:

- Vercel, Cloudflare, GitHub Actions, local scripts, Docker, custom CI/CD.
- Env readiness.
- Logs.
- Preview and production URLs.
- Rollback.
- Release receipts.

### `dx_dcp`

Purpose: native DX capability protocol.

Capabilities:

- Bridge DCP, MCP, ACP, Codex/Claude Code style tools, local Rust tools, browser tools.
- One permission model.
- One schema and receipt format.
- Serializer-packed tool catalogs.

## Feature Tracker

Scale: 0 means not started in the Zed fork. 100 means production-ready inside DX/Zed with docs, UI, permissions, checks, and git-backed status.

| Feature | Current Status | Target 100 Definition | Next Action |
| --- | ---: | --- | --- |
| Screen Dock Carousel | 85/100 | Smooth, full-width screen switching with polish, reduced motion, persistence, and no layout regression | Add spring polish after current feature batch |
| Root product plan backup | 100/100 | Canonical root plan plus detailed roadmap and launch thesis are committed | Keep updated as architecture changes |
| G-drive rebuildable cleanup | 100/100 | Flow Cargo target outputs cleaned without deleting source or models | Repeat only when space drops |
| `dx_catalog` provider/model archive | 100/100 | `rkyv` + `memmap2` catalog loader/generator powers model picker, routing, source materialization, and approved Agent artifact generation | Keep stable while provider registration approvals are wired |
| Universal provider routing | 76/100 | Local, remote, free-tier, premium, and fallback routes work from one catalog, with approved catalog specs writing into native Zed language-model settings, read-only Agent validation for native settings/runtime readiness, and an explicit permissioned Agent registration tool | Continue serializer/RLM execution integration and cross-panel routing |
| Metasearch AI tool | 78/100 | Agent panel can search many engines, inspect service/engine readiness, return token-aware cited source packs, persist managed source-pack receipts, fetch bounded readable extracts, prepare compact context bundles, create approved serializer/RLM execution-plan receipts, hand ready gates into deterministic reduced-context receipts, and surface source/reduced-context receipts in rails | Add runtime proof and richer source-row actions |
| Serializer/RLM prep pipeline | 88/100 | Metasearch source packs, source-pack attachment receipts, and deep extracts can be compacted into a citation-preserving context receipt, produce execution-plan receipts, pass through an explicit runner gate that separates external execution approval from RLM model-call approval, write deterministic reduced-context receipts, surface those receipts in launch rails, prepare guided execution-guard drafts, write dry-run external reducer previews, and run approved no-shell external reducer command vectors with managed receipts | Add governed runtime reducer proof when a reviewed binary/CLI contract is available |
| Forge safety and backup policy | 88/100 | Risky actions can produce permissioned no-permanent-delete Forge/zstd safety-policy receipts, validate reviewed backup/quarantine runner readiness, execute a native zstd backup bundle plus manifest before target mutation, restore that backup into a managed verified preview with receipts, expose Forge receipt history in panel rails, draft non-mutating restore approval reviews, and capture restore approval evidence as managed receipts | Add broader move/overwrite coverage and explicit restore-to-target flow after preview audit |
| Forge panel | 38/100 | Code/media snapshots, remotes, sync plans, jobs, restore warnings, approval handoffs, restore approval receipts, and receipt history are visible through panel-facing contracts; restore preview source rows now expose blocker/risk labels from restore receipts | Add restore approval drilldowns and reducer dry-run preview rows |
| Drive/Sources rail | 72/100 | NotebookLM-style source sets and markdown task docs feed agents through rail-visible source sets plus managed Agent attachment receipts, attach-ready counts, source-derived prompt cards, row-level Attach/Review controls, typed receipt metadata, row-level receipt review handoffs, and produced-file proof rows for output existence, receipt, hash, and empty-file warnings | Add richer source grouping and selection |
| Check panel | 44/100 | Project score and blockers include a typed read-only score schema for workspace structure, receipt root/file state, attach-ready sources, tool proof receipts, deploy target presence, deploy readiness receipts, env/log/rollback receipt inputs, URL/status deploy receipts, validation/visual/runtime proof freshness with compact latest-receipt drilldowns and prompt context, live receipt-review prompt context, runtime proof drafts with Check score/blocker/receipt/deploy context, a dedicated runtime proof import handoff, content-aware runtime proof import/status claim readiness, runtime proof plan/import/status receipts with plan/import separation, restore approval receipts, reducer execution-preview/external-execution receipts, and background-task state | Add broader validation categories and final governed proof evidence |
| Deploy panel | 24/100 | CI/CD readiness, env state, URLs, logs, rollback, receipts visible; workspace deploy target detection recognizes Vercel, Netlify, Cloudflare Wrangler, Fly, and Docker config files, and the rail now summarizes readiness/env/logs/rollback plus URL/status receipt buckets under `tools/dx-deploy` with freshness states and content-aware latest receipt drilldowns | Add governed deploy/runtime proof evidence |
| DCP bridge | 0/100 | DCP/MCP/ACP/local tools share one capability, permission, and receipt model | Define minimum DCP schema |
| Media tool bridge | 80/100 | Agent can plan safe ffmpeg/ffprobe inspect/extract actions, validate approved runner readiness, execute approved no-shell ffmpeg/ffprobe argument vectors, hash produced files, persist managed execution receipts, expose produced files as durable source entries with proof rows, and advertise guided media proof actions | Add runtime media proof |
| Codex-style rails | 74/100 | Left Sources and right project/task rail are optional, cheap when closed, backed by receipt-producing Agent actions, and now include attachment readiness, Check score surfaces, validation/visual/runtime proof freshness with compact latest-receipt drilldowns and prompt context, content-aware runtime proof status, live receipt-review drafts, runtime proof drafts with Check score/blocker/receipt/deploy context, a dedicated runtime proof import handoff, runtime proof plan/import/status receipts with honest readiness/proof separation, restore warnings, restore approval receipts, reducer execution-preview/external-execution receipts, deploy target visibility, source/deploy action prompts, row-level source controls, typed source receipt metadata and review handoffs, compact deploy proof rows, URL/status deploy receipts, content-aware deploy receipt drilldowns, runtime-proof handoff cards, restore approval drafts, reducer guard drafts, and produced-file proof rows | Add governed runtime proof evidence visibility |
| Launch demo package | 95/100 | May 22 sprint demos show speed, local model tools, provider freedom, metasearch, receipt chains, panels, guided proof drafts, source readiness, row-level source controls, typed receipt metadata, row-level receipt review handoffs, Check scoring, validation/visual/runtime proof freshness drilldowns, content-aware runtime proof claim readiness, live receipt-review drafts, runtime proof drafts with live Check/receipt/deploy context, a dedicated runtime proof import handoff, runtime proof plan/import/status receipts, restore warnings, deploy target visibility, source action prompts, deploy readiness receipts, deploy env/log/rollback receipt summaries, URL/status deploy summaries, content-aware deploy receipt drilldowns, runtime-proof handoffs with current proof context, restore approval capture, reducer dry-run previews, governed reducer external-execution receipts, and produced-file proof rows | Add governed runtime proof import from manual validation |

Overall implementation status: 100/100 for the completed launch-spine set.

Current next 100-point feature set status: 99.98/100 for DX Native Tool Execution, Restore, Panels, and Launch Demos.

Overall planning and product direction status: 100/100 for the current roadmap.

## Build Order To Reach 100

1. `dx_catalog`
   - archived structs,
   - generator,
   - last-good fallback,
   - model/provider picker integration.

2. AI panel provider routing
   - free/premium/local route display,
   - auth profile summary,
   - role-based routing,
   - cost and quota hints.

3. Metasearch tool
   - cancellable background searches,
   - cited result cards,
   - code/docs/news/web/media categories,
   - engine health and fallback.

4. Serializer/RLM context pipeline
   - compact tool catalog,
   - source pack summaries,
   - cached reductions,
   - citation preservation,
   - approved runner gate,
   - reduced-context receipt writer before model-call execution,
   - external reducer execution guard review,
   - dry-run external reducer execution preview receipts,
   - approved no-shell external reducer execution receipts under managed roots.

5. Media tools
   - ffmpeg extract audio,
   - convert media,
   - trim media,
   - inspect metadata,
   - no-shell execution receipts,
   - safe output path receipts,
   - produced-file panel/source rail rendering.

6. Forge safety policy
   - no permanent delete by default,
   - permissioned safety-policy receipt,
   - reviewed backup/quarantine runner gate,
   - native zstd backup execution,
   - managed restore preview execution,
   - zstd backup/quarantine path,
   - manifest and restore receipt requirements,
   - visible risk confirmations,
   - non-mutating restore-to-target approval review,
   - receipt-backed restore approval capture,
   - next target: reducer dry-run preview and broader mutation coverage.

7. Forge panel
   - snapshot status,
   - media-aware diffs,
   - remote plans,
   - job receipts,
   - receipt history contract,
   - right-rail workspace tool history,
   - restore preview.

8. Drive/Sources rail
   - source sets,
   - workspace roots,
   - metasearch source-pack receipts,
   - produced media files,
   - Forge restore previews,
   - markdown tasks,
   - project memory packs,
   - attach sources to agent tasks,
   - managed source attachment receipts,
   - source attachment context handoff,
   - media output and restore-preview source attachments.

9. Check panel
   - score schema,
   - file/folder structure review,
   - imported check status,
   - visual proof receipts,
   - runtime proof receipts,
   - runtime proof plan receipt capture,
   - runtime proof import/status receipt capture,
   - recommended fixes.

10. Deploy panel
   - target registry,
   - env readiness,
   - CI/CD logs,
   - preview/production URLs,
   - URL/status receipt buckets,
   - rollback receipts.

11. DCP bridge
    - minimum capability schema,
    - MCP/ACP adapters,
    - permission policy,
    - receipts.

12. Launch polish
    - full-width default workflow,
    - left/right rails,
   - demo workspaces,
   - machine-readable demo recipes,
   - runtime proof plan recipe,
   - runtime proof import recipe,
   - restore approval capture recipe,
   - serializer/RLM execution preview recipe,
   - serializer/RLM governed external execution recipe,
   - website/copy,
   - launch video scripts.

## May 22 Sprint Launch Plan

### Demo 1: Local Model Speed

Show the same prompt through common local-model workflows and DX direct runtime. The point is not only raw tokens per second; it is lower friction, faster task completion, and fewer manual steps.

### Demo 2: Small Model With Powerful Tools

Use a small local model to complete a task that usually makes small models fail:

- extract audio from a video,
- summarize the transcript,
- create a source card,
- attach it to a task,
- cite the output.

The model succeeds because DX provides exact media and source tools.

### Demo 3: Provider Freedom

Show one model picker that includes:

- local models,
- premium accounts,
- free-tier providers,
- OpenAI-compatible endpoints,
- provider capabilities,
- route recommendations.

### Demo 4: Metasearch For Agents

Show the AI panel searching many online places at once, returning cited compact results, then using RLM/serializer to keep the context cheap.

### Demo 5: Project Console

Show Forge, Drive, Check, and Deploy as the right product direction:

- source context,
- project score,
- media/code versioning,
- deployment readiness,
- receipts.

## Testing And Verification Policy

During feature build-out:

- Prefer small source checks and targeted inspections.
- Do not run repeated full builds after tiny edits.
- Do not run `just run` unless a coherent milestone needs runtime proof.
- Do not use repeated heavy Cargo commands in the Zed repo.
- Use `git diff --check`, targeted conflict-marker searches, rustfmt only on touched Rust files, and narrow checks that catch the likely integration issue.

At milestone boundaries:

- run the repo-approved runtime check,
- capture what passed,
- commit only coherent changes,
- push `dev`.

## Disk And Workspace Policy

- Keep the active Zed checkout at `G:\Zed` for launch stability, exposed through `G:\Dx\zed`.
- Keep Flow and DX tools on G drive, exposed through the `G:\Dx` launch hub.
- Clean only rebuildable outputs when G drive becomes tight.
- Do not delete source code, models, provider integrations, docs, or hand-authored assets.
- Safe rebuildable cleanup candidates include Cargo `target` directories and dedicated Cargo target caches such as `G:\.flow-cargo-target`.

Cleanup completed on 2026-05-20:

- removed `G:\Workspaces\flow\target`,
- removed `G:\Workspaces\flow\forge\target`,
- removed `G:\Workspaces\flow\providers\target`,
- removed `G:\.flow-cargo-target`,
- preserved source code, models, docs, and DX tools.

## Business Reality Check

This idea can become a major company if execution is exceptional. The strongest reasons:

- Zed gives a high-performance native base.
- Direct local model execution is a real speed wedge.
- Tool-boosted small models can beat much larger models on practical workflows when the tool is precise.
- Provider aggregation can save users money and reduce friction.
- Developers are unhappy when editors become slow or agent-first instead of editor-first.
- A viral launch is plausible if the demos clearly show faster local AI plus successful tool use.

But the product will not win from vision alone. The hard requirements are:

- trustworthy UX,
- no dummy UI,
- reliable local model setup,
- clear permission boundaries,
- excellent demos,
- fast onboarding,
- strong docs,
- continuous upstream sync,
- and a polished enough first release that users can feel the speed advantage immediately.

The plan is bold, but the smart path is practical: build the provider catalog first, then metasearch and tool bridges, then Forge/Drive/Check/Deploy panels, then launch with the strongest demos.

## Focus Mantra

Do not get lost in the size of the vision. The path is simple:

- Make AI faster.
- Make AI cheaper.
- Make AI safer.
- Give small models powerful tools.
- Give every risky action a backup.
- Keep developers in control.
- Ship the sprint demo on May 22.

If those seven things are true, DX is not just another editor fork. It is a new kind of developer workstation.
