import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

test("Windows scanner keeps expected system-entry metadata failures out of error logs", () => {
  const source = read("crates/worktree/src/worktree.rs");

  const scanDir = sliceBetween(
    source,
    "async fn scan_dir(&self, job: &ScanJob)",
    "async fn reload_entries_for_paths(",
  );
  assert.ok(
    scanDir.includes('log::debug!("skipping excluded directory {:?}", job.path);'),
  );
  assert.doesNotMatch(scanDir, /log::error!\("skipping excluded directory/);

  const metadataGuard = sliceBetween(
    source,
    "fn should_ignore_windows_scan_metadata_error",
    "fn is_windows_reserved_device_name",
  );
  assert.match(metadataGuard, /let Some\(io_error\) = err\.downcast_ref::<std::io::Error>\(\)/);
  assert.match(metadataGuard, /let Some\(raw_os_error\) = io_error\.raw_os_error\(\)/);
  assert.match(
    metadataGuard,
    /return is_windows_expected_reserved_device_error\(raw_os_error\);/,
  );
  assert.match(metadataGuard, /5 \| 32 => is_windows_expected_system_entry\(file_name\)/);
  assert.ok(
    metadataGuard.indexOf("err.downcast_ref::<std::io::Error>()") <
      metadataGuard.indexOf("is_windows_reserved_device_name(file_name)"),
    "reserved device skips must still be gated by a Windows io error",
  );
  assert.doesNotMatch(metadataGuard, /Some\(5\) => is_windows_protected_system_entry/);
  assert.doesNotMatch(metadataGuard, /Some\(32\) => is_windows_locked_system_entry/);

  const reservedErrors = sliceBetween(
    source,
    "fn is_windows_expected_reserved_device_error",
    "fn is_windows_expected_system_entry",
  );
  assert.match(reservedErrors, /matches!\(raw_os_error, 2 \| 3 \| 5 \| 32 \| 87 \| 123\)/);

  const expectedEntries = sliceBetween(
    source,
    "fn is_windows_expected_system_entry",
    "fn is_windows_reserved_device_name",
  );
  assert.match(expectedEntries, /is_windows_protected_system_entry\(file_name\)/);
  assert.match(expectedEntries, /is_windows_locked_system_entry\(file_name\)/);

  const protectedEntries = sliceBetween(
    source,
    "fn is_windows_protected_system_entry",
    "fn is_windows_locked_system_entry",
  );
  assert.match(protectedEntries, /System Volume Information/);
  assert.match(protectedEntries, /\$RECYCLE\.BIN/);

  const lockedEntries = sliceBetween(
    source,
    "fn is_windows_locked_system_entry",
    "fn char_bag_for_path",
  );
  for (const entry of ["pagefile.sys", "swapfile.sys", "hiberfil.sys", "DumpStack.log", "DumpStack.log.tmp"]) {
    assert.match(lockedEntries, new RegExp(entry.replaceAll(".", "\\.")));
  }

  const reloadEntries = sliceBetween(
    source,
    "async fn reload_entries_for_paths(",
    "fn remove_repo_path(",
  );
  assert.match(reloadEntries, /if should_ignore_scan_metadata_error\(&abs_path, &err\) \{/);
  assert.match(reloadEntries, /log::debug!\(\s+"skipping unavailable filesystem entry \{\:\?\} on event: \{err:#\}"/);
  assert.match(reloadEntries, /log::error!\("error reading file \{abs_path:\?\} on event: \{err:#\}"\);/);
});

test("Minidump upload skips local and remote missing-commit dev metadata quietly", () => {
  const source = read("crates/zed/src/reliability.rs");

  const upload = sliceBetween(
    source,
    "async fn upload_minidump(",
    "let mut form = Form::new()",
  );
  assert.match(upload, /if has_missing_minidump_commit_sha\(&metadata\.init\.commit_sha\) \{/);
  assert.match(upload, /log_missing_minidump_commit_sha\(metadata\);/);
  assert.match(upload, /return Ok\(\(\)\);/);

  const missingSha = sliceBetween(
    source,
    "fn has_missing_minidump_commit_sha",
    "fn log_missing_minidump_commit_sha",
  );
  assert.match(missingSha, /matches!\(commit_sha, "no sha" \| "no_sha"\)/);

  const missingShaLog = sliceBetween(
    source,
    "fn log_missing_minidump_commit_sha",
    "let mut form = Form::new()",
  );
  assert.match(missingShaLog, /metadata\.init\.release_channel\.eq_ignore_ascii_case\("dev"\)/);
  assert.match(missingShaLog, /log::debug!\("No commit sha set; skipping dev minidump upload"\);/);
  assert.match(missingShaLog, /log::warn!\("No commit sha set, skipping minidump upload"\);/);

  const previous = sliceBetween(
    source,
    "pub async fn upload_previous_minidumps",
    "fn has_missing_minidump_commit_sha",
  );
  assert.match(previous, /let Some\(minidump_endpoint\) = MINIDUMP_ENDPOINT\.as_ref\(\) else/);
  assert.match(previous, /log::debug!\("Minidump endpoint not set; skipping previous minidump upload"\);/);
  assert.ok(
    previous.indexOf("MINIDUMP_ENDPOINT") < previous.indexOf("paths::logs_dir()"),
    "local minidump reads should stay behind endpoint lookup",
  );

  const remote = sliceBetween(
    source,
    "remote_client.update(cx, |remote_client, cx|",
    "anyhow::Ok(())",
  );
  assert.match(remote, /if !client\.telemetry\(\)\.diagnostics_enabled\(\) \{\s+return;\s+\}/);
  assert.match(remote, /let Some\(endpoint\) = MINIDUMP_ENDPOINT\.as_ref\(\)\.cloned\(\) else/);
  assert.match(remote, /skipping remote minidump upload/);
  assert.ok(
    remote.indexOf("diagnostics_enabled") < remote.indexOf("MINIDUMP_ENDPOINT"),
    "diagnostics gating should run before endpoint lookup",
  );
  assert.ok(
    remote.indexOf("MINIDUMP_ENDPOINT") < remote.indexOf("request(proto::GetCrashFiles {})"),
    "remote crash-file requests should stay behind endpoint lookup",
  );
});

test("production-readiness docs name the Windows reliability source guard", () => {
  const docs = [read("DX.md"), read("todo.txt"), read("changelog.txt")].join("\n");

  assert.match(docs, /DX Windows reliability source guard/);
  assert.match(docs, /script\\dx-windows-reliability-source\.test\.ts/);
  assert.match(docs, /Windows scanner metadata/);
  assert.match(docs, /remote minidump upload/);
  assert.match(docs, /no-Cargo\/no-`just run`/);
});
