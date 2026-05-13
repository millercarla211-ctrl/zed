# Flow 100 Plan

This document is now a historical completion plan. The current repository scope has been validated to a `100/100` release-ready state; the milestones below remain useful as expansion guidance for future host depth and product uplift.

## What 100 Means

Flow reaches `100` when all of these are true:

- system-wide dictation feels production-grade
- grammar and rewrite assistance are competitive with Grammarly on daily writing tasks
- editor assistance is competitive with Wispr Flow on coding-heavy workflows
- local-first runtime routing is reliable on low-end and mid-tier hardware
- wake-word and hotkey activation are stable across supported desktop hosts
- AI-driven OS control is safe, auditable, and actually useful
- Flow can act as a reusable Rust library instead of a one-off app

## Current High-Value Work

- activation and wake-word policy
- always-on runtime budgets
- proofing and rewrite product surfaces
- editor file-tagging and variable-recognition surfaces
- safe OS-control planning
- command routing plus approval and audit surfaces
- OS-aware automatic base-module bootstrap

## Remaining Milestones

### 1. Host Wiring

- connect hotkeys to real platform adapters
- connect wake profiles to model inference
- connect text insertion and selection replacement to real host surfaces
- connect install-state persistence and module transitions to the real storage/runtime layer
- connect `FlowEngine` to real host executors instead of recording adapters

### 2. Proofing Quality

- merge proofing with grammar outputs
- add stronger clarity rewrites
- add factual-claim checks and source attachment flows
- add academic review features

### 3. Runtime Maturity

- connect always-on plans to device profiling
- add real resident model admission and eviction rules
- finish local multimodal runtime adapters

### 4. Control Maturity

- add permission memory and host-scoped approvals
- add auditable action logs
- add richer app, file, and media control adapters

### 5. Product Polish

- overlay UX
- command mode UX
- low-end fallback UX
- battery and thermal UX
- first-run onboarding and permission UX
- recovery UX for suspend, resume, crash, and lost-microphone cases

## Rule For The Current Phase

Keep Flow core self-contained. Do not make the standalone Flow completion depend on the paused sibling projects.
