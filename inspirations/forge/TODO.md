# Forge TODO

## Current Focus

- validate the new transport-backed repository sync path against broader live sync/mirror execution flows
- deepen resumable recovery from per-file retry into chunk/session-level transfer resume
- keep Forge library-first so other Rust hosts can embed it without depending on the CLI
- close validation and hardening gaps before adding wider remote surface area

## Core Model

- expose a stable remote and sync-plan model, not just per-command orchestration
- add richer artifact identities for publish mirrors and non-restorable media platforms
- model current snapshot state and historical mirror runs under one public API

## Multi-Remote Sync

- deepen the new live branch/ref sync execution beyond the current `forge sync run` path
- enforce more per-remote branch/ref policy during live push/pull, not only in the current conflict layer
- add richer conflict reports and remediation hints before execution
- add remote capability discovery and per-remote policy
- add explicit restore/source preference when more than one restorable mirror exists

## Media Workflows

- add metadata extraction and preview descriptors for audio, video, and 3D assets
- separate publish-only backends from byte-restorable backends in the public API
- turn the new large-file Dropbox session path into a durable resumable job with checkpoints
- add artifact manifests for reproducible code and media restores

## Auth And Secrets

- add provider-neutral credential metadata and scope tracking
- add remote account registry and approval boundaries
- add local-only secret policy docs and redaction support
- expand auth health/status inspection beyond the new sync-status CLI view

## Jobs And Recovery

- expand the new durable job layer beyond the new in-place retry path into retrying verify and cleanup jobs too
- extend the new retry backoff scheduling into richer operator controls and retry windows across more job kinds
- deepen resumable checkpoints from per-file push/pull recovery into large-transfer chunk/session checkpoints
- turn mirror-run receipts, jobs, and structured restore locators into a real recovery substrate
- add health/status reporting for remotes and active jobs

## Library API

- harden the new sync-overview, remote-registry, sync-execution, and job-inspection APIs
- keep the new recovery and remote-health APIs stable enough for embedders
- keep the new framed transport and QUIC bootstrap APIs stable enough for embedders
- keep the new transport-backed commit push/pull APIs stable enough for embedders
- keep host surfaces independent from one UI
- make it reusable in `dx`, editors, creative tools, and future shells
- keep the CLI as a thin layer over the library APIs

## Validation

- expand the revived integration suite into broader mirror, pull, and recovery coverage
- add tests for commit/delete/checkout correctness and mirror snapshot replacement
- add validation for media manifests, checksum stability, and restore integrity
- run a full compile/fix pass once the machine can tolerate cargo commands again
