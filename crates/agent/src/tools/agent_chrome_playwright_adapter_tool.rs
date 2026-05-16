use super::agent_chrome_payload_tool::AgentChromePayloadQueueRootMode;
use crate::{AgentTool, ToolCallEventStream, ToolInput, ToolPermissionContext};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME: &str =
    "prepare_managed_chrome_playwright_adapter";
pub const AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_adapter_manifest.v1";
pub const AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_execution_receipt.v1";
pub const AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME: &str = "zed-managed-chrome-runner";
pub const AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME: &str = "managed_chrome_runner.mjs";

/// Prepares the managed Playwright adapter artifact for future external Chrome execution.
///
/// By default this is a dry run. When `write_adapter_files` is true, it writes only versioned
/// adapter files inside the managed workspace tools root or Zed-data plugin root after explicit
/// authorization. It does not install packages, launch Chrome, run Node, dispatch browser input,
/// run page scripts, or touch real browser profiles.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePlaywrightAdapterToolInput {
    /// Prefer workspace-local adapter files under `<workspace>/tools/playwright`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Write the adapter package, runner script, README, and manifest into the managed adapter root.
    pub write_adapter_files: bool,
    /// Include the generated runner script text in the returned JSON for review.
    pub include_script_preview: bool,
}

impl Default for AgentChromePlaywrightAdapterToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            write_adapter_files: false,
            include_script_preview: false,
        }
    }
}

pub struct AgentChromePlaywrightAdapterTool {
    project: Entity<Project>,
}

impl AgentChromePlaywrightAdapterTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromePlaywrightAdapterTool {
    type Input = AgentChromePlaywrightAdapterToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) if input.write_adapter_files => "Prepare Chrome Playwright adapter".into(),
            _ => "Plan Chrome Playwright adapter".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let project_root = cx.update(|cx| workspace_root_for_project(&self.project, cx));
            let plan = ManagedChromePlaywrightAdapterPlan::new(project_root, input.root_mode);
            plan.validate_managed_paths()?;

            if input.write_adapter_files {
                let context = ToolPermissionContext::new(Self::NAME, plan.permission_values());
                let authorize = cx
                    .update(|cx| {
                        event_stream.authorize(
                            self.initial_title(Ok(input.clone()), cx),
                            context,
                            cx,
                        )
                    })
                    .map_err(|error| error.to_string())?;
                authorize.await.map_err(|error| error.to_string())?;
            }

            let result = plan.apply(&input)?;
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize Chrome Playwright adapter result: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(
                if input.write_adapter_files {
                    "Prepared Chrome Playwright adapter"
                } else {
                    "Planned Chrome Playwright adapter"
                },
            ));

            Ok(output)
        })
    }
}

struct ManagedChromePlaywrightAdapterPlan {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    playwright_root: PathBuf,
    adapter_root: PathBuf,
    dx_extension_root: PathBuf,
    managed_profile_root: PathBuf,
    package_json_path: PathBuf,
    runner_script_path: PathBuf,
    readme_path: PathBuf,
    manifest_path: PathBuf,
}

impl ManagedChromePlaywrightAdapterPlan {
    fn new(project_root: Option<PathBuf>, root_mode: AgentChromePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentChromePayloadQueueRootMode::Workspace)
            && project_root.is_some();
        let zed_plugin_root = data_dir().join("agent-plugins");
        let (allowed_root, plugin_root, playwright_root, managed_profile_root) = if use_workspace {
            let workspace_root = project_root.as_ref().expect("workspace root checked above");
            let tools_root = workspace_root.join("tools");
            (
                tools_root.clone(),
                tools_root.join("agent-plugins"),
                tools_root.join("playwright"),
                tools_root.join("browser-profiles").join("chrome"),
            )
        } else {
            (
                zed_plugin_root.clone(),
                zed_plugin_root.clone(),
                zed_plugin_root.join("playwright"),
                zed_plugin_root.join("browser-profiles").join("chrome"),
            )
        };
        let adapter_root = playwright_root.join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME);
        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let package_json_path = adapter_root.join("package.json");
        let runner_script_path = adapter_root.join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME);
        let readme_path = adapter_root.join("README.md");
        let manifest_path = adapter_root.join("adapter-manifest.json");

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            playwright_root,
            adapter_root,
            dx_extension_root,
            managed_profile_root,
            package_json_path,
            runner_script_path,
            readme_path,
            manifest_path,
        }
    }

    fn apply(&self, input: &AgentChromePlaywrightAdapterToolInput) -> Result<Value, String> {
        let files = self.file_specs()?;
        let mut written_files = Vec::new();

        if input.write_adapter_files {
            fs::create_dir_all(&self.adapter_root).map_err(|error| {
                format!(
                    "Failed to prepare Chrome Playwright adapter root {}: {error}",
                    self.adapter_root.display()
                )
            })?;

            for file in &files {
                fs::write(&file.path, file.contents.as_bytes()).map_err(|error| {
                    format!(
                        "Failed to write Chrome Playwright adapter file {}: {error}",
                        file.path.display()
                    )
                })?;
                written_files.push(path_string(&file.path));
            }
        }

        let planned_files = files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "kind": file.kind,
                    "path": path_string(&file.path),
                    "bytes": file.contents.len(),
                })
            })
            .collect::<Vec<_>>();

        let mut result = serde_json::json!({
            "schema": "zed.agent_plugins.managed_chrome_playwright_adapter_prepare.v1",
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "root_mode": self.root_mode_label(),
                "applied": input.write_adapter_files,
                "adapter_root": path_string(&self.adapter_root),
                "written_files": written_files,
                "planned_files": planned_files,
            },
            "adapter": {
                "manifest_schema": AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA,
                "execution_receipt_schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
                "runner_script": path_string(&self.runner_script_path),
                "package_json": path_string(&self.package_json_path),
                "manifest": path_string(&self.manifest_path),
                "readme": path_string(&self.readme_path),
                "supported_actions": ["open_url", "screenshot", "inspect_element", "set_viewport", "wait_for_selector"],
                "input_actions_blocked_by_default": ["click", "type_text", "press_key", "scroll"],
            },
            "roots": {
                "allowed_root": path_string(&self.allowed_root),
                "plugin_root": path_string(&self.plugin_root),
                "playwright_root": path_string(&self.playwright_root),
                "adapter_root": path_string(&self.adapter_root),
                "dx_extension_root": path_string(&self.dx_extension_root),
                "managed_chrome_profile_root": path_string(&self.managed_profile_root),
            },
            "next_actions": [
                "Install or verify Playwright under the managed Playwright root before executing the adapter.",
                "Queue a managed Chrome payload, inspect it, and request the runner gate receipt.",
                "Future executor wiring should invoke this adapter only after the runner receipt is ready.",
                "Keep click, type, key, and scroll blocked until their Playwright dispatch receipts and QA gates are implemented."
            ],
            "safety": {
                "requires_permission_for_writes": true,
                "installs_packages": false,
                "runs_node": false,
                "launches_chrome": false,
                "dispatches_browser_input": false,
                "runs_page_scripts": false,
                "touches_real_browser_profiles": false,
                "write_scope": "managed workspace tools root or Zed-data agent plugin root only",
            }
        });

        if input.include_script_preview {
            result["script_preview"] = Value::String(MANAGED_CHROME_RUNNER_SCRIPT.to_string());
        }

        Ok(result)
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [
            &self.adapter_root,
            &self.package_json_path,
            &self.runner_script_path,
            &self.readme_path,
            &self.manifest_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing Chrome Playwright adapter path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn permission_values(&self) -> Vec<String> {
        vec![
            path_string(&self.adapter_root),
            path_string(&self.package_json_path),
            path_string(&self.runner_script_path),
            path_string(&self.readme_path),
            path_string(&self.manifest_path),
        ]
    }

    fn file_specs(&self) -> Result<Vec<AdapterFile>, String> {
        Ok(vec![
            AdapterFile {
                kind: "package_json",
                path: self.package_json_path.clone(),
                contents: adapter_package_json()?,
            },
            AdapterFile {
                kind: "runner_script",
                path: self.runner_script_path.clone(),
                contents: MANAGED_CHROME_RUNNER_SCRIPT.to_string(),
            },
            AdapterFile {
                kind: "readme",
                path: self.readme_path.clone(),
                contents: MANAGED_CHROME_ADAPTER_README.to_string(),
            },
            AdapterFile {
                kind: "manifest",
                path: self.manifest_path.clone(),
                contents: adapter_manifest_json(self)?,
            },
        ])
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentChromePayloadQueueRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            AgentChromePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentChromePayloadQueueRootMode::ZedData => "zed_data",
        }
    }
}

struct AdapterFile {
    kind: &'static str,
    path: PathBuf,
    contents: String,
}

fn adapter_package_json() -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": "@zed/managed-chrome-runner",
        "version": "0.1.1",
        "private": true,
        "type": "module",
        "description": "Managed Playwright adapter for DX/Zed Agent Chrome plugin receipts.",
        "scripts": {
            "run-payload": "node managed_chrome_runner.mjs"
        },
        "peerDependencies": {
            "playwright": ">=1.40.0"
        }
    }))
    .map_err(|error| format!("Failed to serialize adapter package.json: {error}"))
}

fn adapter_manifest_json(plan: &ManagedChromePlaywrightAdapterPlan) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "adapter": {
            "name": "DX/Zed Managed Chrome Playwright Adapter",
            "version": "0.1.1",
            "root": path_string(&plan.adapter_root),
            "runner_script": path_string(&plan.runner_script_path),
            "execution_receipt_schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
            "request_schema": "zed.agent_plugins.managed_chrome_playwright_run_request.v1"
        },
        "roots": {
            "playwright_root": path_string(&plan.playwright_root),
            "dx_extension_root": path_string(&plan.dx_extension_root),
            "managed_chrome_profile_root": path_string(&plan.managed_profile_root)
        },
        "supported_actions": ["open_url", "screenshot", "inspect_element", "set_viewport", "wait_for_selector"],
        "input_actions_blocked_by_default": ["click", "type_text", "press_key", "scroll"],
        "safety": {
            "managed_profile_only": true,
            "refuses_unmanaged_profile_roots": true,
            "writes_receipts_for_every_attempt": true,
            "does_not_install_packages": true,
            "does_not_run_until_invoked_by_future_runner": true
        }
    }))
    .map_err(|error| format!("Failed to serialize adapter manifest: {error}"))
}

fn workspace_root_for_project(project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

const MANAGED_CHROME_ADAPTER_README: &str = r#"# DX/Zed Managed Chrome Playwright Adapter

This directory is a managed runner artifact for the Zed Agent Chrome plugin.

The adapter is intentionally inert until a future permissioned executor invokes it with a
schema-versioned request and receipt path. It never uses a real Chrome, Edge, or Firefox user
profile. All browser state must stay in the managed profile root provided by Zed.

Supported execution actions:

- open_url
- screenshot
- inspect_element
- set_viewport
- wait_for_selector

Input actions such as click, type_text, press_key, and scroll currently return blocked receipts.
They should remain blocked until the product wires action-specific permission, focus, QA, and
receipt gates.
"#;

const MANAGED_CHROME_RUNNER_SCRIPT: &str = r#"#!/usr/bin/env node
import { chromium } from "playwright";
import fs from "node:fs/promises";
import path from "node:path";

const RECEIPT_SCHEMA = "zed.agent_plugins.managed_chrome_playwright_execution_receipt.v1";
const INPUT_ACTIONS = new Set(["click", "type_text", "press_key", "scroll"]);
const SUPPORTED_ACTIONS = new Set(["open_url", "screenshot", "inspect_element", "set_viewport", "wait_for_selector"]);

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const item = argv[index];
    if (item === "--request") {
      args.request = argv[index + 1];
      index += 1;
    } else if (item === "--receipt") {
      args.receipt = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function isInside(root, candidate) {
  const relative = path.relative(path.resolve(root), path.resolve(candidate));
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function requireInside(root, candidate, label) {
  if (!root || !candidate || !isInside(root, candidate)) {
    throw new Error(`${label} must stay inside the managed root`);
  }
}

function payloadFromRequest(request) {
  const packet = request.payload_packet ?? request.queue_item?.payload_packet ?? request.payload;
  if (!packet || packet.schema !== "zed.agent_plugins.managed_chrome_executor_payload.v1") {
    throw new Error("Request is missing a managed Chrome executor payload packet");
  }
  const payload = packet.payload;
  if (!payload || typeof payload.action !== "string") {
    throw new Error("Managed Chrome payload is missing an action");
  }
  return { packet, payload };
}

async function writeReceipt(receiptPath, receipt) {
  await fs.mkdir(path.dirname(receiptPath), { recursive: true });
  await fs.writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`, "utf8");
}

function baseReceipt(request, payload, outcome) {
  return {
    schema: RECEIPT_SCHEMA,
    generated_at_ms: Date.now(),
    outcome,
    action: payload?.action ?? "unknown",
    request_schema: request?.schema ?? null,
    queue_item_schema: request?.queue_item?.schema ?? null,
    url: null,
    title: null,
    artifacts: {},
    safety: {
      managed_profile_only: true,
      real_browser_profiles_touched: false,
      input_dispatch_blocked_by_default: true,
      page_scripts_executed: false
    },
    details: []
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.request || !args.receipt) {
    throw new Error("Usage: node managed_chrome_runner.mjs --request <request.json> --receipt <receipt.json>");
  }

  const requestText = await fs.readFile(args.request, "utf8");
  const request = JSON.parse(requestText);
  const { payload } = payloadFromRequest(request);
  const roots = request.roots ?? {};
  const profileRoot = roots.managed_chrome_profile_root;
  const adapterRoot = roots.adapter_root;
  const dxExtensionRoot = roots.dx_extension_root;
  const executablePath = request.browser?.executable_path ?? undefined;
  const managedRoot = roots.allowed_root
    ?? roots.managed_root
    ?? roots.workspace_tools_root
    ?? roots.zed_data_plugin_root
    ?? (adapterRoot ? path.dirname(path.dirname(adapterRoot)) : undefined);

  requireInside(managedRoot, args.receipt, "receipt path");
  requireInside(managedRoot, profileRoot, "managed Chrome profile root");

  if (INPUT_ACTIONS.has(payload.action)) {
    const receipt = baseReceipt(request, payload, "blocked_input_adapter_not_enabled");
    receipt.details.push("Input actions remain blocked until action-specific Playwright permission, focus, QA, and receipt gates are implemented.");
    await writeReceipt(args.receipt, receipt);
    return;
  }

  if (!SUPPORTED_ACTIONS.has(payload.action)) {
    const receipt = baseReceipt(request, payload, "blocked_unsupported_action");
    receipt.details.push(`Unsupported managed Chrome action: ${payload.action}`);
    await writeReceipt(args.receipt, receipt);
    return;
  }

  const launchArgs = [];
  try {
    const manifest = path.join(dxExtensionRoot ?? "", "manifest.json");
    await fs.access(manifest);
    launchArgs.push(`--disable-extensions-except=${dxExtensionRoot}`);
    launchArgs.push(`--load-extension=${dxExtensionRoot}`);
  } catch {
    // The extension is optional for the read-only adapter actions supported by this slice.
  }

  const context = await chromium.launchPersistentContext(profileRoot, {
    headless: false,
    executablePath,
    args: launchArgs,
    viewport: {
      width: Number(payload.width ?? payload.viewport_width ?? 1440),
      height: Number(payload.height ?? payload.viewport_height ?? 900)
    }
  });

  let page = context.pages()[0];
  if (!page) {
    page = await context.newPage();
  }

  const receipt = baseReceipt(request, payload, "completed");
  try {
    if (payload.action === "open_url") {
      if (!payload.url) {
        throw new Error("open_url requires payload.url");
      }
      await page.goto(payload.url, {
        waitUntil: "domcontentloaded",
        timeout: Number(payload.timeout_ms ?? 30000)
      });
    } else if (payload.action === "set_viewport") {
      await page.setViewportSize({
        width: Number(payload.width ?? payload.viewport_width ?? 1440),
        height: Number(payload.height ?? payload.viewport_height ?? 900)
      });
    } else if (payload.action === "wait_for_selector") {
      if (!payload.selector) {
        throw new Error("wait_for_selector requires payload.selector");
      }
      await page.locator(payload.selector).first().waitFor({
        state: "visible",
        timeout: Number(payload.timeout_ms ?? 5000)
      });
    } else if (payload.action === "screenshot") {
      const artifactsRoot = roots.artifacts_root ?? path.join(adapterRoot, "artifacts");
      const outputPath = payload.output_path
        ? path.resolve(payload.output_path)
        : path.join(artifactsRoot, `managed-chrome-screenshot-${Date.now()}.png`);
      requireInside(managedRoot, outputPath, "screenshot output path");
      await fs.mkdir(path.dirname(outputPath), { recursive: true });
      await page.screenshot({
        path: outputPath,
        fullPage: Boolean(payload.full_page)
      });
      receipt.artifacts.screenshot = outputPath;
    } else if (payload.action === "inspect_element") {
      if (!payload.selector) {
        throw new Error("inspect_element requires payload.selector");
      }
      const locator = page.locator(payload.selector).first();
      await locator.waitFor({
        state: "attached",
        timeout: Number(payload.timeout_ms ?? 5000)
      });
      const element = await locator.evaluate((node) => {
        const rect = node.getBoundingClientRect();
        const style = window.getComputedStyle(node);
        const attributes = {};
        for (const name of ["id", "class", "role", "aria-label", "name", "type", "href", "src", "alt", "title", "placeholder"]) {
          const value = node.getAttribute(name);
          if (value !== null) {
            attributes[name] = value.slice(0, 500);
          }
        }
        const rawText = (node.innerText || node.textContent || "").replace(/\s+/g, " ").trim();
        return {
          tag_name: node.tagName.toLowerCase(),
          attributes,
          text_preview: rawText.slice(0, 1000),
          text_truncated: rawText.length > 1000,
          bounding_client_rect: {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
          },
          visible: rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none",
          computed_style: {
            display: style.display,
            visibility: style.visibility,
            position: style.position,
            color: style.color,
            background_color: style.backgroundColor,
            font_size: style.fontSize,
            font_family: style.fontFamily,
            z_index: style.zIndex
          }
        };
      });
      receipt.inspection = {
        selector: payload.selector,
        element
      };
      receipt.safety.page_scripts_executed = true;
    }

    receipt.url = page.url();
    receipt.title = await page.title().catch(() => null);
    receipt.details.push("Managed Chrome adapter completed the requested non-input action.");
  } catch (error) {
    receipt.outcome = "failed";
    receipt.error = String(error?.stack ?? error?.message ?? error);
  } finally {
    await context.close();
  }

  await writeReceipt(args.receipt, receipt);
}

main().catch(async (error) => {
  const args = parseArgs(process.argv.slice(2));
  if (args.receipt) {
    await writeReceipt(args.receipt, {
      schema: RECEIPT_SCHEMA,
      generated_at_ms: Date.now(),
      outcome: "failed_before_launch",
      action: "unknown",
      error: String(error?.stack ?? error?.message ?? error),
      safety: {
        managed_profile_only: true,
        real_browser_profiles_touched: false,
        input_dispatch_blocked_by_default: true,
        page_scripts_executed: false
      }
    });
    return;
  }
  console.error(error);
  process.exitCode = 1;
});
"#;
