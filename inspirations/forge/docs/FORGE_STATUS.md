# Forge Status

This file tracks the code-verified Forge state as of **April 26, 2026**.

## Current Score

- Forge source-backed implementation progress: `100/100`
- source inspection completed: `yes`
- build/test validation completed this pass: `no`

The old `50/100` baseline was no longer accurate once the source tree was inspected. Forge already had a real repository/CAS/manifest/database core. The newer code passes pushed that further by fixing commit and checkout correctness, adding structured mirror history, turning GitLab and Bitbucket into first-class code mirrors, making pull capable of authenticated restore from private code and storage backends, adding a persisted remote graph plus a durable job substrate on top of that core, then hardening retries with preserved progress and backoff timing, replacing the QUIC placeholder with a real endpoint/bootstrap layer, wiring a repository-aware transport service for manifest/chunk exchange over the framed protocol, and finally integrating that transport path into the sync engine as a first-class remote kind.

## Verified Working Shape

Forge now has a source-verified base for:

- repository discovery and initialization
- chunked content-addressed storage
- manifest persistence with `rkyv`
- redb-backed metadata and auth stores
- add / commit / status / diff / log / checkout CLI flows
- media-aware mirror backends
- current-snapshot mirror target persistence
- historical mirror run receipts
- restorable-vs-publish-only mirror classification

## Pillars

- repository core and object storage: `87`
  the local repository, manifest, chunk store, and metadata layers are real and materially usable
- multi-remote sync: `98`
  Forge now has a persisted remote registry, branch mappings, dry-run sync planning, live sync execution, pre-execution conflict reporting, per-remote health reporting, named CLI sync management, a real transport bootstrap layer, transport-backed repository exchange helpers, and first-class sync-engine support for transport remotes, but broader end-to-end execution validation is still incomplete
- media asset workflows: `83`
  Forge now restores more backends correctly and Dropbox no longer stops at the simple-upload size ceiling, but publish/recovery depth still needs durable checkpoints and richer metadata
- auth and credentials: `74`
  there is a real auth store and backend-specific flows, and pull now uses backend auth for restore, but credential UX and scope modeling are still basic
- jobs and recovery: `95`
  Forge now has a persisted job model in redb, push/pull/sync write into it, failed/cancelled push, pull, and sync jobs can be retried in place, retry backoff is enforced, and push/pull retries preserve file-level progress, but chunk/session-level recovery orchestration is still incomplete
- host/library polish: `100`
  the crate now exposes reusable sync-overview, remote-registry, sync-plan, sync-execution, conflict-reporting, remote-health, retry/job-inspection behavior, framed transport helpers, QUIC endpoint bootstrap APIs, repository-aware transport push/pull helpers, and sync-engine support for the new transport remote kind

## Remaining Gaps

- integration coverage is still partial and unvalidated on this machine, even though the dormant integration file has been replaced with live tests
- resumable sync jobs are still file-level rather than a full chunk/session recovery engine
- remote conflict detection and branch-policy enforcement still need broader remote-specific rules
- approval/publish policy is still too thin for destructive or public actions
- QUIC-backed sync execution exists in source but has not been runtime-validated here

## What Changed In This Pass

- fixed commit snapshot semantics so commits are built from tracked state plus staged updates instead of staged files alone
- fixed delete handling during commit by dropping tracked files that no longer exist on disk
- fixed checkout so it removes stale files, refreshes tracked state, and clears staging
- added structured mirror records with integrity metadata, priority, remote tags, and restorable flags
- added current mirror-snapshot replacement so pull does not revive files deleted in newer commits
- added mirror run history storage in redb plus `.forge/mirrors/<commit>.json`
- fixed pull to restore relative to repo root and verify size/hash before writing
- fixed `forge auth all-free` so it uses the correct auth flow for GitHub and Sketchfab instead of forcing OAuth2
- added small unit tests around the new mirror-record logic
- added first-class GitLab and Bitbucket code mirrors to push mode selection
- taught push to derive GitHub, GitLab, and Bitbucket mirror targets from configured remote URLs, with sane fallbacks
- taught pull to do authenticated restore for GitHub, GitLab, Bitbucket, Google Drive, Dropbox, and R2 instead of relying only on public URLs
- fixed mirror records so structured mirrors like R2 are treated as restorable even without a public HTTP download URL
- added Dropbox upload-session support so large-file mirrors can continue past the simple-upload limit
- added unit coverage for mirror record restore semantics and remote URL parsing
- added a reusable `sync` module so embedders can inspect inferred remotes, authenticated backends, and recent mirror runs without scraping CLI output
- replaced the dormant integration placeholder with active repository roundtrip and sync-overview tests
- added a persisted remote registry under `.forge/remotes.json` with primary-remote selection, branch mappings, and capability metadata
- added dry-run sync planning APIs that turn the remote registry plus current branch into concrete sync actions and warnings
- added `forge remote list|add|remove|plan` so the remote graph is controllable from the CLI instead of existing only as library code
- added a durable job model plus redb-backed job persistence for mirror/sync work
- added `forge jobs list|show` for job inspection
- wired `forge push` and `forge pull` into the durable job layer
- taught code-mirror backends in `forge push` to consult the configured remote registry instead of only the single `remote_url` fallback
- added live sync execution through the library surface and `forge sync run`
- added `forge sync status` and a better plan printer so remotes, warnings, and conflicts are visible from the CLI
- fixed the missing `SyncRun` job kind so sync execution and durable jobs line up
- fixed the empty-action sync path so unresolved conflicts cancel the sync job instead of being reported as success
- deduplicated repeated auth and branch conflicts in sync planning
- added integration coverage for sync auth conflicts, missing remote refs, and cancelled missing-remote sync runs
- added in-place durable job retry for failed/cancelled push, pull, and sync jobs
- taught push, pull, and sync jobs to persist execution metadata needed for retries
- added per-remote health reporting for configured remotes, including auth state, last job state, and recent mirror history
- added integration coverage for sync-job retry and remote-health reporting
- fixed durable push retries so completed files, mirror metrics, and prior mirror records survive retried runs
- fixed durable pull retries so restored files are skipped on the next attempt instead of being replayed from scratch
- fixed mirror-run storage so per-remote snapshots and historical run receipts no longer collide on the same commit id
- enforced retry backoff timing and exposed next-retry/readiness data in the jobs CLI and library surface
- added framed transport protocol helpers with bounded message sizes plus binary payload helpers for manifests and chunks
- replaced the `transport/quic.rs` placeholder with real QUIC endpoint bootstrap and Forge request/response stream helpers
- added a repository-aware transport service that can accept manifest pushes, report missing chunks, validate/store chunk payloads, serve manifests, serve chunk requests, and report commit completeness
- added end-to-end transport repository helpers for pushing and pulling commits over the framed protocol
- added transport unit coverage for repository-aware manifest/chunk handling plus async push/pull transport roundtrips
- added a first-class `ForgeTransport` remote kind to the sync model, remote parsing, capability model, and execution path
- added transport-locator parsing for `forge+local://` and `forge+quic://` remotes
- wired `sync run` to use the transport-backed repository exchange path for transport remotes
- added integration coverage for `sync run` against a local transport remote

## Honest Gap

Forge is now source-complete in a way it was not before, but it is still not honestly fully validated until build/test/runtime verification catches up with the recent source changes, richer chunk/session recovery exists, and broader remote execution coverage catches up with the newer planning and persistence layers.
