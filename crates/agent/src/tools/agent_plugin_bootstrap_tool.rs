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

const AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_prepare_request.v1";
const AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA: &str = "zed.agent_plugins.bootstrap_asset_plan.v1";
const AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME: &str = "prepare_managed_chrome_playwright_adapter";
const AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME: &str = "zed-managed-chrome-runner";
const AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME: &str = "managed_chrome_runner.mjs";

/// Prepares managed roots for the DX/Zed Agent Plugin Runtime without touching real browser profiles.
///
/// By default this is a dry run. Set `create_managed_roots` to create the managed directories and
/// `write_bootstrap_manifest` to write a small plan file. This tool never downloads packages,
/// launches Chrome, or writes into Chrome/Edge/Firefox user profiles.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPluginBootstrapToolInput {
    /// Prefer workspace-local tools under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPluginBootstrapRootMode,
    /// Create the managed plugin, Playwright, DX extension, and browser profile directories.
    pub create_managed_roots: bool,
    /// Write `agent-plugin-bootstrap.json` inside the managed plugin root.
    pub write_bootstrap_manifest: bool,
}

impl Default for AgentPluginBootstrapToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPluginBootstrapRootMode::Workspace,
            create_managed_roots: false,
            write_bootstrap_manifest: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentPluginBootstrapRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct AgentPluginBootstrapTool {
    project: Entity<Project>,
}

impl AgentPluginBootstrapTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPluginBootstrapTool {
    type Input = AgentPluginBootstrapToolInput;
    type Output = String;

    const NAME: &'static str = "prepare_agent_plugin_runtime";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) if input.create_managed_roots || input.write_bootstrap_manifest => {
                "Prepare agent plugin runtime".into()
            }
            _ => "Plan agent plugin runtime setup".into(),
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
            let plan = BootstrapPlan::new(project_root, input.root_mode);

            if input.create_managed_roots || input.write_bootstrap_manifest {
                let context = ToolPermissionContext::new(
                    Self::NAME,
                    plan.permission_values(input.write_bootstrap_manifest),
                );
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

            let result = plan.apply(input)?;
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize bootstrap result: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title(
                    if result
                        .pointer("/result/applied")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "Prepared agent plugin runtime"
                    } else {
                        "Planned agent plugin runtime setup"
                    },
                ),
            );

            Ok(output)
        })
    }
}

struct BootstrapPlan {
    root_mode: AgentPluginBootstrapRootMode,
    project_root: Option<PathBuf>,
    managed_base_root: PathBuf,
    plugin_root: PathBuf,
    playwright_root: PathBuf,
    dx_extension_root: PathBuf,
    managed_profile_root: PathBuf,
    manifest_path: PathBuf,
}

impl BootstrapPlan {
    fn new(project_root: Option<PathBuf>, root_mode: AgentPluginBootstrapRootMode) -> Self {
        let zed_plugin_root = data_dir().join("agent-plugins");
        let use_workspace =
            matches!(root_mode, AgentPluginBootstrapRootMode::Workspace) && project_root.is_some();

        let (managed_base_root, plugin_root, playwright_root, managed_profile_root) =
            if use_workspace {
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

        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let manifest_path = plugin_root.join("agent-plugin-bootstrap.json");

        Self {
            root_mode,
            project_root,
            managed_base_root,
            plugin_root,
            playwright_root,
            dx_extension_root,
            managed_profile_root,
            manifest_path,
        }
    }

    fn apply(&self, input: AgentPluginBootstrapToolInput) -> Result<Value, String> {
        self.validate_managed_roots()?;

        let directories = self.directories();
        let mut created_or_existing = Vec::new();
        if input.create_managed_roots || input.write_bootstrap_manifest {
            for directory in &directories {
                fs::create_dir_all(directory).map_err(|error| {
                    format!("Failed to create {}: {error}", directory.display())
                })?;
                created_or_existing.push(path_string(directory));
            }
        }

        let mut wrote_manifest = false;
        if input.write_bootstrap_manifest {
            let manifest = self.manifest_value(&input, &directories);
            let manifest_json = serde_json::to_vec_pretty(&manifest)
                .map_err(|error| format!("Failed to serialize bootstrap manifest: {error}"))?;
            fs::write(&self.manifest_path, manifest_json).map_err(|error| {
                format!(
                    "Failed to write bootstrap manifest {}: {error}",
                    self.manifest_path.display()
                )
            })?;
            wrote_manifest = true;
        }

        Ok(serde_json::json!({
            "schema": "zed.agent_plugins.bootstrap_prepare.v1",
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "root_mode": self.root_mode_label(),
                "applied": input.create_managed_roots || input.write_bootstrap_manifest,
                "created_or_existing_directories": created_or_existing,
                "wrote_manifest": wrote_manifest,
                "manifest_path": path_string(&self.manifest_path),
            },
            "prepare_request": self.prepare_request_value(&input),
            "asset_provisioning_plan": self.asset_provisioning_plan(),
            "roots": self.roots_value(),
            "planned_directories": directories.iter().map(path_string).collect::<Vec<_>>(),
            "next_actions": [
                "Install Playwright into the managed Playwright root.",
                "Download or unpack the DX Chrome extension into the managed extension root.",
                "Keep Chrome automation on managed profiles only; never write to user browser profiles.",
                "After assets exist, run list_agent_plugins to verify bootstrap readiness."
            ],
            "safety": {
                "downloads_performed": false,
                "browser_launched": false,
                "real_browser_profiles_touched": false,
                "write_scope": "managed Zed data roots or workspace tools roots only",
            },
        }))
    }

    fn asset_provisioning_plan(&self) -> Value {
        let root_mode = self.request_root_mode();
        let playwright_package = self
            .playwright_root
            .join("node_modules")
            .join("playwright")
            .join("package.json");
        let playwright_adapter_root = self
            .playwright_root
            .join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME);
        let playwright_adapter_manifest = playwright_adapter_root.join("adapter-manifest.json");
        let playwright_runner_script =
            playwright_adapter_root.join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME);
        let dx_extension_manifest = self.dx_extension_root.join("manifest.json");
        let adapter_ready =
            playwright_adapter_manifest.is_file() && playwright_runner_script.is_file();
        let ready = self.manifest_path.is_file()
            && playwright_package.is_file()
            && adapter_ready
            && dx_extension_manifest.is_file();

        serde_json::json!({
            "schema": AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA,
            "readiness_status": if ready { "ready_for_managed_chrome_executor" } else { "ready_to_provision" },
            "safe_to_start_after_plan": ready,
            "root_mode": root_mode,
            "steps": [
                {
                    "id": "bootstrap.manifest",
                    "label": "Agent plugin bootstrap manifest",
                    "state": if self.manifest_path.is_file() { "ready" } else { "pending_prepare_runtime" },
                    "path": path_string(&self.manifest_path),
                    "tool_name": AgentPluginBootstrapTool::NAME,
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
                    "managed_root": path_string(&self.playwright_root),
                    "expected_package_json": path_string(&playwright_package),
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
                    "managed_root": path_string(&playwright_adapter_root),
                    "expected_manifest": path_string(&playwright_adapter_manifest),
                    "expected_runner": path_string(&playwright_runner_script),
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
                    "managed_root": path_string(&self.dx_extension_root),
                    "expected_manifest": path_string(&dx_extension_manifest),
                    "requires_authorization": true,
                    "runs_node": false,
                    "downloads_packages": true,
                    "launches_browser": false,
                    "touches_real_browser_profiles": false
                }
            ],
            "after_asset_provisioning_verification": {
                "catalog_tool": "list_agent_plugins",
                "runtime_status_tool": "inspect_agent_plugin_runtime_status",
                "required_ready_checks": [
                    "asset.bootstrap_manifest",
                    "asset.playwright_package",
                    "asset.playwright_adapter_manifest",
                    "asset.playwright_adapter_runner",
                    "asset.dx_chrome_extension"
                ]
            },
            "safety": {
                "plan_is_metadata_only": true,
                "launches_browser": false,
                "dispatches_input": false,
                "touches_real_browser_profiles": false,
                "requires_receipts_before_executor_actions": true
            }
        })
    }

    fn prepare_request_value(&self, input: &AgentPluginBootstrapToolInput) -> Value {
        let request_root_mode = self.request_root_mode();

        serde_json::json!({
            "schema": AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA,
            "tool_name": AgentPluginBootstrapTool::NAME,
            "received_payload": {
                "root_mode": request_root_mode,
                "create_managed_roots": input.create_managed_roots,
                "write_bootstrap_manifest": input.write_bootstrap_manifest
            },
            "dry_run_payload": {
                "root_mode": request_root_mode,
                "create_managed_roots": false,
                "write_bootstrap_manifest": false
            },
            "apply_payload": {
                "root_mode": request_root_mode,
                "create_managed_roots": true,
                "write_bootstrap_manifest": true
            },
            "authorization_required_for_apply": true,
            "downloads_packages": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false,
            "after_prepare_verification": {
                "tool_name": "inspect_agent_plugin_runtime_status",
                "payload": {
                    "root_mode": request_root_mode,
                    "include_host_checks": true,
                    "include_bootstrap_readiness": true,
                    "include_latest_handoffs": true,
                    "include_next_actions": true
                }
            },
        })
    }

    fn permission_values(&self, write_manifest: bool) -> Vec<String> {
        let mut values = self
            .directories()
            .into_iter()
            .map(|path| path_string(&path))
            .collect::<Vec<_>>();
        if write_manifest {
            values.push(path_string(&self.manifest_path));
        }
        values
    }

    fn directories(&self) -> Vec<PathBuf> {
        vec![
            self.managed_base_root.clone(),
            self.plugin_root.clone(),
            self.playwright_root.clone(),
            self.dx_extension_root.clone(),
            self.managed_profile_root.clone(),
        ]
    }

    fn manifest_value(
        &self,
        input: &AgentPluginBootstrapToolInput,
        directories: &[PathBuf],
    ) -> Value {
        serde_json::json!({
            "schema": "zed.agent_plugins.bootstrap_manifest.v1",
            "generated_at_ms": current_epoch_millis(),
            "root_mode": self.root_mode_label(),
            "requested": {
                "create_managed_roots": input.create_managed_roots,
                "write_bootstrap_manifest": input.write_bootstrap_manifest,
            },
            "roots": self.roots_value(),
            "directories": directories.iter().map(path_string).collect::<Vec<_>>(),
            "download_policy": {
                "playwright": "download_or_update_on_first_use",
                "dx_chrome_extension": "download_or_update_on_first_use",
            },
            "profile_policy": {
                "managed_profile_only": true,
                "never_write_to_user_browser_profiles": true,
            },
        })
    }

    fn roots_value(&self) -> Value {
        serde_json::json!({
            "project_root": self.project_root.as_ref().map(path_string),
            "managed_base_root": path_string(&self.managed_base_root),
            "plugin_root": path_string(&self.plugin_root),
            "playwright_root": path_string(&self.playwright_root),
            "dx_chrome_extension_root": path_string(&self.dx_extension_root),
            "managed_chrome_profile_root": path_string(&self.managed_profile_root),
        })
    }

    fn validate_managed_roots(&self) -> Result<(), String> {
        let allowed_root = match self.root_mode {
            AgentPluginBootstrapRootMode::Workspace if self.project_root.is_some() => self
                .project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools"),
            _ => data_dir().join("agent-plugins"),
        };

        for path in self
            .directories()
            .into_iter()
            .chain(std::iter::once(self.manifest_path.clone()))
        {
            if !path.starts_with(&allowed_root) {
                return Err(format!(
                    "Refusing to prepare unmanaged path {} outside {}",
                    path.display(),
                    allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentPluginBootstrapRootMode::Workspace if self.project_root.is_some() => "workspace",
            AgentPluginBootstrapRootMode::Workspace => "zed_data_fallback",
            AgentPluginBootstrapRootMode::ZedData => "zed_data",
        }
    }

    fn request_root_mode(&self) -> &'static str {
        match self.root_mode {
            AgentPluginBootstrapRootMode::Workspace if self.project_root.is_some() => "workspace",
            _ => "zed_data",
        }
    }
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
