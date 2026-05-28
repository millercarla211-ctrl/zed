import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/repl/src/components/kernel_options.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

const functionBody = (haystack: string, name: string) => {
  const start = haystack.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = haystack.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < haystack.length; index += 1) {
    const char = haystack[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return haystack.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const assertBefore = (
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) => {
  const beforeIndex =
    typeof before === "string"
      ? haystack.indexOf(before)
      : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string"
      ? haystack.indexOf(after)
      : haystack.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("kernel picker clamps stale filtered entry indexes before header handling", () => {
  const clampEntryIndex = functionBody(source, "clamp_entry_index");
  const setSelectedIndex = functionBody(source, "set_selected_index");

  assertBefore(
    clampEntryIndex,
    "if self.filtered_entries.is_empty()",
    "ix.min(self.filtered_entries.len() - 1)",
    "empty filtered lists must return before len - 1 clamping",
  );
  assertBefore(
    setSelectedIndex,
    "let Some(ix) = self.clamp_entry_index(ix) else",
    "self.filtered_entries.get(ix)",
    "stale picker indexes must be clamped before entry lookup",
  );
  assertBefore(
    setSelectedIndex,
    "self.selected_kernelspec = None;",
    "return;",
    "empty filtered lists must clear stale selected-kernel state",
  );
  assertBefore(
    setSelectedIndex,
    "self.filtered_entries.get(ix)",
    "self.sync_selected_kernelspec();",
    "header skipping must finish before selected-kernel state is synced",
  );
});

test("kernel picker clears stale selected kernel after empty or header-only matches", () => {
  const syncSelectedKernelspec = functionBody(source, "sync_selected_kernelspec");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(
    syncSelectedKernelspec,
    /Some\(KernelPickerEntry::Kernel \{ spec, \.\. \}\) => Some\(spec\.clone\(\)\)/,
  );
  assert.match(syncSelectedKernelspec, /_ => None/);
  assertBefore(
    updateMatches,
    "self.selected_index = Self::first_selectable_index(&self.filtered_entries);",
    "self.sync_selected_kernelspec();",
    "filter updates must resync selection after recalculating the first selectable row",
  );
  assert.doesNotMatch(
    updateMatches,
    /self\.selected_kernelspec = Some/,
    "match updates should not leave stale selected kernels when no kernel row matches",
  );
});

test("REPL UI source guard is focused on production kernel picker code", () => {
  assert.equal(sourcePath, "crates/repl/src/components/kernel_options.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production kernel picker code",
  );
});
