use crate::{
    AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME, AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
    AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA, AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
    AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME, AGENT_BROWSER_PAYLOAD_TOOL_NAME,
    AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
    AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME, AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
    AGENT_CHROME_PAYLOAD_QUEUE_RESULT_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
    AGENT_CHROME_PAYLOAD_RESULT_SCHEMA, AGENT_CHROME_PAYLOAD_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA, AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME,
    AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
    AGENT_CHROME_PLAYWRIGHT_INVOCATION_RESULT_SCHEMA, AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
    AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA, AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
    AGENT_CHROME_RUNNER_GATE_TOOL_NAME, AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME,
    AGENT_CHROME_RUNNER_RECEIPT_SCHEMA, AGENT_PC_USE_INSPECT_TOOL_NAME,
    AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
    AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA, AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
    AGENT_PC_USE_PAYLOAD_SCHEMA, AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME,
    AGENT_PC_USE_PAYLOAD_TOOL_NAME, AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
    AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME, AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
    AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA, AGENT_PC_USE_TARGET_MANIFEST_SCHEMA,
    AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME, AGENT_PC_USE_TARGET_SNAPSHOT_SCHEMA,
    AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME, AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_SCHEMA,
    AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME, AGENT_PC_USE_UI_SNAPSHOT_SCHEMA,
    AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME, AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
    AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME,
    AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA, AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
    AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA, AGENT_PLUGIN_RUNTIME_STATUS_SCHEMA,
    AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME, AgentTool, ToolCallEventStream, ToolInput,
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
const AGENT_BROWSER_EXECUTOR_VALIDATION_PROGRESS_SCHEMA: &str =
    "zed.web_preview.agent_browser_executor_validation_progress.v1";
const AGENT_BROWSER_FINAL_VALIDATION_BUNDLE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_bundle.v1";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_result.v1";
const AGENT_BROWSER_FINAL_VALIDATION_OBSERVABILITY_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_observability.v1";
const AGENT_BROWSER_FINAL_VALIDATION_DIR_NAME: &str = "browser-final-validation";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_FILE_NAME: &str =
    "latest-agent-browser-final-validation-result.json";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_ARCHIVE_PREFIX: &str =
    "agent-browser-final-validation-result-";
const AGENT_BROWSER_FUNCTION_SURFACES_SCHEMA: &str =
    "zed.web_preview.agent_browser_function_surfaces.v1";
const AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA: &str = "zed.agent_plugins.bootstrap_readiness.v1";
const AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA: &str = "zed.agent_plugins.bootstrap_manifest.v1";
const AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_prepare_request.v1";
const AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA: &str = "zed.agent_plugins.bootstrap_asset_plan.v1";
const AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA: &str =
    "zed.agent_plugins.managed_asset_operator_recipe.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_blocker_summary.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_SCORECARD_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_readiness_scorecard.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_operator_handoff.v1";
const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_digest.v1";

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
                "runtime_status": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "compose_browser_action_payload": AGENT_BROWSER_PAYLOAD_TOOL_NAME,
                "stage_browser_action_payload": AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
                "queue_browser_action_payload": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
                "inspect_browser_action_payload_queue": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "compose_chrome_action_payload": AGENT_CHROME_PAYLOAD_TOOL_NAME,
                "queue_chrome_action_payload": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
                "inspect_chrome_action_payload_queue": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "request_chrome_payload_run": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                "prepare_chrome_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "invoke_chrome_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                "inspect_chrome_playwright_executions": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                "inspect_zed_window_context": AGENT_PC_USE_INSPECT_TOOL_NAME,
                "compose_zed_pc_use_action_payload": AGENT_PC_USE_PAYLOAD_TOOL_NAME,
                "stage_zed_pc_use_action_payload": AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME,
                "queue_zed_pc_use_action_payload": AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
                "inspect_zed_pc_use_payload_queue": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "request_zed_pc_use_payload_run": AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                "inspect_zed_pc_use_runner_receipts": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
                "inspect_zed_pc_use_targets": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
                "inspect_zed_pc_use_target_snapshot": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
                "inspect_zed_pc_use_ui_snapshot_contract": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
                "inspect_zed_pc_use_ui_snapshot": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                "prepare_managed_assets": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "prepare_runtime": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL
            },
            "runtime_status": {
                "tool_name": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "schema": AGENT_PLUGIN_RUNTIME_STATUS_SCHEMA,
                "runtime_green_blocker_summary_schema": AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA,
                "runtime_green_readiness_scorecard_schema": AGENT_PLUGIN_RUNTIME_GREEN_SCORECARD_SCHEMA,
                "runtime_green_operator_handoff_schema": AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA,
                "runtime_observability_digest_schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA,
                "runtime_green_ready_outcomes": {
                    "browser_final_validation_result": "runtime_green_candidate=true",
                    "managed_chrome_runner_receipt": "ready_runner_adapter_pending",
                    "managed_chrome_execution_receipt": "completed",
                    "pc_use_runner_receipt": "ready_future_executor_pending"
                },
                "read_only": true,
                "payload": {
                    "root_mode": "workspace",
                    "include_latest_handoffs": true,
                    "include_host_checks": true,
                    "include_next_actions": true,
                    "include_workflows": true,
                    "include_validation_matrix": true,
                    "include_observability_profiles": true,
                    "include_observability_digest": true
                },
                "purpose": "Summarize Browser, managed Chrome, PC-use readiness, compact observability digest, proof freshness, and profiles without launching browsers, running Node, screenshots, or input dispatch."
            },
            "webpreview_handoffs": {
                "runtime_green_operator_handoff": {
                    "schema": AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA,
                    "copy_action": "copy_agent_plugin_runtime_green_handoff",
                    "send_action": "send_agent_plugin_runtime_green_handoff_to_agent",
                    "read_only": true,
                    "purpose": "Share one compact WebPreview runtime-green operator packet with current lane evidence and the Agent runtime-status payload."
                }
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
                "managed_asset_provisioner_tool": {
                    "name": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                    "result_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
                    "receipt_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
                    "asset_readiness_summary_schema": AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA,
                    "operator_recipe_schema": AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA,
                    "dry_run_payload": {
                        "root_mode": "workspace",
                        "write_asset_receipt": false,
                        "copy_dx_chrome_extension": false,
                        "dx_chrome_extension_source_root": null,
                        "overwrite_existing_files": false,
                        "include_file_preview": true
                    },
                    "receipt_payload": {
                        "root_mode": "workspace",
                        "write_asset_receipt": true,
                        "copy_dx_chrome_extension": false,
                        "dx_chrome_extension_source_root": null,
                        "overwrite_existing_files": false,
                        "include_file_preview": true
                    },
                    "local_extension_copy_payload": {
                        "root_mode": "workspace",
                        "write_asset_receipt": true,
                        "copy_dx_chrome_extension": true,
                        "dx_chrome_extension_source_root": "<local unpacked extension root>",
                        "overwrite_existing_files": false,
                        "include_file_preview": true
                    },
                    "requires_permission_for_writes": true,
                    "downloads_packages": false,
                    "runs_node": false,
                    "launches_browser": false,
                    "touches_real_browser_profiles": false
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
            "bootstrap_readiness_handoff": {
                "schema": AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA,
                "copy_action": "copy_agent_plugin_bootstrap_readiness",
                "send_action": "send_agent_plugin_bootstrap_readiness_to_agent",
                "read_only": true,
                "source": "WebPreview More menu",
                "purpose": "Copy or send the compact bootstrap readiness packet without copying the full plugin catalog or runtime-status output."
            },
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

fn browser_function_surfaces_manifest() -> Value {
    let screenshot_state = if cfg!(target_os = "windows") {
        "available"
    } else {
        "platform_unavailable"
    };
    let devtools_state = if cfg!(any(target_os = "windows", target_os = "macos")) {
        "available"
    } else {
        "platform_unavailable"
    };

    serde_json::json!({
        "schema": AGENT_BROWSER_FUNCTION_SURFACES_SCHEMA,
        "source": "WebPreview More menu and active WebPreview session snapshot",
        "backend": "web_preview",
        "surfaces": [
            {
                "id": "browser.screenshot.capture",
                "state": screenshot_state,
                "menu_action": "take_screenshot",
                "menu_label": "Take Screenshot",
                "output": ["clipboard_image", "agent_panel_image_attachment", "profile_screenshots_png"],
                "uses_page_script": false,
                "dispatches_input": false,
                "requires_interactive_unlock": false,
            },
            {
                "id": "browser.screenshot.area",
                "state": screenshot_state,
                "menu_action": "capture_selected_area_screenshot",
                "menu_label": "Capture Area",
                "overlay_completion_kind": "capture-area",
                "output": ["cropped_clipboard_image", "agent_panel_image_attachment", "profile_screenshots_png"],
                "uses_page_script": true,
                "dispatches_input": false,
                "requires_interactive_unlock": false,
            },
            {
                "id": "browser.screenshot.annotate",
                "state": screenshot_state,
                "menu_action": "annotate_screenshot",
                "menu_label": "Annotate Screenshot",
                "overlay_completion_kind": "annotated-screenshot",
                "output": ["clipboard_image", "agent_panel_image_attachment", "annotation_metadata_json"],
                "uses_page_script": true,
                "dispatches_input": false,
                "requires_interactive_unlock": false,
            },
            {
                "id": "browser.element.inspect",
                "state": "available_after_page_ready",
                "menu_action": "inspect_element",
                "menu_label": "Inspect Element",
                "overlay_completion_kind": "inspect-element",
                "output": ["agent_panel_selector", "agent_panel_html", "computed_style_summary", "optional_cropped_screenshot"],
                "uses_page_script": true,
                "dispatches_input": false,
                "requires_interactive_unlock": false,
            },
            {
                "id": "browser.devtools.open",
                "state": devtools_state,
                "menu_action": "open_devtools",
                "menu_label": "Open DevTools",
                "output": ["native_devtools_window"],
                "uses_page_script": false,
                "dispatches_input": false,
                "requires_interactive_unlock": false,
            },
            {
                "id": "browser.viewport.responsive",
                "state": "available",
                "menu_actions": ["viewport_full", "viewport_iphone_15", "viewport_ipad_air", "viewport_laptop", "viewport_rotate"],
                "executor_action": "run_permissioned_viewport_executor",
                "presets": [
                    {"mode": "full", "label": "Full", "width": null, "height": null},
                    {"mode": "fixed", "label": "iPhone 15", "width": 393, "height": 852},
                    {"mode": "fixed", "label": "iPad Air", "width": 820, "height": 1180},
                    {"mode": "fixed", "label": "Laptop", "width": 1280, "height": 900}
                ],
                "uses_page_script": false,
                "dispatches_input": false,
                "requires_interactive_unlock_for_executor": true,
            }
        ],
        "workflow": [
            "Use screenshot or selected-area capture for visual context.",
            "Use annotated screenshots when the user needs specific regions called out.",
            "Use Inspect Element after page ready for selector, HTML, style, rect, and screenshot context.",
            "Use responsive viewport menu entries for visual testing and the permissioned viewport executor only when explicit interactive unlock is active.",
            "Use DevTools for manual debugging on native WebPreview backends."
        ],
        "safety": {
            "read_only_or_agent_handoff": true,
            "dispatches_page_input": false,
            "mutates_external_browser_profiles": false
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
        "observability_profile": {
            "status": "code_complete_pending_windows_runtime_validation",
            "code_score": 99,
            "runtime_green_blocker": "Run one final Windows just run pass, exercise editor typing/WebPreview/native executor flows, fill the final result template, and import the filled result.",
            "proof_handoffs": {
                "validation_progress": "copy_agent_browser_executor_validation_progress",
                "final_bundle": "copy_agent_browser_final_validation_bundle",
                "final_result_template": "copy_agent_browser_final_validation_result_template",
                "final_result_import": "import_agent_browser_final_validation_result_from_clipboard",
                "final_result_send": "send_agent_browser_final_validation_result_to_agent"
            },
            "watch_surfaces": [
                "editor caret and typing latency",
                "WebPreview focus after navigation or reload",
                "native click/type/key/scroll/history/cache receipts",
                "managed Chrome execution receipts",
                "PC-use queue and runner receipts"
            ],
            "next_feature_set": "Agent Plugin Runtime Observability"
        },
        "function_surfaces_schema": AGENT_BROWSER_FUNCTION_SURFACES_SCHEMA,
        "function_surfaces": browser_function_surfaces_manifest(),
        "function_surfaces_handoff": {
            "schema": AGENT_BROWSER_FUNCTION_SURFACES_SCHEMA,
            "copy_action": "copy_agent_browser_function_surfaces",
            "send_action": "send_agent_browser_function_surfaces_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send the concrete screenshot, inspect, DevTools, and responsive viewport surface map without requiring the larger session or catalog."
        },
        "bootstrap_readiness_handoff": {
            "schema": AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA,
            "copy_action": "copy_agent_plugin_bootstrap_readiness",
            "send_action": "send_agent_plugin_bootstrap_readiness_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send the compact host dependency, managed-root, and managed-asset readiness packet before managed Chrome or PC-use provisioning."
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
            "payload_queue_item_schema": AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "payload_queue_inspection_schema": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
            "payload_queue_inspect_tool_name": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "payload_import_receipt_schema": "zed.web_preview.agent_browser_action_payload_import_receipt.v1",
            "executor_validation_progress_schema": AGENT_BROWSER_EXECUTOR_VALIDATION_PROGRESS_SCHEMA,
            "final_validation_bundle_schema": AGENT_BROWSER_FINAL_VALIDATION_BUNDLE_SCHEMA,
            "final_validation_result_schema": AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA,
            "final_validation_observability_schema": AGENT_BROWSER_FINAL_VALIDATION_OBSERVABILITY_SCHEMA,
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
                "Managed queue items should be checked with the queue inspection tool before WebPreview import when an Agent did not just queue the payload.",
                "Interactive executors still require unlock, fresh preflight, action-specific gates, and receipts.",
                "type_text requires non-empty payload.text and rejects selector mismatches when a selector is supplied."
            ]
        },
        "validation_progress_handoff": {
            "schema": AGENT_BROWSER_EXECUTOR_VALIDATION_PROGRESS_SCHEMA,
            "copy_action": "copy_agent_browser_executor_validation_progress",
            "send_action": "send_agent_browser_executor_validation_progress_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send grouped Browser executor evidence without requiring larger status/readiness/runbook packets."
        },
        "final_validation_bundle_handoff": {
            "schema": AGENT_BROWSER_FINAL_VALIDATION_BUNDLE_SCHEMA,
            "result_schema": AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA,
            "result_status_values": ["not_run", "pass", "fail", "blocked", "skipped"],
            "runtime_green_requires": [
                "manual_evidence_template.status == pass",
                "every required manual_evidence_template.checks entry has status == pass",
                "manual_evidence_template.overall_blocker == null",
                "executor_validation_progress.status == manual_windows_runtime_validation_ready"
            ],
            "copy_action": "copy_agent_browser_final_validation_bundle",
            "send_action": "send_agent_browser_final_validation_bundle_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send the canonical final Windows validation bundle before claiming runtime-green."
        },
        "final_validation_result_template_handoff": {
            "schema": AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA,
            "copy_action": "copy_agent_browser_final_validation_result_template",
            "send_action": "send_agent_browser_final_validation_result_template_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send only the fillable manual Windows result template before the final runtime proof."
        },
        "final_validation_result_handoff": {
            "schema": AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA,
            "import_action": "import_agent_browser_final_validation_result_from_clipboard",
            "copy_action": "copy_agent_browser_final_validation_result",
            "send_action": "send_agent_browser_final_validation_result_to_agent",
            "managed_result_dir": AGENT_BROWSER_FINAL_VALIDATION_DIR_NAME,
            "managed_result_file": AGENT_BROWSER_FINAL_VALIDATION_RESULT_FILE_NAME,
            "managed_result_archive_prefix": AGENT_BROWSER_FINAL_VALIDATION_RESULT_ARCHIVE_PREFIX,
            "runtime_status_field": "runtime_green_blocker_summary.latest_evidence.browser_final_validation_result",
            "copy_send_read_only": true,
            "import_writes_managed_result": true,
            "managed_roots_only": true,
            "source": "WebPreview More menu",
            "purpose": "Import, persist, copy, or send the filled manual Windows result after the final runtime proof."
        },
        "final_validation_observability_handoff": {
            "schema": AGENT_BROWSER_FINAL_VALIDATION_OBSERVABILITY_SCHEMA,
            "copy_action": "copy_agent_browser_final_validation_observability",
            "send_action": "send_agent_browser_final_validation_observability_to_agent",
            "read_only": true,
            "source": "WebPreview More menu",
            "purpose": "Copy or send the compact final proof-state and recovery-action summary without requiring the larger session or action manifest."
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
            capability("browser.function_surfaces", "available", "Copy or send the concrete WebPreview screenshot, inspect, DevTools, and responsive viewport surface map."),
            capability("browser.plugin_bootstrap_readiness", "available", "Copy or send compact Agent Plugin Runtime host, managed-root, and managed-asset readiness from WebPreview."),
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
            capability("browser.action.executor_validation_progress", "available", "Copy or send grouped Browser executor validation progress for final Windows proof without dispatching input."),
            capability("browser.validation.final_bundle", "available", "Copy or send the final Windows validation bundle tying readiness, progress, runbook, manifest, plugin catalog, and proof order together."),
            capability("browser.validation.final_result_template", "available", "Copy or send the fillable manual Windows result template with allowed status values and runtime-green requirements."),
            capability("browser.validation.final_result", "available", "Import, copy, or send the filled final Windows validation result after manual runtime proof."),
            capability("browser.validation.final_proof_state", "available", "Copy or send compact final proof-state observability and recovery actions without generating larger proof packets."),
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
        "observability_profile": {
            "status": "managed_adapter_ready_pending_windows_runtime_validation",
            "code_score": 94,
            "runtime_green_blocker": "Validate the managed Chrome queue, runner gate, Playwright adapter invoke path, and execution receipt inspection on Windows without touching real browser profiles.",
            "proof_handoffs": {
                "queue_inspection_tool": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "runner_gate_tool": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                "asset_provisioner_tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "adapter_prepare_tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "adapter_invoke_tool": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                "execution_inspect_tool": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                "webpreview_status_copy": "copy_managed_chrome_execution_status",
                "webpreview_status_send": "send_managed_chrome_execution_status_to_agent"
            },
            "watch_surfaces": [
                "managed workspace or Zed-data roots only",
                "asset provisioning receipts prove managed assets were prepared before Chrome execution",
                "real Chrome, Edge, and Firefox profiles stay untouched",
                "adapter execution remains limited to open_url, screenshot, set_viewport, and wait_for_selector",
                "click, type, key, and scroll stay blocked in the managed adapter",
                "runner and execution receipts stay inspectable from Agent and WebPreview catalogs"
            ],
            "next_action": "Run the readiness and receipt chain from queue inspection through execution inspection during final Windows validation."
        },
        "action_payload_contract": {
            "payload_tool_name": AGENT_CHROME_PAYLOAD_TOOL_NAME,
            "payload_queue_tool_name": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_queue_inspect_tool_name": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "runner_gate_tool_name": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
            "asset_provisioner_tool_name": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
            "asset_provisioning_receipt_file": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME,
            "asset_provisioning_result_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
            "asset_provisioning_receipt_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
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
                "The managed asset provisioner can write an asset receipt or copy a local unpacked DX Chrome extension into managed roots without downloads, Node, or Chrome launch.",
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
            capability("chrome.runtime.asset_provisioner", "available_requires_authorization", "Use prepare_agent_plugin_managed_assets to write an asset receipt or copy a local unpacked DX Chrome extension into managed roots without downloads, Node, or Chrome launch."),
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
        "status": "read_only_inspection_available",
        "default_enabled": true,
        "ships_with_editor": true,
        "scope": "zed_window_and_permissioned_desktop",
        "runtime": {
            "backend": "zed_window_runtime",
            "inspect_tool_name": AGENT_PC_USE_INSPECT_TOOL_NAME,
            "inspect_schema": "zed.agent_plugins.pc_use.zed_window_context.v1",
            "target_manifest_tool_name": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
            "target_manifest_schema": AGENT_PC_USE_TARGET_MANIFEST_SCHEMA,
            "target_snapshot_tool_name": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
            "target_snapshot_schema": AGENT_PC_USE_TARGET_SNAPSHOT_SCHEMA,
            "ui_snapshot_contract_tool_name": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
            "ui_snapshot_contract_schema": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_SCHEMA,
            "ui_snapshot_tool_name": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
            "ui_snapshot_schema": AGENT_PC_USE_UI_SNAPSHOT_SCHEMA,
            "payload_tool_name": AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            "payload_stage_tool_name": AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME,
            "payload_queue_tool_name": AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_queue_inspect_tool_name": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "runner_gate_tool_name": AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
            "runner_receipt_inspect_tool_name": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
            "webpreview_pc_use_status_copy_action": "copy_pc_use_status",
            "webpreview_pc_use_status_agent_action": "send_pc_use_status_to_agent",
            "webpreview_pc_use_status_schema": "zed.web_preview.pc_use_status.v1",
            "payload_schema": AGENT_PC_USE_PAYLOAD_SCHEMA,
            "payload_queue_item_schema": AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "payload_queue_inspection_schema": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
            "runner_receipt_schema": AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA,
            "runner_receipt_inspection_schema": AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
            "future_input_payload_requires_target_snapshot_id": true,
            "plugin_root": workspace_plugin_root
                .as_ref()
                .map(|root| root.join("pc-use"))
                .unwrap_or_else(|| default_plugin_root.join("pc-use"))
                .display()
                .to_string(),
            "os_wide_automation": "requires_separate_explicit_permission"
        },
        "observability_profile": {
            "status": "payload_and_receipt_gates_ready_pending_ui_executor_validation",
            "code_score": 90,
            "runtime_green_blocker": "Validate PC-use context, target, UI snapshot, payload queue, runner receipt, and WebPreview status handoff without screenshots, focus, input dispatch, process launch, or OS-wide control.",
            "proof_handoffs": {
                "context_tool": AGENT_PC_USE_INSPECT_TOOL_NAME,
                "target_manifest_tool": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
                "target_snapshot_tool": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
                "ui_snapshot_contract_tool": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
                "ui_snapshot_tool": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                "payload_queue_inspect_tool": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "runner_receipts_tool": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
                "webpreview_status_copy": "copy_pc_use_status",
                "webpreview_status_send": "send_pc_use_status_to_agent"
            },
            "watch_surfaces": [
                "read-only or managed-root-scoped operations only",
                "future UI snapshot target ids require matching snapshot receipt ids",
                "no OS-wide desktop control",
                "no focus, click, type, screenshot, or process launch in the current gate",
                "runner receipts stay auditable before any future executor exists"
            ],
            "next_action": "Validate the PC-use context, target, queue, runner, and receipt chain in the final Windows pass."
        },
        "capabilities": [
            capability("pc.zed_window.inspect_context", "available", "Use inspect_zed_window_context to read safe workspace and managed-root context before any future PC-use action."),
            capability("pc.zed_window.target_manifest", "available", "Use inspect_zed_pc_use_targets to read the supported Zed surfaces, action prerequisites, and target-id contract before composing PC-use payloads."),
            capability("pc.zed_window.target_snapshot", "available", "Use inspect_zed_pc_use_target_snapshot to read safe current workspace and project-panel target ids before composing PC-use payloads."),
            capability("pc.zed_window.ui_snapshot_contract", "available", "Use inspect_zed_pc_use_ui_snapshot_contract to read the future UI snapshot target namespaces required before focus, click, or type payloads can be input-ready."),
            capability("pc.zed_window.ui_snapshot", "available", "Use inspect_zed_pc_use_ui_snapshot to read the current partial Zed UI snapshot and live-UI gaps without screenshots, focus, click, or type dispatch."),
            capability("pc.zed_window.payload_compose", "available", "Use compose_zed_pc_use_action_payload to validate future Zed-window screenshot, focus, click, type, or inspect intents against the manifest and target-snapshot contract without dispatching input."),
            capability("pc.zed_window.payload_stage_clipboard", "available_requires_authorization", "Use stage_zed_pc_use_action_payload to write a validated Zed-window PC-use payload packet to the clipboard without dispatching input."),
            capability("pc.zed_window.payload_queue_managed", "available_requires_authorization", "Use queue_zed_pc_use_action_payload to write a validated Zed-window PC-use payload packet into managed workspace or Zed-data handoff roots without dispatching input."),
            capability("pc.zed_window.payload_queue_inspect", "available", "Use inspect_zed_pc_use_payload_queue to validate the latest managed PC-use payload handoff before any future importer or executor exists."),
            capability("pc.zed_window.runner_gate_receipt", "available_requires_authorization", "Use request_zed_pc_use_payload_run to write an auditable runner-gate receipt after validating the managed PC-use queue, without taking screenshots or dispatching input."),
            capability("pc.zed_window.runner_receipt_inspect", "available", "Use inspect_zed_pc_use_runner_receipts to read recent PC-use runner-gate receipts without taking screenshots, focusing Zed, or dispatching input."),
            capability("pc.zed_window.status_handoff", "available", "Use WebPreview Copy/Send Zed PC-use Status to hand the latest managed queue or runner receipt summary to the Agent Panel."),
            capability("pc.zed_window.screenshot", "planned", "Capture Zed-window screenshots for agent context."),
            capability("pc.zed_window.focus", "planned", "Focus Zed panes, panels, and tabs by safe editor-native handles."),
            capability("pc.zed_window.click", "planned_permission_gate", "Click within Zed surfaces only after permission and target preflight."),
            capability("pc.zed_window.type", "planned_permission_gate", "Type within Zed surfaces only after permission and target preflight."),
            capability("pc.zed_window.inspect_ui", "planned", "Read safe visible UI metadata for currently visible Zed surfaces."),
            capability("pc.desktop.os_wide", "blocked_by_default", "OS-wide desktop automation remains unavailable until the user explicitly enables it.")
        ],
        "safety": {
            "read_only_context_available": true,
            "read_only_target_manifest_available": true,
            "read_only_target_snapshot_available": true,
            "read_only_ui_snapshot_contract_available": true,
            "read_only_ui_snapshot_available": true,
            "read_only_payload_compose_available": true,
            "payload_stage_clipboard_available": true,
            "payload_queue_managed_available": true,
            "payload_queue_inspection_available": true,
            "runner_gate_receipt_available": true,
            "runner_receipt_inspection_available": true,
            "webpreview_pc_use_status_available": true,
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
    let managed_base_root = workspace_tools_root
        .clone()
        .unwrap_or_else(|| default_plugin_root.to_path_buf());
    let plugin_root = workspace_plugin_root
        .clone()
        .unwrap_or_else(|| default_plugin_root.to_path_buf());
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
    let bootstrap_manifest = plugin_root.join("agent-plugin-bootstrap.json");
    let asset_provisioning_receipt =
        plugin_root.join(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME);
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
    let bootstrap_manifest_schema = json_file_schema(&bootstrap_manifest);
    let bootstrap_manifest_ready =
        bootstrap_manifest_schema.as_deref() == Some(AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA);
    let asset_provisioning_receipt_schema = json_file_schema(&asset_provisioning_receipt);
    let asset_provisioning_receipt_ready = asset_provisioning_receipt_schema.as_deref()
        == Some(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA);

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
            "root.managed_base",
            "Managed runtime base root",
            managed_base_root.is_dir(),
            Some(managed_base_root.clone()),
            "provision_required",
            "Create this managed base root before writing plugin queues, assets, profiles, or receipts.",
        ),
        bootstrap_check(
            "root.plugin",
            "Managed plugin root",
            plugin_root.is_dir(),
            Some(plugin_root.clone()),
            "provision_required",
            "Create this managed plugin root before writing Browser, Chrome, or PC-use handoff files.",
        ),
        bootstrap_check(
            "root.playwright",
            "Managed Playwright root",
            playwright_root.is_dir(),
            Some(playwright_root.clone()),
            "provision_required",
            "Create this managed Playwright root before installing or preparing Playwright adapter files.",
        ),
        bootstrap_check(
            "root.dx_chrome_extension",
            "Managed DX Chrome extension root",
            dx_extension_root.is_dir(),
            Some(dx_extension_root.clone()),
            "provision_required",
            "Create this managed extension root before unpacking the DX Chrome extension.",
        ),
        bootstrap_check(
            "profile.managed_chrome",
            "Managed Chrome profile root",
            managed_profile_root.is_dir(),
            Some(managed_profile_root.clone()),
            "provision_required",
            "Create this profile root and never write into a user's real Chrome, Edge, or Firefox profile.",
        ),
        bootstrap_check(
            "asset.bootstrap_manifest",
            "Agent plugin bootstrap manifest",
            bootstrap_manifest_ready,
            Some(bootstrap_manifest.clone()),
            "provision_required",
            "Write the bootstrap manifest so future agents can verify the managed-root policy before provisioning assets.",
        ),
        bootstrap_check(
            "asset.provisioning_receipt",
            "Managed asset provisioning receipt",
            asset_provisioning_receipt_ready,
            Some(asset_provisioning_receipt.clone()),
            "provision_required",
            "Run the managed asset provisioner with write_asset_receipt=true so agents can prove managed assets were prepared before Chrome execution.",
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
            "Use the managed asset provisioner to copy a local unpacked DX Chrome extension before loading managed Chrome with the bridge.",
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
        "schema": AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "prepare_tool_name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
        "project_root": project_root.map(path_string),
        "phase_summary": bootstrap_phase_summary(&checks),
        "prepare_runtime_handoff": {
            "tool_name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
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
            "downloads_packages": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false
        },
        "prepare_runtime_request": bootstrap_prepare_request(status, project_root.is_some()),
        "asset_provisioning_plan": bootstrap_asset_provisioning_plan(
            status,
            if project_root.is_some() { "workspace" } else { "zed_data" },
            &bootstrap_manifest,
            bootstrap_manifest_ready,
            &playwright_root,
            &playwright_package,
            &playwright_adapter_root,
            &playwright_adapter_manifest,
            &playwright_runner_script,
            playwright_adapter_manifest_ready,
            &dx_extension_root,
            &dx_extension_manifest,
        ),
        "roots": {
            "zed_data_plugin_root": path_string(default_plugin_root),
            "managed_base_root": path_string(&managed_base_root),
            "plugin_root": path_string(&plugin_root),
            "workspace_plugin_root": workspace_plugin_root.as_ref().map(path_string),
            "workspace_tools_root": workspace_tools_root.as_ref().map(path_string),
            "playwright_root": path_string(&playwright_root),
            "playwright_adapter_root": path_string(&playwright_adapter_root),
            "playwright_adapter_manifest": path_string(&playwright_adapter_manifest),
            "playwright_runner_script": path_string(&playwright_runner_script),
            "dx_chrome_extension_root": path_string(&dx_extension_root),
            "managed_chrome_profile_root": path_string(&managed_profile_root),
            "bootstrap_manifest": path_string(&bootstrap_manifest),
        },
        "host": {
            "node": node.as_ref().map(path_string),
            "npm": npm.as_ref().map(path_string),
            "chrome_or_edge": browser.as_ref().map(path_string),
        },
        "manifest": {
            "path": path_string(&bootstrap_manifest),
            "expected_schema": AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA,
            "actual_schema": bootstrap_manifest_schema,
            "ready": bootstrap_manifest_ready,
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

fn bootstrap_asset_provisioning_plan(
    status: &str,
    root_mode: &str,
    bootstrap_manifest: &Path,
    bootstrap_manifest_ready: bool,
    playwright_root: &Path,
    playwright_package: &Path,
    playwright_adapter_root: &Path,
    playwright_adapter_manifest: &Path,
    playwright_runner_script: &Path,
    playwright_adapter_manifest_ready: bool,
    dx_extension_root: &Path,
    dx_extension_manifest: &Path,
) -> Value {
    let adapter_ready = playwright_adapter_manifest_ready && playwright_runner_script.is_file();

    serde_json::json!({
        "schema": AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA,
        "readiness_status": status,
        "safe_to_start_after_plan": status == "ready_for_managed_chrome_executor",
        "root_mode": root_mode,
        "operator_recipe": managed_asset_operator_recipe(root_mode),
        "steps": [
            {
                "id": "bootstrap.manifest",
                "label": "Agent plugin bootstrap manifest",
                "state": if bootstrap_manifest_ready { "ready" } else { "pending_prepare_runtime" },
                "path": path_string(bootstrap_manifest),
                "tool_name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
                "apply_payload": {
                    "root_mode": root_mode,
                    "create_managed_roots": true,
                    "write_bootstrap_manifest": true
                },
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": false,
                "launches_browser": false
            },
            {
                "id": "playwright.package",
                "label": "Managed Playwright package",
                "state": if playwright_package.is_file() { "ready" } else { "pending_manual_or_future_provisioner" },
                "managed_root": path_string(playwright_root),
                "expected_package_json": path_string(playwright_package),
                "requires_authorization": true,
                "runs_node": true,
                "downloads_packages": true,
                "launches_browser": false,
                "touches_real_browser_profiles": false
            },
            {
                "id": "playwright.adapter",
                "label": "Managed Chrome Playwright adapter",
                "state": if adapter_ready { "ready" } else { "pending_prepare_managed_adapter" },
                "tool_name": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "managed_root": path_string(playwright_adapter_root),
                "expected_manifest": path_string(playwright_adapter_manifest),
                "expected_runner": path_string(playwright_runner_script),
                "dry_run_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": false,
                    "include_script_preview": false
                },
                "write_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": true,
                    "include_script_preview": false
                },
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": false,
                "launches_browser": false
            },
            {
                "id": "dx.chrome_extension",
                "label": "Managed DX Chrome extension",
                "state": if dx_extension_manifest.is_file() { "ready" } else { "pending_manual_or_future_provisioner" },
                "managed_root": path_string(dx_extension_root),
                "expected_manifest": path_string(dx_extension_manifest),
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": true,
                "launches_browser": false,
                "touches_real_browser_profiles": false
            }
        ],
        "after_asset_provisioning_verification": {
            "catalog_tool": AgentPluginCatalogTool::NAME,
            "runtime_status_tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "asset_provisioner_tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
            "required_ready_checks": [
                "asset.bootstrap_manifest",
                "asset.provisioning_receipt",
                "asset.playwright_package",
                "asset.playwright_adapter_manifest",
                "asset.playwright_adapter_runner",
                "asset.dx_chrome_extension"
            ]
        },
        "safety": {
            "plan_is_metadata_only": true,
            "writes_files": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
            "requires_receipts_before_executor_actions": true
        }
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

fn bootstrap_phase_summary(checks: &[Value]) -> Value {
    let host = bootstrap_phase("host_dependencies", checks, &["workspace.", "host."]);
    let roots = bootstrap_phase("managed_roots", checks, &["root.", "profile."]);
    let assets = bootstrap_phase("managed_assets", checks, &["asset."]);
    let ready_phase_count = [&host, &roots, &assets]
        .into_iter()
        .filter(|phase| phase.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();

    serde_json::json!({
        "host_dependencies": host,
        "managed_roots": roots,
        "managed_assets": assets,
        "ready_phase_count": ready_phase_count,
        "total_phase_count": 3,
    })
}

fn bootstrap_phase(name: &str, checks: &[Value], prefixes: &[&str]) -> Value {
    let phase_checks = checks
        .iter()
        .filter(|check| {
            check
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| prefixes.iter().any(|prefix| id.starts_with(prefix)))
        })
        .cloned()
        .collect::<Vec<_>>();
    let total = phase_checks.len();
    let ready = phase_checks
        .iter()
        .filter(|check| check.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let missing = phase_checks
        .iter()
        .filter_map(|check| {
            let is_ready = check.get("ready").and_then(Value::as_bool).unwrap_or(false);
            (!is_ready)
                .then(|| check.get("id").and_then(Value::as_str).map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "name": name,
        "ready": ready == total && total > 0,
        "ready_check_count": ready,
        "total_check_count": total,
        "missing": missing,
    })
}

fn managed_asset_operator_recipe(root_mode: &str) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA,
        "root_mode": root_mode,
        "goal": "Prepare managed Browser and Chrome plugin assets in the safe order before any external Chrome execution.",
        "ordered_steps": [
            {
                "step": "inspect_bootstrap_readiness",
                "tool": AgentPluginCatalogTool::NAME,
                "recommended_payload": {
                    "include_bootstrap_readiness": true,
                    "include_workflow_recipes": true,
                    "include_validation_matrix": true
                },
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "prepare_managed_roots",
                "tool": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "create_managed_roots": true,
                    "write_bootstrap_manifest": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "write_asset_receipt",
                "tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_asset_receipt": true,
                    "copy_dx_chrome_extension": false,
                    "dx_chrome_extension_source_root": Value::Null,
                    "overwrite_existing_files": false,
                    "include_file_preview": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "copy_dx_chrome_extension_if_missing",
                "tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_asset_receipt": true,
                    "copy_dx_chrome_extension": true,
                    "dx_chrome_extension_source_root": "<local unpacked extension root>",
                    "overwrite_existing_files": false,
                    "include_file_preview": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "prepare_playwright_adapter",
                "tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": true,
                    "include_script_preview": false
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "inspect_runtime_status_again",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "include_bootstrap_readiness": true,
                    "include_observability_profiles": true,
                    "include_observability_digest": true,
                    "include_latest_handoff": true,
                    "include_next_actions": true
                },
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "final_windows_validation",
                "manual_command": "just run",
                "when": "only after managed asset status, adapter readiness, Browser/WebPreview receipts, managed Chrome receipts, and PC-use receipts are ready for a final runtime pass",
                "writes_files": false,
                "dispatches_input": "manual_validation_only"
            }
        ],
        "safety": {
            "recipe_is_metadata_only": true,
            "never_write_to_real_browser_profiles": true,
            "external_browser_launch_requires_later_permissioned_adapter_step": true,
            "input_dispatch_requires_webpreview_or_future_executor_receipts": true
        }
    })
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

fn json_file_schema(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()?
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_owned)
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
            "Run prepare_agent_plugin_managed_assets to write an asset receipt or copy a local unpacked DX Chrome extension into the managed agent plugin root.",
            "Keep managed Chrome profile data in the prepared profile root; never touch real user browser profiles.",
        ],
        _ => vec![
            "Chrome plugin bootstrap assets are present.",
            "Invoke the prepared Playwright adapter for safe actions, then inspect execution receipts before enabling input dispatch.",
        ],
    }
}

fn bootstrap_prepare_request(status: &str, workspace_available: bool) -> Value {
    let root_mode = if workspace_available {
        "workspace"
    } else {
        "zed_data"
    };
    let should_prepare = status == "ready_to_provision";

    serde_json::json!({
        "schema": AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA,
        "tool_name": PREPARE_AGENT_PLUGIN_RUNTIME_TOOL,
        "readiness_status": status,
        "should_call_prepare": should_prepare,
        "authorization_required": should_prepare,
        "recommended_payload": {
            "root_mode": root_mode,
            "create_managed_roots": should_prepare,
            "write_bootstrap_manifest": should_prepare
        },
        "dry_run_payload": {
            "root_mode": root_mode,
            "create_managed_roots": false,
            "write_bootstrap_manifest": false
        },
        "blocked_by": match status {
            "blocked_missing_host_dependencies" => vec!["host_dependencies"],
            "ready_for_managed_chrome_executor" => vec!["already_prepared"],
            _ => Vec::new(),
        },
        "after_prepare_verification": {
            "tool_name": AgentPluginCatalogTool::NAME,
            "payload": {
                "include_bootstrap_readiness": true,
                "include_bootstrap_plan": true,
                "include_planned_plugins": true
            }
        },
        "safety": {
            "writes_only_when_authorized": true,
            "downloads_packages": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false,
            "workspace_preferred_when_available": true,
        },
    })
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
