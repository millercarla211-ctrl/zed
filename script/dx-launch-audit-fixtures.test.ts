import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const fixtureRoot =
  process.env.DX_LAUNCH_EXAMPLES_ROOT ?? "G:/Dx/cli/fixtures/launch-examples";
const packetSchemas = {
  "schemas.json": "dx.launch.schemas.v1",
  "fixtures.json": "dx.launch.fixtures.v1",
  "smoke.json": "dx.launch.smoke.v1",
  "status.json": "dx.launch.status.v1",
};
const expectedLaunchCommands = [
  "dx launch drift --json",
  "dx launch fallback-drill --json",
  "dx launch import-manifest --json",
  "dx launch import-summary --json",
  "dx launch release-gate --json",
];
const sensitiveRedactionFields = [
  "exports_source_file_contents",
  "exports_source_file_paths",
  "exports_secret_values",
  "exports_receipt_bodies",
  "exports_prompts",
  "exports_transcripts",
  "exports_command_payloads",
];

const readPacket = (fileName: keyof typeof packetSchemas) =>
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

const assertMetadataOnly = (fileName: keyof typeof packetSchemas, packet: any) => {
  assert.equal(nestedCommandFanoutCount(packet), 0, `${fileName} declares command fanout`);
  assert.notEqual(packet.next_action?.trim(), "", `${fileName} next action cannot be blank`);

  for (const field of sensitiveRedactionFields) {
    assert.equal(
      packet.redaction?.[field],
      false,
      `${fileName} redaction ${field} must stay false`,
    );
  }
};

test("DX launch audit fixtures preserve the Zed launch audit contract", () => {
  assert.ok(existsSync(fixtureRoot), `missing launch audit fixture root: ${fixtureRoot}`);

  const packets = Object.fromEntries(
    Object.entries(packetSchemas).map(([fileName, schema]) => {
      const packet = readPacket(fileName as keyof typeof packetSchemas);
      assert.equal(packet.schema_version, schema, `${fileName} schema changed`);
      assert.equal(packet.status, "ready", `${fileName} should stay ready`);
      assertMetadataOnly(fileName as keyof typeof packetSchemas, packet);
      return [fileName, packet];
    }),
  );

  const schemas = packets["schemas.json"];
  assert.ok(schemas.command_count >= schemas.commands.length);
  assert.ok(schemas.commands.length >= expectedLaunchCommands.length);
  const commandNames = new Set(schemas.commands.map((command: any) => command.cli_command));
  for (const commandName of expectedLaunchCommands) {
    assert.ok(commandNames.has(commandName), `missing launch command ${commandName}`);
  }

  for (const command of schemas.commands) {
    assert.equal(command.execution_risk, "metadata_only_no_execution");
    assert.ok(Array.isArray(command.reads), `${command.id} reads must be listed`);
    assert.ok(Array.isArray(command.writes), `${command.id} writes must be listed`);
    assert.equal(command.writes.length, 0, `${command.id} should not write in audit fixtures`);
    assert.equal(typeof command.poll_on_startup, "boolean");
    assert.equal(typeof command.user_action_required, "boolean");
  }

  const fixtures = packets["fixtures.json"];
  assert.ok(fixtures.fixture_count >= fixtures.fixtures.length);
  assert.ok(fixtures.fixtures.length >= 1);
  for (const fixture of fixtures.fixtures) {
    assert.equal(fixture.status_matches_expected, true, `${fixture.id} status drifted`);
    assert.notEqual(fixture.render_state?.primary_action?.trim(), "");
    assert.equal(fixture.render_state?.live_command, "dx launch status --json");
  }

  const smoke = packets["smoke.json"];
  assert.ok(smoke.check_count >= smoke.checks.length);
  assert.equal(smoke.passed_count + smoke.warning_count + smoke.failed_count, smoke.check_count);
  assert.equal(smoke.warning_count, 0);
  assert.equal(smoke.failed_count, 0);
  for (const check of smoke.checks) {
    assert.equal(check.metadata_only, true, `${check.id} should stay metadata-only`);
    assert.equal(check.command_fanout, false, `${check.id} should not fan out commands`);
  }

  const status = packets["status.json"];
  for (const section of ["agents", "tokens", "discovery"]) {
    assert.equal(status[section]?.status, "ready", `${section} status should stay ready`);
    assert.notEqual(status[section]?.next_action?.trim(), "");
  }
});
