use crate::{
    AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME, AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
    AGENT_BROWSER_PAYLOAD_TOOL_NAME, AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
    AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME, AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
    AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_RESULT_SCHEMA,
    AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME, AGENT_CHROME_PAYLOAD_RESULT_SCHEMA,
    AGENT_CHROME_PAYLOAD_TOOL_NAME, AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA,
    AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME, AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
    AGENT_CHROME_PLAYWRIGHT_INVOCATION_RESULT_SCHEMA, AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA, AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
    AGENT_CHROME_RUNNER_GATE_TOOL_NAME, AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME,
    AGENT_CHROME_RUNNER_RECEIPT_SCHEMA, AgentTool, ToolCallEventStream, ToolInput,
};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

const PREPARE_AGENT_PLUGIN_RUNTIME_TOOL: &str = "prepare_agent_plugin_runtime";

/// Lists the built-in DX/Zed agent plugin catalog for browser, Chrome, and PC-use workflows.
///
/// Use this before trying to control the in-app WebPreview browser, external Chrome through
/// Playwright and the DX Chrome extension, or future permissioned PC UI tools. The tool is
/// read-only and returns capability manifests, bootstrap roots, current readiness, and safety
/// requirements.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPluginCatalogToolInput {
    /// Include plugins that are planned or require bootstrap before they can execute.
    pub include_planned_plugins: bool,
    /// Include install roots and download/update policy for default plugin provisioning.
    pub include_bootstrap_plan: bool,
    /// Include current host/workspace readiness for Chrome, Playwright, and the DX extension.
    pub include_bootstrap_readiness: bool,
}

impl Default for AgentPluginCatalogToolInput {
    fn default() -> Self {
        Self {
            include_planned_plugins: true,
            include_bootstrap_plan: true,
            include_bootstrap_readiness: true,
        }
    }
}

pub struct AgentPluginCatalogTool {
    project: Entity<Project>,
}

impl AgentPluginCatalogTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPluginCatalogTool {
    type Input = AgentPluginCatalogToolInput;
    type Output = String;

    const NAME: &'static str = "list_agent_plugins";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "List agent plugins".into()
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
            let catalog = agent_plugin_catalog(project_root, input);
            let catalog = serde_json::to_string_pretty(&catalog)
                .map_err(|error| format!("Failed to serialize agent plugin catalog: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Listed agent plugin catalog"),
            );

            Ok(catalog)
        })
    }
}

fn workspace_root_for_project(project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
}

fn agent_plugin_catalog(
    project_root: Option<PathBuf>,
    input: AgentPluginCatalogToolInput,
) -> Value {
    let zed_data_root = data_dir();
    let default_plugin_root = zed_data_root.join("agent-plugins");
    let workspace_plugin_root = project_root
        .as_ref()
        .map(|root| root.join("tools").join("agent-plugins"));
    let workspace_tools_root = project_root.as_ref().map(|root| root.join("tools"));

    let mut plugins = vec![
        browser_plugin_manifest(),
        chrome_plugin_manifest(
            workspace_tools_root.clone(),
            workspace_plugin_root.clone(),
            &default_plugin_root,
        ),
        pc_use_plugin_manifest(workspace_plugin_root.clone(), &default_plugin_root),
    ];

    if !input.include_planned_plugins {
        plugins.retain(|plugin| {
            plugin
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "available")
        });
    }

    serde_json::json!({
        "schema": "zed.agent_plugins.catalog.v1",
        "generated_at_ms": current_epoch_millis(),
        "catalog": {
            "name": "DX Agent Plugin Runtime",
            "status": "discovery_layer_available",
            "default_enabled_plugins": ["zed.browser", "zed.chrome", "zed.pc_use"],
            "tool_name": AgentPluginCatalogTool::NAME,
            "tools": {
                "discovery": AgentPluginCatalogTool::NAME,
                "compose_browser_action_payload": AGENT_BROWSER_PAYLOAD_TOOL_NAME,
                "stage_browser_action_payload": AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
                "queue_browser_action_payload": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
                "compose_chrome_action_payload": AGENT_CHROME_PAYLOAD_TOOL_NAME,
                "queue_chrome_action_payload": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
                "inspect_chrome_action_payload_queue": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "request_chrome_payload_run": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                "prepare_chrome_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "invoke_chrome_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                "inspect_chrome_playwright_executions": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                "prepare_runtime": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL
            },
            "available_to": [
                "agent_panel",
                "subagents",
                "acp_threads",
                "web_preview_agent_handoff"
            ],
            "bootstrap_plan": input.include_bootstrap_plan.then(|| serde_json::json!({
                "default_download": true,
                "download_policy": "download_or_update_on_first_use",
                "prepare_tool": {
                    "name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
                    "dry_run_payload": {
                        "root_mode": "workspace",
                        "create_managed_roots": false,
                        "write_bootstrap_manifest": false
                    },
                    "workspace_payload": {
                        "root_mode": "workspace",
                        "create_managed_roots": true,
                        "write_bootstrap_manifest": true
                    },
                    "zed_data_payload": {
                        "root_mode": "zed_data",
                        "create_managed_roots": true,
                        "write_bootstrap_manifest": true
                    },
                    "requires_permission_for_writes": true,
                    "downloads_or_launches_browser": false
                },
                "playwright_adapter_prepare_tool": {
                    "name": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                    "dry_run_payload": {
                        "root_mode": "workspace",
                        "write_adapter_files": false,
                        "include_script_preview": false
                    },
                    "workspace_payload": {
                        "root_mode": "workspace",
                        "write_adapter_files": true,
                        "include_script_preview": false
                    },
                    "zed_data_payload": {
                        "root_mode": "zed_data",
                        "write_adapter_files": true,
                        "include_script_preview": false
                    },
                    "requires_permission_for_writes": true,
                    "installs_packages": false,
                    "launches_browser": false,
                    "runs_node": false
                },
                "playwright_adapter_invoke_tool": {
                    "name": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                    "dry_run_payload": {
                        "root_mode": "workspace",
                        "execute_adapter": false,
                        "timeout_ms": 60000,
                        "include_process_output": false,
                        "include_payload_packet": false
                    },
                    "execute_payload": {
                        "root_mode": "workspace",
                        "execute_adapter": true,
                        "timeout_ms": 60000,
                        "include_process_output": false,
                        "include_payload_packet": false
                    },
                    "requires_permission_for_execution": true,
                    "safe_actions_only": ["open_url", "screenshot", "set_viewport", "wait_for_selector"],
                    "input_actions_blocked": ["click", "type_text", "press_key", "scroll"]
                },
                "playwright_execution_inspect_tool": {
                    "name": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                    "payload": {
                        "root_mode": "workspace",
                        "max_entries": 8,
                        "include_requests": false,
                        "include_receipts": false
                    },
                    "read_only": true,
                    "launches_browser": false,
                    "runs_node": false
                },
                "zed_data_plugin_root": default_plugin_root.display().to_string(),
                "workspace_plugin_root": workspace_plugin_root.as_ref().map(|path| path.display().to_string()),
                "workspace_tools_root": workspace_tools_root.as_ref().map(|path| path.display().to_string()),
                "dx_chrome_extension": {
                    "install_policy": "download_or_update_on_first_use",
                    "preferred_root": workspace_plugin_root
                        .as_ref()
                        .map(|root| root.join("dx-chrome-extension"))
                        .unwrap_or_else(|| default_plugin_root.join("dx-chrome-extension"))
                        .display()
                        .to_string(),
                    "load_mode": "unpacked_extension",
                    "never_write_to_user_browser_profiles": true
                },
                "playwright": {
                    "preferred_root": workspace_tools_root
                        .as_ref()
                        .map(|root| root.join("playwright"))
                        .unwrap_or_else(|| default_plugin_root.join("playwright"))
                        .display()
                        .to_string(),
                    "managed_by": "DX Code Editor",
                    "install_policy": "download_or_update_on_first_use"
                }
            })),
            "bootstrap_readiness": input.include_bootstrap_readiness.then(|| {
                agent_plugin_bootstrap_readiness(
                    project_root.as_ref(),
                    &default_plugin_root,
                    workspace_plugin_root.as_ref(),
                    workspace_tools_root.as_ref(),
                )
            }),
            "permission_model": {
                "read_only_discovery_without_prompt": true,
                "browser_interactions_require_explicit_session_unlock": true,
                "external_chrome_and_pc_use_require_user_visible_permission": true,
                "receipts_required_for_every_mutating_action": true,
                "fresh_preflight_required_before_input": true
            },
            "plugins": plugins,
        }
    })
}

fn browser_plugin_manifest() -> Value {
    serde_json::json!({
        "id": "zed.browser",
        "name": "Browser",
        "description": "Controls the in-app WebPreview browser session through Zed's native browser context, diagnostics, screenshots, and permissioned executor gates.",
        "kind": "built_in",
        "status": "available",
        "default_enabled": true,
        "ships_with_editor": true,
        "scope": "in_app_web_preview",
        "runtime": {
            "backend": "web_preview",
            "requires_external_process": false,
            "platforms": {
                "windows": "webview2_composition",
                "macos": "wkwebview",
                "linux": "webkitgtk_planned"
            }
        },
        "entrypoints": [
            "WebPreview More menu",
            "Agent Panel content handoff",
            "list_agent_plugins tool"
        ],
        "action_payload_contract": {
            "payload_tool_name": AGENT_BROWSER_PAYLOAD_TOOL_NAME,
            "payload_stage_tool_name": AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
            "payload_queue_tool_name": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
            "bridge_schema": "zed.web_preview.agent_browser_action_payload_bridge.v1",
            "executor_payload_schema": "zed.web_preview.agent_browser_executor_payload.v1",
            "payload_queue_item_schema": "zed.agent_plugins.browser_action_payload_queue_item.v1",
            "payload_import_receipt_schema": "zed.web_preview.agent_browser_action_payload_import_receipt.v1",
            "clipboard_import_action": "import_agent_browser_action_payload_from_clipboard",
            "managed_queue_import_action": "import_agent_browser_action_payload_from_managed_queue",
            "examples": [
                {
                    "action": "type_text",
                    "payload": {
                        "schema": "zed.web_preview.agent_browser_executor_payload.v1",
                        "payload": {
                            "action": "type_text",
                            "selector": "optional CSS selector from the latest type preflight",
                            "text": "Text to insert",
                            "clear_existing": false
                        }
                    }
                },
                {
                    "action": "press_key",
                    "payload": {
                        "schema": "zed.web_preview.agent_browser_executor_payload.v1",
                        "payload": {
                            "action": "press_key",
                            "key": "Escape",
                            "modifiers": []
                        }
                    }
                }
            ],
            "rules": [
                "Payload bridges are handoff artifacts and never dispatch by themselves.",
                "Interactive executors still require unlock, fresh preflight, action-specific gates, and receipts.",
                "type_text requires non-empty payload.text and rejects selector mismatches when a selector is supplied."
            ]
        },
        "capabilities": [
            capability("browser.sessions.list", "available", "List open WebPreview sessions and workspace inventory."),
            capability("browser.session.snapshot", "available", "Read the active WebPreview session metadata, bounds, profile, URL, and policy."),
            capability("browser.page.diagnostics", "available", "Collect ready state, title, URL, DOM counts, and page metadata."),
            capability("browser.dom.snapshot", "available", "Collect a bounded DOM tree snapshot for model context."),
            capability("browser.runtime.events", "available", "Read bounded console, page-error, fetch, and XHR event buffers."),
            capability("browser.screenshot.capture", "available", "Capture WebPreview screenshots for Agent Panel attachments."),
            capability("browser.screenshot.area", "available", "Capture a selected WebPreview rectangle for Agent Panel attachments."),
            capability("browser.screenshot.annotate", "available", "Draw page annotations and capture the marked WebPreview screenshot with metadata."),
            capability("browser.element.inspect", "available", "Pick a page element and send selector, HTML, computed styles, rect, and screenshot context to the Agent Panel."),
            capability("browser.devtools.open", "available", "Open the native browser DevTools for the active WebPreview backend."),
            capability("browser.viewport.responsive", "available", "Switch the active WebPreview between full, phone, tablet, laptop, and rotated responsive viewports."),
            capability("browser.action.open_url", "available_when_unlocked", "Open the current URL/search editor text through the permissioned WebPreview executor shell."),
            capability("browser.action.reload", "available_when_unlocked", "Reload through the permissioned WebPreview executor shell."),
            capability("browser.action.go_back", "available_when_unlocked", "Navigate back through the native WebPreview history executor after unlock, native history trace, QA checklist, and receipt logging."),
            capability("browser.action.go_forward", "available_when_unlocked", "Navigate forward through the native WebPreview history executor after unlock, native history trace, QA checklist, and receipt logging."),
            capability("browser.action.clear_data", "available_when_unlocked", "Clear WebPreview browsing data through the permissioned executor shell."),
            capability("browser.action.clear_cache", "available_when_unlocked", "Clear only WebPreview disk cache and cache storage through the scoped native executor after unlock, cache-reset trace, QA checklist, and receipt logging."),
            capability("browser.action.set_viewport", "available_when_unlocked", "Switch to the next responsive viewport preset through the permissioned WebPreview executor shell."),
            capability("browser.action.click_preflight", "available", "Select a visible click target and emit the receipt a future native click must satisfy without dispatching input."),
            capability("browser.action.type_preflight", "available", "Select a visible text-entry target and emit the receipt a future native type action must satisfy without dispatching input."),
            capability("browser.action.key_preflight", "available", "Prepare a safe key candidate and emit the receipt a future native key action must satisfy without dispatching input."),
            capability("browser.action.scroll_preflight", "available", "Select a scrollable page or element target and emit the receipt a future native scroll action must satisfy without dispatching input."),
            capability("browser.action.native_input_bridge", "planned_manual_qa_gate", "Trace the disabled-by-default native input bridge readiness before click, type, key, or scroll dispatch can be enabled."),
            capability("browser.action.native_click_trace", "available", "Translate the latest click preflight target into native WebPreview coordinates and emit a trace receipt without dispatching input."),
            capability("browser.action.native_type_trace", "available", "Translate the latest type preflight target into native WebPreview coordinate and keyboard-focus planning without dispatching input."),
            capability("browser.action.native_key_trace", "available", "Translate the latest key preflight candidate into native keyboard-focus planning without dispatching input."),
            capability("browser.action.native_scroll_trace", "available", "Translate the latest scroll preflight target into native wheel-coordinate planning without dispatching input."),
            capability("browser.action.native_history_trace", "available", "Trace native back/forward readiness and receipt requirements without navigating the page."),
            capability("browser.action.native_cache_reset_trace", "available", "Trace scoped cache-reset readiness and profile-safety requirements without clearing browser data."),
            capability("browser.dispatch.manual_qa_checklist", "available", "Generate the final manual QA checklist required before enabling native browser dispatch."),
            capability("browser.action.payload_compose", "available", "Use compose_agent_browser_action_payload to generate validated WebPreview action payload packets before importing them into the payload bridge."),
            capability("browser.action.payload_stage_clipboard", "available_requires_authorization", "Use stage_agent_browser_action_payload to write a validated WebPreview action payload packet to the clipboard for explicit WebPreview import."),
            capability("browser.action.payload_queue_managed", "available_requires_authorization", "Use queue_agent_browser_action_payload to write a validated payload packet into the managed workspace or Zed-data Browser payload queue for explicit WebPreview import."),
            capability("browser.action.payload_bridge", "available", "Generate or send a schema-versioned payload bridge that maps Agent action payloads into WebPreview executors without dispatching by itself."),
            capability("browser.action.payload_import_clipboard", "available_explicit_user_action", "Import a JSON action payload or plain text from the clipboard into the active WebPreview payload bridge for the next type executor attempt."),
            capability("browser.action.payload_import_queue", "available_explicit_user_action", "Import the latest managed Agent Browser payload queue item into the active WebPreview payload bridge without dispatching input."),
            capability("browser.action.payload_import_receipt", "available", "Copy or send the latest WebPreview payload import receipt, with accepted schema, action metadata, redacted text length, permission state, and next-step safety notes."),
            capability("browser.action.click", "available_when_unlocked", "Click visible page targets through the Windows native WebView executor after unlock, fresh preflight, QA checklist, and receipt logging."),
            capability("browser.action.type", "available_when_unlocked_payload_required", "Insert explicit payload text through the WebView2 DevTools Protocol executor after unlock, fresh type preflight, focused-target check, keyboard-focus gate, QA checklist, and receipt logging."),
            capability("browser.action.key", "available_when_unlocked", "Send allowlisted key presses through the WebView2 DevTools Protocol executor after unlock, fresh preflight, keyboard-focus gate, QA checklist, and receipt logging."),
            capability("browser.action.scroll", "available_when_unlocked", "Scroll page or element targets through the Windows native WebView executor after unlock, fresh preflight, QA checklist, and receipt logging.")
        ],
        "safety": {
            "interactive_locked_by_default": true,
            "uses_current_webpreview_profile": true,
            "does_not_mutate_external_browser_profiles": true,
            "requires_receipts": true
        }
    })
}

fn chrome_plugin_manifest(
    workspace_tools_root: Option<PathBuf>,
    workspace_plugin_root: Option<PathBuf>,
    default_plugin_root: &std::path::Path,
) -> Value {
    serde_json::json!({
        "id": "zed.chrome",
        "name": "Chrome",
        "description": "Controls an external managed Chrome session with Playwright plus the DX Chrome extension bridge.",
        "kind": "built_in",
        "status": "requires_bootstrap",
        "default_enabled": true,
        "ships_with_editor": true,
        "scope": "external_chrome_playwright_dx_extension",
        "runtime": {
            "backend": "playwright",
            "requires_node": true,
            "requires_managed_chrome": true,
            "requires_dx_chrome_extension": true,
            "playwright_root": workspace_tools_root
                .as_ref()
                .map(|root| root.join("playwright"))
                .unwrap_or_else(|| default_plugin_root.join("playwright"))
                .display()
                .to_string(),
            "playwright_adapter_root": workspace_tools_root
                .as_ref()
                .map(|root| root.join("playwright").join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME))
                .unwrap_or_else(|| {
                    default_plugin_root
                        .join("playwright")
                        .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME)
                })
                .display()
                .to_string(),
            "playwright_runner_script": workspace_tools_root
                .as_ref()
                .map(|root| {
                    root.join("playwright")
                        .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME)
                        .join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME)
                })
                .unwrap_or_else(|| {
                    default_plugin_root
                        .join("playwright")
                        .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME)
                        .join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME)
                })
                .display()
                .to_string(),
            "dx_extension_root": workspace_plugin_root
                .as_ref()
                .map(|root| root.join("dx-chrome-extension"))
                .unwrap_or_else(|| default_plugin_root.join("dx-chrome-extension"))
                .display()
                .to_string(),
            "profile_policy": "managed_profile_only"
        },
        "download": {
            "default_download": true,
            "policy": "download_or_update_on_first_use",
            "assets": [
                "playwright_node_runtime",
                "playwright_chromium_or_system_chrome_adapter",
                "dx_chrome_extension_unpacked"
            ],
            "never_write_to_user_real_chrome_profile": true
        },
        "action_payload_contract": {
            "payload_tool_name": AGENT_CHROME_PAYLOAD_TOOL_NAME,
            "payload_queue_tool_name": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_queue_inspect_tool_name": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "runner_gate_tool_name": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
            "playwright_adapter_tool_name": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
            "playwright_invoke_tool_name": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
            "playwright_execution_inspect_tool_name": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
            "webpreview_execution_status_copy_action": "copy_managed_chrome_execution_status",
            "webpreview_execution_status_agent_action": "send_managed_chrome_execution_status_to_agent",
            "webpreview_execution_status_schema": "zed.web_preview.managed_chrome_execution_status.v1",
            "playwright_run_request_schema": AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA,
            "playwright_invocation_result_schema": AGENT_CHROME_PLAYWRIGHT_INVOCATION_RESULT_SCHEMA,
            "playwright_adapter_manifest_schema": AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA,
            "playwright_execution_receipt_schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
            "playwright_execution_inspection_schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
            "playwright_adapter_root_name": AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME,
            "playwright_runner_script_name": AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
            "runner_receipt_schema": AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
            "latest_runner_receipt_file": AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME,
            "payload_result_schema": AGENT_CHROME_PAYLOAD_RESULT_SCHEMA,
            "executor_payload_schema": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
            "payload_queue_item_schema": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "payload_queue_result_schema": AGENT_CHROME_PAYLOAD_QUEUE_RESULT_SCHEMA,
            "latest_queue_file": AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
            "managed_queue_roots": {
                "workspace": workspace_plugin_root
                    .as_ref()
                    .map(|root| root.join("chrome-payloads").display().to_string()),
                "zed_data": default_plugin_root
                    .join("chrome-payloads")
                    .display()
                    .to_string()
            },
            "managed_adapter_roots": {
                "workspace": workspace_tools_root.as_ref().map(|root| {
                    root.join("playwright")
                        .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME)
                        .display()
                        .to_string()
                }),
                "zed_data": default_plugin_root
                    .join("playwright")
                    .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME)
                    .display()
                    .to_string()
            },
            "managed_execution_roots": {
                "workspace": workspace_tools_root.as_ref().map(|root| {
                    root.join("agent-plugins")
                        .join("chrome-executions")
                        .display()
                        .to_string()
                }),
                "zed_data": default_plugin_root
                    .join("chrome-executions")
                    .display()
                    .to_string()
            },
            "supported_actions": [
                "open_url",
                "click",
                "type_text",
                "press_key",
                "scroll",
                "screenshot",
                "wait_for_selector",
                "set_viewport"
            ],
            "examples": [
                {
                    "action": "open_url",
                    "payload": {
                        "schema": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
                        "payload": {
                            "action": "open_url",
                            "url": "http://localhost:3000"
                        }
                    }
                },
                {
                    "action": "type_text",
                    "payload": {
                        "schema": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
                        "payload": {
                            "action": "type_text",
                            "selector": "input[name='email']",
                            "text": "user@example.com"
                        }
                    }
                },
                {
                    "action": "set_viewport",
                    "payload": {
                        "schema": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
                        "payload": {
                            "action": "set_viewport",
                            "width": 390,
                            "height": 844,
                            "device_scale_factor": 3.0
                        }
                    }
                }
            ],
            "rules": [
                "Payload tools never launch Chrome, install Playwright, dispatch input, or run page scripts.",
                "Queued payloads are written only to managed workspace or Zed-data plugin roots after authorization.",
                "The Playwright adapter preparation tool writes only versioned adapter files under managed roots and does not run Node.",
                "The Playwright invocation tool can run only open_url, screenshot, set_viewport, and wait_for_selector after authorization and a ready runner receipt.",
                "The Playwright execution inspection tool is read-only and summarizes managed request and receipt files.",
                "WebPreview can copy or send the latest managed execution status to the Agent Panel without launching Chrome.",
                "Future execution must use managed profiles, explicit permission, fresh preflight, and receipts.",
                "The runner must never write into the user's real Chrome, Edge, or Firefox profile."
            ]
        },
        "capabilities": [
            capability("chrome.action.payload_compose", "available", "Use compose_managed_chrome_action_payload to generate validated managed Chrome/Playwright action packets."),
            capability("chrome.action.payload_queue_managed", "available_requires_authorization", "Use queue_managed_chrome_action_payload to write a validated Chrome action packet into the managed workspace or Zed-data queue."),
            capability("chrome.action.payload_queue_inspect", "available", "Use inspect_managed_chrome_payload_queue to validate the latest queued Chrome payload and runner prerequisites before launch or dispatch exists."),
            capability("chrome.action.runner_gate", "available_requires_authorization", "Use request_managed_chrome_payload_run to write a permissioned runner receipt that blocks until queue, bootstrap, managed-profile, and future adapter requirements are satisfied."),
            capability("chrome.runtime.playwright_adapter_prepare", "available_requires_authorization", "Use prepare_managed_chrome_playwright_adapter to write a versioned managed Playwright adapter artifact without installing packages, launching Chrome, or dispatching input."),
            capability("chrome.runtime.playwright_adapter_invoke", "available_requires_authorization", "Use invoke_managed_chrome_playwright_adapter to run the prepared adapter for open_url, screenshot, set_viewport, or wait_for_selector after a ready runner receipt."),
            capability("chrome.runtime.playwright_execution_inspect", "available", "Use inspect_managed_chrome_playwright_executions to read recent managed run requests and execution receipts without launching Chrome."),
            capability("chrome.runtime.playwright_execution_status_handoff", "available", "Use WebPreview Copy/Send Managed Chrome Execution Status to hand the latest managed request or receipt summary to the Agent Panel."),
            capability("chrome.action.payload_queue_schema", "available", "Read the managed Chrome payload packet, queue item, queue result, and latest-file schemas for future runner execution."),
            capability("chrome.session.launch", "requires_bootstrap", "Launch or attach to a managed Chrome profile."),
            capability("chrome.page.open_url", "requires_bootstrap", "Open URLs in managed Chrome tabs."),
            capability("chrome.page.click", "requires_permission", "Click elements through Playwright locators or extension targets."),
            capability("chrome.page.type", "requires_permission", "Type into focused inputs through Playwright or extension bridge."),
            capability("chrome.page.press_key", "requires_permission", "Press keyboard shortcuts in managed Chrome."),
            capability("chrome.page.scroll", "requires_permission", "Scroll pages and containers in managed Chrome."),
            capability("chrome.page.screenshot", "requires_bootstrap", "Capture full-page or viewport screenshots."),
            capability("chrome.page.dom_snapshot", "requires_bootstrap", "Read DOM/accessibility snapshots."),
            capability("chrome.runtime.console", "requires_bootstrap", "Read console, page errors, and network events."),
            capability("chrome.extension.bridge", "requires_bootstrap", "Use the DX Chrome extension bridge for pages where DevTools-only control is insufficient.")
        ],
        "safety": {
            "managed_profile_only": true,
            "explicit_permission_required_for_input": true,
            "receipts_required": true,
            "os_wide_control": false
        }
    })
}

fn pc_use_plugin_manifest(
    workspace_plugin_root: Option<PathBuf>,
    default_plugin_root: &std::path::Path,
) -> Value {
    serde_json::json!({
        "id": "zed.pc_use",
        "name": "PC Use",
        "description": "Permissioned Zed-window UI inspection and future local computer-use actions for agent workflows.",
        "kind": "built_in",
        "status": "planned_permission_gate",
        "default_enabled": true,
        "ships_with_editor": true,
        "scope": "zed_window_and_permissioned_desktop",
        "runtime": {
            "backend": "zed_window_runtime",
            "plugin_root": workspace_plugin_root
                .as_ref()
                .map(|root| root.join("pc-use"))
                .unwrap_or_else(|| default_plugin_root.join("pc-use"))
                .display()
                .to_string(),
            "os_wide_automation": "requires_separate_explicit_permission"
        },
        "capabilities": [
            capability("pc.zed_window.screenshot", "planned", "Capture Zed-window screenshots for agent context."),
            capability("pc.zed_window.focus", "planned", "Focus Zed panes, panels, and tabs by safe editor-native handles."),
            capability("pc.zed_window.click", "planned_permission_gate", "Click within Zed surfaces only after permission and target preflight."),
            capability("pc.zed_window.type", "planned_permission_gate", "Type within Zed surfaces only after permission and target preflight."),
            capability("pc.zed_window.inspect_ui", "planned", "Read safe UI metadata for currently visible Zed surfaces."),
            capability("pc.desktop.os_wide", "blocked_by_default", "OS-wide desktop automation remains unavailable until the user explicitly enables it.")
        ],
        "safety": {
            "zed_window_first": true,
            "os_wide_actions_blocked_by_default": true,
            "explicit_permission_required_for_input": true,
            "receipts_required": true
        }
    })
}

fn agent_plugin_bootstrap_readiness(
    project_root: Option<&PathBuf>,
    default_plugin_root: &Path,
    workspace_plugin_root: Option<&PathBuf>,
    workspace_tools_root: Option<&PathBuf>,
) -> Value {
    let workspace_tools_root = workspace_tools_root.cloned();
    let workspace_plugin_root = workspace_plugin_root.cloned();
    let playwright_root = workspace_tools_root
        .as_ref()
        .map(|root| root.join("playwright"))
        .unwrap_or_else(|| default_plugin_root.join("playwright"));
    let playwright_adapter_root = playwright_root.join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME);
    let playwright_adapter_manifest = playwright_adapter_root.join("adapter-manifest.json");
    let playwright_runner_script =
        playwright_adapter_root.join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME);
    let dx_extension_root = workspace_plugin_root
        .as_ref()
        .map(|root| root.join("dx-chrome-extension"))
        .unwrap_or_else(|| default_plugin_root.join("dx-chrome-extension"));
    let managed_profile_root = workspace_tools_root
        .as_ref()
        .map(|root| root.join("browser-profiles").join("chrome"))
        .unwrap_or_else(|| default_plugin_root.join("browser-profiles").join("chrome"));

    let node = find_executable(&["node", "node.exe"]);
    let npm = find_executable(&["npm", "npm.cmd", "npm.exe"]);
    let browser = find_browser_executable();
    let playwright_package = playwright_root
        .join("node_modules")
        .join("playwright")
        .join("package.json");
    let playwright_adapter_manifest_ready = adapter_manifest_ready(&playwright_adapter_manifest);
    let dx_extension_manifest = dx_extension_root.join("manifest.json");

    let checks = vec![
        bootstrap_check(
            "workspace.root",
            "Workspace root",
            project_root.is_some(),
            project_root.cloned(),
            "host_blocker",
            "A workspace root is needed so managed browser tools stay inside the project.",
        ),
        bootstrap_check(
            "host.node",
            "Node.js runtime",
            node.is_some(),
            node.clone(),
            "host_blocker",
            "Playwright and Chrome plugin bootstrapping need Node.js.",
        ),
        bootstrap_check(
            "host.npm",
            "npm package manager",
            npm.is_some(),
            npm.clone(),
            "host_blocker",
            "Playwright package provisioning needs npm or a compatible npm executable.",
        ),
        bootstrap_check(
            "host.chrome_or_edge",
            "Chrome or Edge executable",
            browser.is_some(),
            browser.clone(),
            "host_blocker",
            "External Chrome control needs Chrome, Edge, or Chromium on this OS.",
        ),
        bootstrap_check(
            "asset.playwright_package",
            "Managed Playwright package",
            playwright_package.is_file(),
            Some(playwright_package.clone()),
            "provision_required",
            "Install Playwright into the managed tools root before launching external Chrome.",
        ),
        bootstrap_check(
            "asset.playwright_adapter_manifest",
            "Managed Playwright adapter manifest",
            playwright_adapter_manifest_ready,
            Some(playwright_adapter_manifest.clone()),
            "provision_required",
            "Prepare the managed Playwright adapter artifact before launching external Chrome.",
        ),
        bootstrap_check(
            "asset.playwright_adapter_runner",
            "Managed Playwright adapter runner",
            playwright_runner_script.is_file(),
            Some(playwright_runner_script.clone()),
            "provision_required",
            "Prepare the managed Playwright runner script before launching external Chrome.",
        ),
        bootstrap_check(
            "asset.dx_chrome_extension",
            "DX Chrome extension manifest",
            dx_extension_manifest.is_file(),
            Some(dx_extension_manifest.clone()),
            "provision_required",
            "Download or unpack the DX Chrome extension before loading managed Chrome with the bridge.",
        ),
        bootstrap_check(
            "profile.managed_chrome",
            "Managed Chrome profile root",
            managed_profile_root.is_dir(),
            Some(managed_profile_root.clone()),
            "provision_required",
            "Create this profile root and never write into a user's real Chrome, Edge, or Firefox profile.",
        ),
    ];

    let host_blockers = readiness_issues(&checks, "host_blocker");
    let provision_required = readiness_issues(&checks, "provision_required");
    let status = if !host_blockers.is_empty() {
        "blocked_missing_host_dependencies"
    } else if !provision_required.is_empty() {
        "ready_to_provision"
    } else {
        "ready_for_managed_chrome_executor"
    };

    serde_json::json!({
        "schema": "zed.agent_plugins.bootstrap_readiness.v1",
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "prepare_tool_name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
        "project_root": project_root.map(path_string),
        "roots": {
            "zed_data_plugin_root": path_string(default_plugin_root),
            "workspace_plugin_root": workspace_plugin_root.as_ref().map(path_string),
            "workspace_tools_root": workspace_tools_root.as_ref().map(path_string),
            "playwright_root": path_string(&playwright_root),
            "playwright_adapter_root": path_string(&playwright_adapter_root),
            "playwright_adapter_manifest": path_string(&playwright_adapter_manifest),
            "playwright_runner_script": path_string(&playwright_runner_script),
            "dx_chrome_extension_root": path_string(&dx_extension_root),
            "managed_chrome_profile_root": path_string(&managed_profile_root),
        },
        "host": {
            "node": node.as_ref().map(path_string),
            "npm": npm.as_ref().map(path_string),
            "chrome_or_edge": browser.as_ref().map(path_string),
        },
        "checks": checks,
        "host_blockers": host_blockers,
        "provision_required": provision_required,
        "next_actions": bootstrap_next_actions(status),
        "safety": {
            "write_scope": "managed Zed data roots or workspace tools roots only",
            "never_write_to_user_browser_profiles": true,
            "external_browser_input_requires_user_permission": true,
            "receipts_required_for_executor_actions": true,
        },
    })
}

fn bootstrap_check(
    id: &str,
    label: &str,
    ready: bool,
    path: Option<PathBuf>,
    missing_kind: &str,
    details: &str,
) -> Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "state": if ready { "ready" } else { missing_kind },
        "ready": ready,
        "path": path.as_ref().map(path_string),
        "details": details,
    })
}

fn readiness_issues(checks: &[Value], state: &str) -> Vec<Value> {
    checks
        .iter()
        .filter(|check| {
            check
                .get("state")
                .and_then(Value::as_str)
                .is_some_and(|check_state| check_state == state)
        })
        .cloned()
        .collect()
}

fn adapter_manifest_ready(path: &Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("schema")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .is_some_and(|schema| schema == AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA)
}

fn bootstrap_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "blocked_missing_host_dependencies" => vec![
            "Install missing host dependencies first: Node.js, npm, and Chrome/Edge/Chromium.",
            "Re-run list_agent_plugins with include_bootstrap_readiness=true before provisioning.",
        ],
        "ready_to_provision" => vec![
            "Run prepare_agent_plugin_runtime with create_managed_roots=true and write_bootstrap_manifest=true to create the managed roots.",
            "Install Playwright into the managed tools root.",
            "Run prepare_managed_chrome_playwright_adapter with write_adapter_files=true.",
            "Download or unpack the DX Chrome extension into the managed agent plugin root.",
            "Keep managed Chrome profile data in the prepared profile root; never touch real user browser profiles.",
        ],
        _ => vec![
            "Chrome plugin bootstrap assets are present.",
            "Invoke the prepared Playwright adapter for safe actions, then inspect execution receipts before enabling input dispatch.",
        ],
    }
}

fn find_browser_executable() -> Option<PathBuf> {
    find_executable(&[
        "chrome",
        "chrome.exe",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "msedge",
        "msedge.exe",
        "microsoft-edge",
    ])
    .or_else(|| existing_file(common_browser_candidates()))
}

fn common_browser_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if cfg!(target_os = "windows") {
        for env_name in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Some(root) = env_path(env_name) {
                candidates.push(
                    root.join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                );
                candidates.push(
                    root.join("Microsoft")
                        .join("Edge")
                        .join("Application")
                        .join("msedge.exe"),
                );
            }
        }
    } else if cfg!(target_os = "macos") {
        candidates.push(
            PathBuf::from("/Applications")
                .join("Google Chrome.app")
                .join("Contents")
                .join("MacOS")
                .join("Google Chrome"),
        );
        candidates.push(
            PathBuf::from("/Applications")
                .join("Microsoft Edge.app")
                .join("Contents")
                .join("MacOS")
                .join("Microsoft Edge"),
        );
    }

    candidates
}

fn find_executable(names: &[&str]) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn existing_file(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn capability(id: &str, state: &str, description: &str) -> Value {
    serde_json::json!({
        "id": id,
        "state": state,
        "description": description,
    })
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
