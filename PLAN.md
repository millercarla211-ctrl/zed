# DX/Zed Launch Plan

Date: 2026-05-20
Launch target: 2026-05-28
Primary repo: `F:\Zed`
Supporting DX tools: `G:\Dx`, `G:\Flow`, `G:\Workspaces\flow`

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

The public launch story for May 28 should be:

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
| `dx_catalog` provider/model archive | 99/100 | `rkyv` + `memmap2` catalog loader/generator powers model picker, routing, and source materialization | Expose an approved command/settings trigger that writes the production artifact to the Agent-discoverable path |
| Universal provider routing | 50/100 | Local, remote, free-tier, premium, and fallback routes work from one catalog | Wire registration specs into provider settings after explicit approval |
| Metasearch AI tool | 10/100 | Agent panel can search many engines with cited compact results | Add Zed tool adapter around metasearch crates |
| Serializer/RLM prep pipeline | 10/100 | Tool catalogs, chats, sources, and search results compact before model calls | Define AI context packing boundary |
| Forge safety and backup policy | 10/100 | Risky actions create zstd/Forge backups, receipts, and restore paths instead of permanent loss | Define no-permanent-delete policy |
| Forge panel | 5/100 | Code/media snapshots, remotes, sync plans, jobs, and restore warnings visible | Add Forge host adapter plan and panel skeleton |
| Drive/Sources rail | 5/100 | NotebookLM-style source sets and markdown task docs feed agents | Define source set model |
| Check panel | 5/100 | Project score and blockers include structure, lint/format status, visual proof, deploy readiness | Define score schema and read-only scanner |
| Deploy panel | 0/100 | CI/CD readiness, env state, URLs, logs, rollback, receipts visible | Define deploy target registry |
| DCP bridge | 0/100 | DCP/MCP/ACP/local tools share one capability, permission, and receipt model | Define minimum DCP schema |
| Media tool bridge | 10/100 | Agent can manipulate audio/video/images through direct tools like ffmpeg | Start with safe ffmpeg actions |
| Codex-style rails | 5/100 | Left Sources and right project/task rail are optional and cheap when closed | Design rail state model |
| Launch demo package | 0/100 | May 28 demos show speed, local model tools, provider freedom, metasearch, and panels | Build 3 demo scripts |

Overall implementation status: 70/100.

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
   - citation preservation.

5. Media tools
   - ffmpeg extract audio,
   - convert media,
   - trim media,
   - inspect metadata,
   - safe output path receipts.

6. Forge safety policy
   - no permanent delete by default,
   - zstd backup/quarantine path,
   - Forge snapshot receipts,
   - restore commands,
   - visible risk confirmations.

7. Forge panel
   - snapshot status,
   - media-aware diffs,
   - remote plans,
   - job receipts,
   - restore preview.

8. Drive/Sources rail
   - source sets,
   - markdown tasks,
   - project memory packs,
   - attach sources to agent tasks.

9. Check panel
   - score schema,
   - file/folder structure review,
   - imported check status,
   - visual proof receipts,
   - recommended fixes.

10. Deploy panel
   - target registry,
   - env readiness,
   - CI/CD logs,
   - preview/production URLs,
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
    - website/copy,
    - launch video scripts.

## May 28 Launch Plan

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

- Keep Zed in `F:\Zed`.
- Keep Flow and DX tools on G drive.
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
- Ship the demo on May 28.

If those seven things are true, DX is not just another editor fork. It is a new kind of developer workstation.
