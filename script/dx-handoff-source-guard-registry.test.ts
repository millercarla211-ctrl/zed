import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const registeredGuardScripts = [
  "script/dx-handoff-source-guard-registry.test.ts",
  "script/dx-windows-reliability-source.test.ts",
  "script/dx-workspace-reentrant-source.test.ts",
  "script/dx-source-quality.test.ts",
  "script/dx-studio-project-source.test.ts",
  "script/dx-agent-panel-clipboard-source.test.ts",
  "script/dx-mention-set-source.test.ts",
  "script/dx-message-editor-source.test.ts",
  "script/dx-thread-metadata-source.test.ts",
  "script/dx-deploy-panel-source.test.ts",
  "script/dx-deploy-receipts-source.test.ts",
  "script/dx-deploy-launch-gate-source.test.ts",
  "script/dx-deploy-launch-evidence-source.test.ts",
  "script/dx-check-panel-source.test.ts",
  "script/dx-launch-workspace-source.test.ts",
  "script/dx-launch-audit-source.test.ts",
  "script/dx-launch-audit-fixtures.test.ts",
  "script/dx-launch-binary-cache-source.test.ts",
  "script/dx-launch-contracts-source.test.ts",
  "script/dx-launch-contracts-fixtures.test.ts",
  "script/dx-launch-prompts-source.test.ts",
  "script/dx-launch-readiness-source.test.ts",
  "script/dx-launch-readiness-fixtures.test.ts",
  "script/dx-launch-receipts-source.test.ts",
  "script/dx-launch-source-audit-source.test.ts",
  "script/dx-launch-status-source.test.ts",
  "script/dx-receipt-history-source.test.ts",
  "script/dx-runtime-proof-status-source.test.ts",
  "script/dx-source-sets-source.test.ts",
  "script/dx-agent-bridge-source.test.ts",
  "script/web-preview-payload-source.test.ts",
  "script/dx-www-launch-evidence-source.test.ts",
  "script/web-preview-platform-lifecycle.test.ts",
];

test("DX.md exposes the lightweight source guard registry", () => {
  const dx = read("DX.md");

  assert.match(dx, /## Lightweight Source Guard Registry/);
  assert.match(dx, /These guards are source-contract checks only\./);
  assert.match(dx, /do not prove native runtime behavior/);
  assert.match(dx, /Run the narrowest guard that matches the owned lane/);

  for (const guard of registeredGuardScripts) {
    assert.ok(dx.includes(guard), `DX.md should list ${guard}`);
  }
});

test("handoff docs keep source-only proof separate from runtime readiness", () => {
  const dx = read("DX.md");
  const agents = read("AGENTS.md");

  assert.match(
    dx,
    /Existing `100\/100`, "complete", and "production" notes in older handoffs mean source\/code-complete/,
  );
  assert.match(
    dx,
    /Do not claim runtime-green, production-ready, or launch-ready from these docs alone\./,
  );
  assert.match(dx, /no-`just run` and no-Cargo by direct instruction/);
  assert.match(
    agents,
    /current user prompt explicitly opens the final validation window/,
  );
  assert.match(agents, /source-only or release-hygiene passes/);
  assert.match(
    agents,
    /\*\*NEVER\*\* when the current user prompt or handoff lane forbids it/,
  );
});

test("current handoff names the no-runtime-proof production-readiness boundary", () => {
  const dx = read("DX.md");
  const todo = read("todo.txt");

  assert.match(dx, /Current production readiness is source-audited only/i);
  assert.match(
    dx,
    /Skipped by direct instruction: Cargo build\/check\/test\/clippy, `just run`, local servers, browser automation, and live editor runtime proof\./,
  );
  assert.match(todo, /Production-readiness source audit/);
  assert.match(todo, /Skipped by direct instruction: Cargo build\/check\/test\/clippy, `just run`, local servers, browser automation, and live editor runtime proof\./);
});
