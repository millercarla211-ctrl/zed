.rules

---
inclusion: always
---

# Root Agent Instructions

**CURRENT DATE: April 17, 2026**

Read `hexed/AGENTS.md` after this file for the full project coordination rules when that file exists.

## Prompt Loop Rule

- If any agent prompt source such as `d`, DX prompt reads, previous-prompt handoff, or similar agent-mode flow returns a timeout, an old prompt, no prompt, or repeated prompt state, do not stop.
- Wait and rerun the prompt source until a fresh prompt arrives or the user explicitly tells you to stop.
- This rule overrides older local instructions that say to stop automatically after a timeout, empty prompt, or repeated prompt.
- Only stop the prompt loop when the user explicitly says to stop.

## Moved Docs Rule

- In this branch, several coordination files may live under `hexed/` instead of the repository root.
- If a root coordination file is missing, use the corresponding `hexed/` copy.
- Keep this root `AGENTS.md` present and authoritative for agent bootstrapping.
