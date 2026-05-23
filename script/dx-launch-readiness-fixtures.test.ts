import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const fixtureRoot =
  process.env.DX_LAUNCH_EXAMPLES_ROOT ?? "G:/Dx/cli/fixtures/launch-examples";
const sensitiveRedactionFields = [
  "exports_source_file_contents",
  "exports_source_file_paths",
  "exports_secret_values",
  "exports_receipt_bodies",
  "exports_prompts",
  "exports_transcripts",
  "exports_command_payloads",
];

const families = {
  "import-summary": {
    schema: "dx.launch.import_summary.v1",
    files: [
      "import-summary-ready.json",
      "import-summary-warning.json",
      "import-summary-blocked.json",
    ],
    recoveryCommands: ["startup", "refresh", "release_gate", "receipts"],
  },
  "release-gate": {
    schema: "dx.launch.release_gate.v1",
    files: [
      "release-gate-fresh.json",
      "release-gate-stale.json",
      "release-gate-expired.json",
      "release-gate-malformed.json",
      "release-gate-missing.json",
    ],
    recoveryCommands: [],
  },
  "fallback-drill": {
    schema: "dx.launch.fallback_drill.v1",
    files: [
      "fallback-drill-ready.json",
      "fallback-drill-warning.json",
      "fallback-drill-blocked.json",
    ],
    recoveryCommands: ["refresh", "release_gate", "receipts", "import_summary"],
  },
};

const readJson = (fileName: string) =>
  JSON.parse(readFileSync(path.join(fixtureRoot, fileName), "utf8"));

const nestedCommandFanoutCount = (value: unknown): number => {
  if (Array.isArray(value)) {
    return value.reduce((total, item) => total + nestedCommandFanoutCount(item), 0);
  }

  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    const here = record.command_fanout === true ? 1 : 0;
    return (
      here +
      Object.values(record).reduce(
        (total, item) => total + nestedCommandFanoutCount(item),
        0,
      )
    );
  }

  return 0;
};

test("DX launch readiness fixtures stay schema-safe and metadata-only", () => {
  assert.ok(existsSync(fixtureRoot), `missing launch readiness fixture root: ${fixtureRoot}`);
  const actualFiles = new Set(readdirSync(fixtureRoot));

  for (const [family, contract] of Object.entries(families)) {
    const seenStatuses = new Set<string>();

    for (const fileName of contract.files) {
      assert.ok(actualFiles.has(fileName), `missing ${family} fixture ${fileName}`);
      const packet = readJson(fileName);

      assert.equal(packet.schema_version, contract.schema, `${fileName} schema changed`);
      assert.equal(packet.no_command_fanout, true, `${fileName} should remain metadata-only`);
      assert.equal(
        nestedCommandFanoutCount(packet),
        0,
        `${fileName} should not declare command fanout`,
      );
      assert.equal(typeof packet.next_action, "string", `${fileName} needs an explicit next action`);
      assert.notEqual(packet.next_action.trim(), "", `${fileName} next action cannot be blank`);
      seenStatuses.add(packet.status);

      for (const field of sensitiveRedactionFields) {
        assert.equal(
          packet.redaction?.[field],
          false,
          `${fileName} redaction ${field} must stay false`,
        );
      }

      for (const commandKey of contract.recoveryCommands) {
        assert.equal(
          typeof packet.recovery_commands?.[commandKey],
          "string",
          `${fileName} missing recovery command ${commandKey}`,
        );
      }
    }

    assert.ok(seenStatuses.has("ready"), `${family} fixtures need a ready example`);
    assert.ok(
      seenStatuses.has("warning") || seenStatuses.has("blocked"),
      `${family} fixtures need a non-ready example`,
    );
  }
});
