import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
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
const requiredManifestCommands = [
  "dx launch import-manifest --json",
  "dx launch handoff --json",
  "dx launch import-summary --json",
  "dx launch fallback-drill --json",
  "dx launch status --json",
  "dx launch release-gate --json",
];

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

const assertSafePacket = (label: string, packet: any) => {
  assert.equal(packet.status, "ready", `${label} should stay ready`);
  assert.notEqual(packet.next_action?.trim(), "", `${label} next action cannot be blank`);
  assert.equal(nestedCommandFanoutCount(packet), 0, `${label} declares command fanout`);
  for (const field of sensitiveRedactionFields) {
    assert.equal(packet.redaction?.[field], false, `${label} redaction ${field} must stay false`);
  }
};

test("DX launch contract fixtures preserve handoff and import-manifest safety", () => {
  assert.ok(existsSync(fixtureRoot), `missing launch contract fixture root: ${fixtureRoot}`);

  const manifest = readJson("import-manifest.json");
  assert.equal(manifest.schema_version, "dx.launch.import_manifest.v1");
  assertSafePacket("import-manifest.json", manifest);
  assert.equal(manifest.packet_count, manifest.packets.length);
  assert.ok(manifest.fixture_family_count >= 4);

  const manifestCommands = new Set(manifest.packets.map((packet: any) => packet.command));
  for (const command of requiredManifestCommands) {
    assert.ok(manifestCommands.has(command), `missing manifest command ${command}`);
  }
  for (const [index, packet] of manifest.packets.entries()) {
    assert.equal(packet.call_order, index + 1, `${packet.id} call order drifted`);
    assert.equal(packet.metadata_only, true, `${packet.id} should stay metadata-only`);
    assert.equal(packet.command_fanout, false, `${packet.id} should not fan out commands`);
    assert.ok(Array.isArray(packet.parser_fixtures), `${packet.id} parser fixtures must be listed`);
    assert.notEqual(packet.zed_surface?.trim(), "", `${packet.id} needs a Zed surface`);
  }

  const handoff = readJson("handoff.json");
  assert.equal(handoff.schema_version, "dx.launch.handoff.v1");
  assertSafePacket("handoff.json", handoff);
  assert.equal(handoff.no_command_fanout, true);
  assert.equal(handoff.latest_status_receipt?.schema_version, "dx.launch.status.v1");
  assert.equal(handoff.latest_status_receipt?.status, "ready");
  assert.equal(handoff.action_map?.action_count, handoff.action_map?.actions?.length);
  assert.ok(handoff.polling?.startup_commands?.includes("dx launch handoff --json"));
  assert.ok(handoff.polling?.diagnostics_commands?.includes("dx launch drift --json"));
  assert.equal(handoff.polling?.cached_receipt_path, ".dx/receipts/launch/status-latest.json");

  for (const action of handoff.action_map.actions) {
    assert.equal(action.metadata_only, true, `${action.id} should stay metadata-only`);
    assert.equal(action.command_fanout, false, `${action.id} should not fan out commands`);
    assert.equal(typeof action.confirmation_required, "boolean");
    assert.ok(Array.isArray(action.next_actions), `${action.id} next actions must be listed`);
  }
});
