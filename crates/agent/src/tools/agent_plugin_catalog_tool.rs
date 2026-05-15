use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// Lists the built-in DX/Zed agent plugin catalog for browser, Chrome, and PC-use workflows.
///
/// Use this before trying to control the in-app WebPreview browser, external Chrome through
/// Playwright and the DX Chrome extension, or future permissioned PC UI tools. The tool is
/// read-only and returns capability manifests, bootstrap roots, and safety requirements.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPluginCatalogToolInput {
    /// Include plugins that are planned or require bootstrap before they can execute.
    pub include_planned_plugins: bool,
    /// Include install roots and download/update policy for default plugin provisioning.
    pub include_bootstrap_plan: bool,
}

impl Default for AgentPluginCatalogToolInput {
    fn default() -> Self {
        Self {
            include_planned_plugins: true,
            include_bootstrap_plan: true,
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
            "available_to": [
                "agent_panel",
                "subagents",
                "acp_threads",
                "web_preview_agent_handoff"
            ],
            "bootstrap_plan": input.include_bootstrap_plan.then(|| serde_json::json!({
                "default_download": true,
                "download_policy": "download_or_update_on_first_use",
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
        "capabilities": [
            capability("browser.sessions.list", "available", "List open WebPreview sessions and workspace inventory."),
            capability("browser.session.snapshot", "available", "Read the active WebPreview session metadata, bounds, profile, URL, and policy."),
            capability("browser.page.diagnostics", "available", "Collect ready state, title, URL, DOM counts, and page metadata."),
            capability("browser.dom.snapshot", "available", "Collect a bounded DOM tree snapshot for model context."),
            capability("browser.runtime.events", "available", "Read bounded console, page-error, fetch, and XHR event buffers."),
            capability("browser.screenshot.capture", "available", "Capture WebPreview screenshots for Agent Panel attachments."),
            capability("browser.action.reload", "available_when_unlocked", "Reload through the permissioned WebPreview executor shell."),
            capability("browser.action.click", "planned_executor", "Click visible page targets after unlock, fresh preflight, and receipt logging."),
            capability("browser.action.type", "planned_executor", "Type into page inputs after unlock, fresh preflight, and receipt logging."),
            capability("browser.action.key", "planned_executor", "Send key presses after unlock, fresh preflight, and receipt logging."),
            capability("browser.action.scroll", "planned_executor", "Scroll page or element targets after unlock, fresh preflight, and receipt logging.")
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
        "capabilities": [
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
