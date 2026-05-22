import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;
const normalizedPath = (path: string) => path.replaceAll("\\", "/");

const collectFiles = (root: string, extensions: Set<string>): string[] => {
  if (!existsSync(root)) return [];

  const stats = statSync(root);
  if (stats.isFile()) {
    return extensions.has(root.slice(root.lastIndexOf("."))) ? [root] : [];
  }

  return readdirSync(root, { withFileTypes: true })
    .flatMap((entry) => {
      const child = join(root, entry.name);
      if (entry.isDirectory()) return collectFiles(child, extensions);
      return extensions.has(entry.name.slice(entry.name.lastIndexOf(".")))
        ? [child]
        : [];
    })
    .sort();
};

test("DX Studio source is split into small owned modules", () => {
  const rustFiles = [
    "crates/web_preview/src/dx_studio.rs",
    "crates/web_preview/src/dx_studio_bridge.rs",
    "crates/web_preview/src/dx_studio_source_edit.rs",
    ...collectFiles("crates/web_preview/src/dx_studio", new Set([".rs"])),
    ...collectFiles("crates/web_preview/src/dx_studio_source_edit", new Set([".rs"])),
  ];

  assert.ok(rustFiles.length >= 10, "expected DX Studio to stay split by ownership");
  for (const file of rustFiles) {
    assert.ok(lineCount(file) < 800, `${file} is too large`);
  }
});

test("DX Studio bridge is assembled from focused browser-script fragments", () => {
  const bridgeSource = read("crates/web_preview/src/dx_studio_bridge.rs");
  const fragments = [
    "crates/web_preview/src/dx_studio_bridge/preamble.ts",
    "crates/web_preview/src/dx_studio_bridge/selection.ts",
    "crates/web_preview/src/dx_studio_bridge/overlay.ts",
    "crates/web_preview/src/dx_studio_bridge/capture.ts",
    "crates/web_preview/src/dx_studio_bridge/source_edit.ts",
    "crates/web_preview/src/dx_studio_bridge/api.ts",
  ];
  const discoveredFragments = collectFiles(
    "crates/web_preview/src/dx_studio_bridge",
    new Set([".ts"]),
  );

  assert.match(bridgeSource, /concat!\(/);
  assert.match(bridgeSource, /include_str!\("dx_studio_bridge\/preamble\.ts"\)/);
  assert.doesNotMatch(bridgeSource, /r#"/);
  assert.deepEqual(discoveredFragments.map(normalizedPath), [...fragments].sort());

  for (const fragment of fragments) {
    assert.ok(lineCount(fragment) < 380, `${fragment} is too large`);
  }

  const combinedScript = fragments.map((fragment) => read(fragment)).join("");
  assert.doesNotThrow(() => new Function(combinedScript));
});
