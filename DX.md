# DX/Zed Launch Worker Notes

Date: 2026-05-21
Launch target: 2026-05-22

## Worker Role

This checkout is the Zed/DX editor surface. Worker chats here should focus on GPUI integration, AI panel polish, DX Agents bridge hooks, token meters, source rails, progress rails, and launch-safe runtime wiring.

## Required Reading

- `G:\Dx\WORKER_PROMPTS.md`
- `G:\Dx\DX.md`
- `G:\Zed\AGENTS.md`
- `G:\Zed\PLAN.md`

## Hard Rules

- Use `[@superpowers](plugin://superpowers@openai-curated)` in new worker chats.
- Preserve existing Zed AI behavior.
- Do not create dummy UI.
- Do not start local servers.
- Do not run `just run`, full Cargo builds, or expensive workspace checks until the user opens the governed final validation window.
- Prefer code reading, targeted implementation, `git diff --check`, conflict-marker search, and narrow checks after coherent milestones.
- Update `todo.txt`, `changelog.txt`, and this file when changing launch status.

## Current Launch Targets

- Add `Agent` alongside existing AI actions like `Write` and `Ask`.
- Add left Sources rail and right Progress/Git/Background Tasks rail.
- Add New Chat, Search, Plugins, and Automations actions.
- Add Pinned and All Chats workspace groups.
- Wire DX token/RLM/serializer receipts into token/tool meters.
- Wire DX Agents CLI receipts into GPUI status surfaces.

## Current Worker Update

- Implemented the DX launch workspace chrome around existing Agent thread rendering when the panel is zoomed/full-workspace: left Sources/receipts rail, right Progress/Git/Background Tasks rail, and token/tool meter slots.
- Added receipt hooks for `G:\Dx\.dx\receipts` with graceful missing/empty states and no CLI execution from Zed.
- Added an Agent action beside the existing Write/Ask mode and model controls while preserving the current model picker, profile selector, token usage, and thread behavior.
- Added New Chat, Search, Plugins, and Automations sidebar actions plus Pinned/All Chats group headers; Automations routes to the existing project debug-task configuration until the DX automation receipt producer has a first-class Zed panel.
- Made the Agent panel default to full-width/zoomed when the workspace has no active editor item; user zoom toggles still override that default.
- Added `execute_dx_media_tool`, a permissioned Agent tool that consumes approved media runner gates, executes ffmpeg/ffprobe via no-shell argument vectors, refuses overwrites and path traversal, records stdout/stderr previews, hashes produced files, and writes managed DX media execution receipts.
- Added a cached workspace tool-history scanner for `tools/dx-forge` and `tools/dx-media/executions`, then rendered Forge History and Media Executions in the DX right rail with missing/empty states.
- Added the first durable Sources rail model for workspace roots, DX metasearch source-pack receipts, produced media files, and Forge restore previews.
- Added `prepare_dx_source_attachment`, a permissioned Agent tool that packages selected workspace roots, metasearch source-pack receipts, produced media files, and Forge restore previews into managed source attachment receipts for later Agent context.
- Wired `prepare_dx_metasearch_context` to accept source attachment manifests/receipts, embed metasearch source-pack receipt text into compact cited context, and warn on path-only media/restore references instead of embedding binary payloads.
- Added `list_dx_launch_demo_recipes`, a permissioned read-only Agent tool that lists metasearch-to-context, media-output-to-sources, and Forge-restore-preview-to-sources demo flows with required tool chains, receipt gates, safety notes, and workspace receipt-root status.
- Added `gate_dx_serializer_rlm_runner`, a permissioned Agent tool that validates serializer/RLM execution plans or receipts, requires explicit runner approval, requires persisted execution receipts by default, separates RLM model-call approval, and can write managed runner-gate receipts without running serializer/RLM code.
- Added `write_dx_serializer_rlm_reduced_context`, a permissioned Agent tool that consumes runner-gate receipts and context bundles, writes deterministic reduced-context receipts with cited source summaries, and does not run external serializer/RLM crates, Cargo, network, browser input, or model calls.
- Surfaced reduced-context receipts in the launch workspace: the Sources rail now has a Reduced Context set, and the right Tool History rail scans workspace `tools/dx-serializer-rlm` receipts.
- Added guided DX sidebar actions for Demo Recipe and Review Receipts; they prepare permission-safe Agent drafts for the flagship metasearch-to-reduced-context receipt chain without auto-running tools.
- Added right-rail Media Proof and Forge Proof cards that prepare permission-safe Agent drafts for proof flows without running tools automatically.
- Added source attachment readiness to the Sources rail and a typed read-only Check score to the right rail, both backed by actual workspace/source/receipt/tool-history state.
- Added restore-preview warning labels to Forge source rows and a read-only Deploy rail backed by detected workspace deploy config files.
- Added source-derived action prompt cards and deploy readiness receipt counts/latest entries, still drafting only permission-safe Agent follow-ups.
- Added Deploy rail receipt buckets for readiness, env, logs, and rollback under `tools/dx-deploy`, with compact missing/fresh/stale/old states and safer deploy-readiness prompts that include the bucket summary.
- Advanced the current DX Native Tool Execution/Restore/Panels/Demos set to 84/100; the next highest-value target is validation/visual proof receipt freshness plus produced-file proof cards.

## Remaining Proof

- Runtime visual proof is still pending because this launch lane forbids local servers, `just run`, full Cargo builds, and heavy validation without explicit permission.
- The DX CLI receipt producers remain external to Zed; this slice reads receipt files and reports missing or empty receipt states only.
- The media runner source slice has rustfmt/diff/conflict validation only so far; Cargo check/test and runtime ffmpeg proof remain deferred under the repo's launch validation rules.
- The reduced-context writer has formatting/diff/conflict validation only so far; Cargo check/test and runtime Agent proof remain deferred under the repo's launch validation rules.
- The reduced-context rail slice has formatting/diff/conflict validation only so far; runtime visual proof remains deferred under the repo's launch validation rules.
- The guided action slice has formatting/diff/conflict validation only so far; runtime click proof remains deferred under the repo's launch validation rules.
- The guided proof card slice has formatting/diff/conflict validation only so far; runtime click proof remains deferred under the repo's launch validation rules.
- The source attachment and Check score slice has formatting/diff/conflict validation only so far; runtime visual proof remains deferred under the repo's launch validation rules.
- The restore-warning and Deploy registry slice has formatting/diff/conflict validation only so far; runtime visual proof remains deferred under the repo's launch validation rules.
- The source action and deploy readiness receipt slice has formatting/diff/conflict validation only so far; runtime click proof remains deferred under the repo's launch validation rules.
- The deploy receipt bucket slice has formatting/diff/conflict validation only so far; runtime visual proof remains deferred under the repo's launch validation rules.
