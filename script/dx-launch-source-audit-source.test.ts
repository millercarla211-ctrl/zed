import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch source audit keeps paths, packet IO, fields, rows, and status focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_source_audit.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_source_audit/fields.rs";
  const packetIoPath = "crates/agent_ui/src/dx_launch_source_audit/packet_io.rs";
  const pathsPath = "crates/agent_ui/src/dx_launch_source_audit/paths.rs";
  const rowsPath = "crates/agent_ui/src/dx_launch_source_audit/rows.rs";
  const snapshotPath = "crates/agent_ui/src/dx_launch_source_audit/snapshot.rs";
  const statusPath = "crates/agent_ui/src/dx_launch_source_audit/status.rs";

  assert.ok(existsSync(fieldsPath), "missing focused source-audit field helpers");
  assert.ok(existsSync(packetIoPath), "missing focused source-audit packet IO");
  assert.ok(existsSync(pathsPath), "missing focused source-audit path module");
  assert.ok(existsSync(rowsPath), "missing focused source-audit row module");
  assert.ok(existsSync(snapshotPath), "missing focused source-audit snapshot type module");
  assert.ok(existsSync(statusPath), "missing focused source-audit status module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const packetIo = read(packetIoPath);
  const paths = read(pathsPath);
  const rows = read(rowsPath);
  const snapshot = read(snapshotPath);
  const status = read(statusPath);

  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod packet_io;$/m);
  assert.match(parent, /^mod paths;$/m);
  assert.match(parent, /^mod rows;$/m);
  assert.match(parent, /^mod snapshot;$/m);
  assert.match(parent, /^mod status;$/m);
  assert.match(parent, /pub\(crate\) use self::snapshot::DxLaunchSourceAuditSnapshot;/);
  assert.doesNotMatch(parent, /pub\(crate\) struct DxLaunchSourceAuditSnapshot/);
  assert.match(parent, /use self::fields::\{bool_field, string_field, usize_field\};/);
  assert.match(parent, /use self::packet_io::\{packet_schema, read_json_packet\};/);
  assert.match(parent, /use self::paths::source_audit_paths;/);
  assert.match(parent, /use self::rows::\{delta_row, repo_row\};/);
  assert.match(parent, /use self::status::\{source_audit_operator_summary, source_audit_status\};/);
  assert.doesNotMatch(parent, /fn read_json_packet\(/);
  assert.doesNotMatch(parent, /fn packet_schema\(/);
  assert.doesNotMatch(parent, /fn repo_row\(/);
  assert.doesNotMatch(parent, /fn delta_row\(/);
  assert.doesNotMatch(parent, /fn signed_field\(/);
  assert.doesNotMatch(parent, /fn bool_label\(/);
  assert.doesNotMatch(parent, /fn string_field</);
  assert.match(fields, /pub\(super\) fn string_field/);
  assert.match(fields, /pub\(super\) fn usize_field/);
  assert.match(fields, /pub\(super\) fn bool_field/);
  assert.match(packetIo, /pub\(super\) fn read_json_packet/);
  assert.match(packetIo, /pub\(super\) fn packet_schema/);
  assert.match(packetIo, /MAX_AUDIT_BYTES/);
  assert.match(paths, /pub\(super\) struct SourceAuditPaths/);
  assert.match(paths, /pub\(super\) fn source_audit_paths/);
  assert.match(rows, /pub\(super\) fn repo_row/);
  assert.match(rows, /pub\(super\) fn delta_row/);
  assert.match(rows, /fn signed_field/);
  assert.match(rows, /fn bool_label/);
  assert.match(snapshot, /pub\(crate\) struct DxLaunchSourceAuditSnapshot/);
  assert.match(snapshot, /pub latest_path: PathBuf/);
  assert.match(status, /pub\(super\) fn source_audit_status/);
  assert.match(status, /pub\(super\) fn source_audit_operator_summary/);
  assert.match(status, /coordination_status\.contains\("blocked"\)/);

  assert.ok(lineCount(parentPath) < 240, "dx_launch_source_audit.rs should stay focused on cache and snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 35, "source-audit field helpers should stay small");
  assert.ok(lineCount(packetIoPath) < 55, "source-audit packet IO should stay small");
  assert.ok(lineCount(pathsPath) < 60, "source-audit path module should stay small");
  assert.ok(lineCount(rowsPath) < 70, "source-audit row module should stay small");
  assert.ok(lineCount(snapshotPath) < 70, "source-audit snapshot type module should stay small");
  assert.ok(lineCount(statusPath) < 65, "source-audit status module should stay small");
});
