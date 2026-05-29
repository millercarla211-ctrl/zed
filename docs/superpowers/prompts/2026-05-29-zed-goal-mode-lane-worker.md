# Zed Goal Mode Lane Worker Prompt

Paste this prompt into each of the 6 Zed worker chats. Every worker runs the same prompt. The allocator assigns a lane by counter and prints the exact `PLAN.md` tasks for that lane.

```text
Use [@superpowers](plugin://superpowers@openai-curated) first.

You are a Codex Desktop GPT-5.5 extra-high implementation worker for G:\Dx\zed. Use exactly 6 GPT-5.5 extra-high subagents inside your assigned lane.

Repo: G:\Dx\zed
Plan file: G:\Dx\zed\PLAN.md
Lane allocator: G:\Dx\zed\scripts\codex\claim-zed-plan-lane.ps1

Goal Mode setup:
- Create a goal for this lane.
- Token budget: unlimited / no cap. If the UI requires a number, use the maximum available.
- Time budget: unlimited / no cap. Keep working until the lane is complete or honestly blocked.
- Reasoning: GPT-5.5 extra-high. Think very hard about architecture, correctness, maintainability, GPUI/Zed patterns, and regression risk.
- Do not stop after planning. Continue lane allocation, implementation, source review, lightweight verification, commit, and honest final report.

Goal objective:
Complete my assigned G:\Dx\zed PLAN.md lane end-to-end with 100/100 production-ready, professional, maintainable code, using exactly 6 GPT-5.5 extra-high subagents inside my lane, source-inspection-first verification, no broad/heavy builds, and an honest final report.

First action inside the goal:
Run this exact command before source edits:

powershell -NoProfile -ExecutionPolicy Bypass -File "G:\Dx\zed\scripts\codex\claim-zed-plan-lane.ps1"

The script assigns your lane and prints your exact PLAN.md tasks. Work only those tasks.

If the script prints generatedAgentId: True, copy the printed resumeCommand and use that exact command for every future allocator run in this same worker chat.

If the script says PLAN.md is missing or no tasks were parsed:
- Do not invent tasks.
- Report NEEDS_CONTEXT with the exact script output.
- Wait for the coordinator to provide the correct PLAN.md or PlanPath.

Required Superpowers workflow:
- Use Superpowers:using-git-worktrees or explicitly verify branch/worktree safety before edits.
- Use Superpowers:writing-plans for a lane-local implementation plan.
- Use Superpowers:subagent-driven-development to coordinate exactly 6 GPT-5.5 extra-high subagents inside your lane.
- Use Superpowers:verification-before-completion before claiming done.
- Use Superpowers:requesting-code-review for risky or broad changes.

Subagent requirement:
- Use exactly 6 GPT-5.5 extra-high subagents.
- Keep all subagents strictly inside your assigned lane tasks.
- Give each subagent isolated, non-overlapping scope.
- Do not let subagents touch tasks from other lanes.

Zed repo rules:
- Read G:\Dx\zed\AGENTS.md and follow it.
- Read the relevant parts of G:\Dx\zed\DX.md, G:\Dx\zed\PLAN.md, G:\Dx\zed\todo.txt, and G:\Dx\zed\changelog.txt before source edits.
- Preserve existing Zed behavior unless your PLAN.md lane explicitly changes it.
- Keep todo.txt and changelog.txt current if you implement source changes.
- Do not run just run unless the coordinator/user explicitly authorizes it.
- Do not run cargo test, cargo check, cargo build, cargo clippy, just check, just build, just fmt, or just lint unless explicitly authorized.
- Start with rg/source scans, targeted file reads, and code review.
- Use git diff --check as the normal lightweight verification.

Quality bar:
- Complete your lane as 100/100 production-ready professional code.
- No dummy UI, fake state, fake wiring, or decorative changes.
- Keep files small, focused, typed, and consistent with existing Zed/GPUI patterns.
- Preserve unrelated work.
- Commit only your lane changes.

Verification style:
- Code-heavy and source-review-first.
- Use focused inspections and targeted checks only.
- Avoid broad/heavy commands.
- If a task requires runtime proof, mark it honestly as needing authorized validation instead of running forbidden commands.

Keep working until:
- All assigned PLAN.md lane tasks are implemented or honestly blocked.
- Lightweight checks are run or explicitly skipped with reason.
- Lane changes are committed.
- Final report is honest about what is fully wired, partial, risky, or blocked.

Final response format:
Status: DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
Goal:
Lane:
Task range:
Tasks completed:
6 subagents used:
Files changed:
Focused checks run:
Fully wired:
Preview-only or incomplete:
Risks:
Commit:
Next exact step:
```
