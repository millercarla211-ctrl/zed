import { build } from "esbuild";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const target = process.argv[2] ?? "chromium";
const outdir = path.join(root, "dist", target);

const manifests = {
  chromium: "manifest.chromium.json",
  firefox: "manifest.firefox.json",
  safari: "manifest.safari.json",
};

const backgroundEntry =
  target === "chromium"
    ? path.join(root, "src", "background", "chromium.ts")
    : path.join(root, "src", "background", "gecko.ts");

const entryPoints = {
  "background/index": backgroundEntry,
  "content/index": path.join(root, "src", "content", "index.ts"),
  "ui/popup": path.join(root, "src", "ui", "popup.ts"),
  "ui/options": path.join(root, "src", "ui", "options.ts"),
  "ui/sidepanel": path.join(root, "src", "ui", "sidepanel.ts"),
  "ui/sidebar": path.join(root, "src", "ui", "sidebar.ts"),
  "ui/offscreen": path.join(root, "src", "ui", "offscreen.ts"),
};

await fs.rm(outdir, { recursive: true, force: true });
await fs.mkdir(outdir, { recursive: true });

await build({
  absWorkingDir: root,
  bundle: true,
  entryPoints,
  outdir,
  format: "esm",
  target: "es2022",
  sourcemap: false,
  platform: "browser",
  define: {
    __FLOW_TARGET_BROWSER__: JSON.stringify(target),
    __FLOW_BROWSER_FLAVOR__: JSON.stringify(target),
  },
});

for (const file of [
  "popup.html",
  "options.html",
  "sidepanel.html",
  "sidebar.html",
  "offscreen.html",
  "flow.css",
  "content-overlay.css",
]) {
  await fs.copyFile(path.join(root, "static", file), path.join(outdir, file));
}

await fs.copyFile(
  path.join(root, "manifests", manifests[target] ?? manifests.chromium),
  path.join(outdir, "manifest.json"),
);
