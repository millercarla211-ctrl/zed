import { readFileSync, readdirSync } from "node:fs";

export const deploySourceDir = "crates/agent_ui/src";

export const read = (path) => readFileSync(path, "utf8");

export const lineCount = (path) => read(path).split(/\r?\n/).length;

export const deploySourceFiles = () =>
  readdirSync(deploySourceDir)
    .filter((name) => name.startsWith("dx_deploy") && name.endsWith(".rs"))
    .sort();

export const deployGuardFiles = () =>
  readdirSync("script")
    .filter((name) => name.startsWith("dx-deploy") && name.endsWith("-source.test.ts"))
    .sort();
