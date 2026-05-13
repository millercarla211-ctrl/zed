import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

import { zipSync, strToU8 } from "fflate";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const target = process.argv[2] ?? "chromium";
const distDir = path.join(root, "dist", target);
const artifactsDir = path.join(root, "artifacts");
const packageJson = JSON.parse(
  await fs.readFile(path.join(root, "package.json"), "utf8"),
);

async function fileTree(dir, relative = "") {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files = {};

  for (const entry of entries) {
    const absolute = path.join(dir, entry.name);
    const nextRelative = relative ? `${relative}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      Object.assign(files, await fileTree(absolute, nextRelative));
      continue;
    }

    files[nextRelative] = new Uint8Array(await fs.readFile(absolute));
  }

  return files;
}

try {
  await fs.access(distDir);
} catch {
  throw new Error(`Missing build output for ${target}. Run the build first.`);
}

const files = await fileTree(distDir);
const archiveName = `flow-webext-${target}-v${packageJson.version}.zip`;
const archivePath = path.join(artifactsDir, archiveName);
const checksumPath = `${archivePath}.sha256`;

await fs.mkdir(artifactsDir, { recursive: true });

const archive = zipSync(files, { level: 9 });
await fs.writeFile(archivePath, archive);

const sha256 = crypto.createHash("sha256").update(archive).digest("hex");
await fs.writeFile(checksumPath, `${sha256}  ${archiveName}\n`, "utf8");

console.log(`Created ${archivePath}`);
console.log(`Created ${checksumPath}`);
