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
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME: &str = "prepare_agent_plugin_managed_assets";
pub const AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA: &str =
    "zed.agent_plugins.asset_provisioning_result.v1";
pub const AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA: &str =
    "zed.agent_plugins.asset_provisioning_receipt.v1";
pub const AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME: &str =
    "agent-plugin-asset-provisioning.json";
pub const AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.asset_readiness_summary.v1";

const MAX_EXTENSION_FILES: usize = 512;
const MAX_EXTENSION_FILE_BYTES: u64 = 5 * 1024 * 1024;
const EXCLUDED_SOURCE_DIRS: &[&str] = &[
    ".cache",
    ".cargo",
    ".git",
    ".github",
    ".kiro",
    ".vscode",
    "models",
    "node_modules",
    "target",
    "tmp",
    "tools",
    "trash",
    "vendor",
];

/// Prepares managed Browser/Chrome plugin assets without downloading packages or launching Chrome.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPluginAssetProvisionerToolInput {
    /// Prefer workspace-local assets under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Write a receipt summarizing current managed asset readiness and any local extension copy.
    pub write_asset_receipt: bool,
    /// Copy a local unpacked DX Chrome extension into the managed extension root.
    pub copy_dx_chrome_extension: bool,
    /// Local source root for an unpacked Chrome extension. It must contain a manifest.json.
    pub dx_chrome_extension_source_root: Option<String>,
    /// Replace destination files when copying the local extension.
    pub overwrite_existing_files: bool,
    /// Include a bounded preview of extension files that would be copied.
    pub include_file_preview: bool,
}

impl Default for AgentPluginAssetProvisionerToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            write_asset_receipt: false,
            copy_dx_chrome_extension: false,
            dx_chrome_extension_source_root: None,
            overwrite_existing_files: false,
            include_file_preview: true,
        }
    }
}

pub struct AgentPluginAssetProvisionerTool {
    project: Entity<Project>,
}

impl AgentPluginAssetProvisionerTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPluginAssetProvisionerTool {
    type Input = AgentPluginAssetProvisionerToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) if input.copy_dx_chrome_extension => "Provision DX Chrome extension".into(),
            Ok(input) if input.write_asset_receipt => "Write agent plugin asset receipt".into(),
            _ => "Plan agent plugin assets".into(),
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
            let plan = ManagedAssetProvisioningPlan::new(project_root, input.root_mode);
            plan.validate_managed_paths()?;
            plan.validate_source_request(&input)?;

            if input.write_asset_receipt || input.copy_dx_chrome_extension {
                let context =
                    ToolPermissionContext::new(Self::NAME, plan.permission_values(&input));
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
                format!("Failed to serialize asset provisioning result: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(
                if input.copy_dx_chrome_extension {
                    "Provisioned DX Chrome extension"
                } else if input.write_asset_receipt {
                    "Wrote agent plugin asset receipt"
                } else {
                    "Planned agent plugin assets"
                },
            ));

            Ok(output)
        })
    }
}

struct ManagedAssetProvisioningPlan {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    playwright_root: PathBuf,
    playwright_package_json: PathBuf,
    dx_extension_root: PathBuf,
    dx_extension_manifest: PathBuf,
    managed_profile_root: PathBuf,
    receipt_path: PathBuf,
}

impl ManagedAssetProvisioningPlan {
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
        let playwright_package_json = playwright_root
            .join("node_modules")
            .join("playwright")
            .join("package.json");
        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let dx_extension_manifest = dx_extension_root.join("manifest.json");
        let receipt_path = plugin_root.join(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME);

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            playwright_root,
            playwright_package_json,
            dx_extension_root,
            dx_extension_manifest,
            managed_profile_root,
            receipt_path,
        }
    }

    fn apply(&self, input: &AgentPluginAssetProvisionerToolInput) -> Result<Value, String> {
        let copy_report = self.dx_extension_copy_report(input)?;
        let mut wrote_receipt = false;

        let receipt = if input.write_asset_receipt {
            fs::create_dir_all(&self.plugin_root).map_err(|error| {
                format!(
                    "Failed to prepare agent plugin root {}: {error}",
                    self.plugin_root.display()
                )
            })?;
            let receipt = self.receipt_value(input, &copy_report, true);
            let receipt_json = serde_json::to_vec_pretty(&receipt)
                .map_err(|error| format!("Failed to serialize asset receipt: {error}"))?;
            fs::write(&self.receipt_path, receipt_json).map_err(|error| {
                format!(
                    "Failed to write asset provisioning receipt {}: {error}",
                    self.receipt_path.display()
                )
            })?;
            wrote_receipt = true;
            receipt
        } else {
            self.receipt_value(input, &copy_report, false)
        };
        let asset_readiness_summary =
            self.asset_readiness_summary(input, &copy_report, wrote_receipt);

        Ok(serde_json::json!({
            "schema": AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "root_mode": self.root_mode_label(),
                "applied": input.write_asset_receipt || input.copy_dx_chrome_extension,
                "copied_dx_chrome_extension": input.copy_dx_chrome_extension,
                "wrote_asset_receipt": wrote_receipt,
                "receipt_path": path_string(&self.receipt_path),
            },
            "asset_readiness_summary": asset_readiness_summary,
            "assets": self.assets_value(wrote_receipt),
            "dx_chrome_extension_copy": copy_report,
            "receipt": receipt,
            "next_actions": [
                "Run inspect_agent_plugin_runtime_status with include_bootstrap_readiness=true to verify the managed asset checks.",
                "Run prepare_managed_chrome_playwright_adapter with write_adapter_files=true if the adapter files are still missing.",
                "Install Playwright into the managed Playwright root with an explicit future installer or manual operator step before invoking managed Chrome.",
                "Keep all managed Chrome execution on the prepared managed profile root."
            ],
            "safety": {
                "downloads_packages": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_browser_input": false,
                "touches_real_browser_profiles": false,
                "copy_scope": "local unpacked DX Chrome extension source into managed plugin root only",
            }
        }))
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [
            &self.plugin_root,
            &self.playwright_root,
            &self.playwright_package_json,
            &self.dx_extension_root,
            &self.dx_extension_manifest,
            &self.managed_profile_root,
            &self.receipt_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing asset provisioning path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn validate_source_request(
        &self,
        input: &AgentPluginAssetProvisionerToolInput,
    ) -> Result<(), String> {
        if !input.copy_dx_chrome_extension {
            return Ok(());
        }
        let source = dx_extension_source(input)?;
        if !source.is_dir() {
            return Err(format!(
                "DX Chrome extension source root is not a directory: {}",
                source.display()
            ));
        }
        let manifest = source.join("manifest.json");
        if !manifest.is_file() {
            return Err(format!(
                "DX Chrome extension source root must contain manifest.json: {}",
                manifest.display()
            ));
        }
        if source == self.dx_extension_root {
            return Err(
                "DX Chrome extension source and managed destination are the same path".to_string(),
            );
        }
        Ok(())
    }

    fn permission_values(&self, input: &AgentPluginAssetProvisionerToolInput) -> Vec<String> {
        let mut values = vec![
            path_string(&self.plugin_root),
            path_string(&self.dx_extension_root),
            path_string(&self.receipt_path),
        ];
        if let Some(source) = input.dx_chrome_extension_source_root.as_ref() {
            values.push(source.clone());
        }
        values
    }

    fn dx_extension_copy_report(
        &self,
        input: &AgentPluginAssetProvisionerToolInput,
    ) -> Result<Value, String> {
        let source = match dx_extension_source(input) {
            Ok(source) => source,
            Err(_) if !input.copy_dx_chrome_extension => {
                return Ok(serde_json::json!({
                    "status": "source_not_configured",
                    "source_root": Value::Null,
                    "destination_root": path_string(&self.dx_extension_root),
                    "planned_files": [],
                    "copied_files": [],
                    "skipped_files": [],
                    "truncated": false,
                }));
            }
            Err(error) => return Err(error),
        };
        let (files, truncated, mut skipped_files) = collect_extension_files(&source)?;
        let mut planned_files = Vec::new();
        let mut copied_files = Vec::new();

        for file in files {
            let metadata = file.metadata().map_err(|error| {
                format!(
                    "Failed to read DX extension file metadata {}: {error}",
                    file.display()
                )
            })?;
            let relative = file.strip_prefix(&source).map_err(|error| {
                format!(
                    "Failed to resolve DX extension relative path {}: {error}",
                    file.display()
                )
            })?;
            let destination = self.dx_extension_root.join(relative);
            if !destination.starts_with(&self.dx_extension_root) {
                skipped_files.push(skip_value(relative, "outside_managed_destination"));
                continue;
            }

            let file_value = serde_json::json!({
                "relative_path": relative_path_string(relative),
                "source": path_string(&file),
                "destination": path_string(&destination),
                "bytes": metadata.len(),
            });
            if input.include_file_preview {
                planned_files.push(file_value);
            }

            if !input.copy_dx_chrome_extension {
                continue;
            }
            if metadata.len() > MAX_EXTENSION_FILE_BYTES {
                skipped_files.push(skip_value(relative, "file_too_large"));
                continue;
            }
            if destination.exists() && !input.overwrite_existing_files {
                skipped_files.push(skip_value(relative, "destination_exists"));
                continue;
            }

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "Failed to prepare DX extension destination {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::copy(&file, &destination).map_err(|error| {
                format!(
                    "Failed to copy DX extension file {} to {}: {error}",
                    file.display(),
                    destination.display()
                )
            })?;
            copied_files.push(path_string(&destination));
        }

        Ok(serde_json::json!({
            "status": if input.copy_dx_chrome_extension { "copy_attempted" } else { "dry_run" },
            "source_root": path_string(&source),
            "source_manifest": path_string(source.join("manifest.json")),
            "destination_root": path_string(&self.dx_extension_root),
            "destination_manifest": path_string(&self.dx_extension_manifest),
            "planned_file_count": planned_files.len(),
            "copied_file_count": copied_files.len(),
            "skipped_file_count": skipped_files.len(),
            "planned_files": planned_files,
            "copied_files": copied_files,
            "skipped_files": skipped_files,
            "truncated": truncated,
        }))
    }

    fn receipt_value(
        &self,
        input: &AgentPluginAssetProvisionerToolInput,
        copy_report: &Value,
        receipt_written_by_this_run: bool,
    ) -> Value {
        let asset_readiness_summary =
            self.asset_readiness_summary(input, copy_report, receipt_written_by_this_run);

        serde_json::json!({
            "schema": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
            "generated_at_ms": current_epoch_millis(),
            "root_mode": self.root_mode_label(),
            "requested": {
                "write_asset_receipt": input.write_asset_receipt,
                "copy_dx_chrome_extension": input.copy_dx_chrome_extension,
                "overwrite_existing_files": input.overwrite_existing_files,
                "has_dx_chrome_extension_source_root": input.dx_chrome_extension_source_root.is_some(),
            },
            "asset_readiness_summary": asset_readiness_summary,
            "assets": self.assets_value(receipt_written_by_this_run),
            "dx_chrome_extension_copy": copy_report,
            "safety": {
                "downloads_packages": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_browser_input": false,
                "touches_real_browser_profiles": false,
                "managed_profile_only": true,
            }
        })
    }

    fn asset_readiness_summary(
        &self,
        input: &AgentPluginAssetProvisionerToolInput,
        copy_report: &Value,
        receipt_written_by_this_run: bool,
    ) -> Value {
        let managed_base_root_ready = self.allowed_root.is_dir();
        let plugin_root_ready = self.plugin_root.is_dir();
        let playwright_root_ready = self.playwright_root.is_dir();
        let playwright_package_ready = self.playwright_package_json.is_file();
        let dx_extension_manifest_ready = self.dx_extension_manifest.is_file();
        let managed_profile_root_ready = self.managed_profile_root.is_dir();
        let asset_receipt_ready = receipt_written_by_this_run || self.receipt_path.is_file();
        let skipped_file_count = copy_report
            .get("skipped_file_count")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let copy_truncated = copy_report
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut blockers = Vec::new();
        if !managed_base_root_ready {
            blockers.push("missing_managed_base_root");
        }
        if !plugin_root_ready {
            blockers.push("missing_managed_plugin_root");
        }
        if !playwright_root_ready {
            blockers.push("missing_managed_playwright_root");
        }
        if !playwright_package_ready {
            blockers.push("missing_playwright_package");
        }
        if !dx_extension_manifest_ready {
            blockers.push("missing_dx_chrome_extension_manifest");
        }
        if !managed_profile_root_ready {
            blockers.push("missing_managed_chrome_profile_root");
        }
        if !asset_receipt_ready {
            blockers.push("missing_asset_provisioning_receipt");
        }

        let mut warnings = Vec::new();
        if skipped_file_count > 0 {
            warnings.push("dx_extension_copy_skipped_files");
        }
        if copy_truncated {
            warnings.push("dx_extension_copy_preview_truncated");
        }
        if input.copy_dx_chrome_extension && !dx_extension_manifest_ready {
            warnings.push("dx_extension_copy_did_not_produce_manifest");
        }

        let ready_flags = [
            managed_base_root_ready,
            plugin_root_ready,
            playwright_root_ready,
            playwright_package_ready,
            dx_extension_manifest_ready,
            managed_profile_root_ready,
            asset_receipt_ready,
        ];
        let ready_count = ready_flags.iter().filter(|ready| **ready).count();
        let next_actions = asset_readiness_next_actions(&blockers, &warnings);
        let status = if blockers.is_empty() {
            "ready_for_managed_chrome_validation"
        } else {
            "blocked_missing_managed_assets"
        };

        serde_json::json!({
            "schema": AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA,
            "status": status,
            "ready": blockers.is_empty(),
            "ready_count": ready_count,
            "required_count": ready_flags.len(),
            "required": {
                "managed_base_root": managed_base_root_ready,
                "managed_plugin_root": plugin_root_ready,
                "managed_playwright_root": playwright_root_ready,
                "playwright_package": playwright_package_ready,
                "dx_chrome_extension_manifest": dx_extension_manifest_ready,
                "managed_chrome_profile_root": managed_profile_root_ready,
                "asset_provisioning_receipt": asset_receipt_ready,
            },
            "blockers": blockers,
            "warnings": warnings,
            "next_actions": next_actions,
            "copy_status": copy_report.get("status").and_then(Value::as_str).unwrap_or("unknown"),
        })
    }

    fn assets_value(&self, receipt_written_by_this_run: bool) -> Value {
        serde_json::json!({
            "playwright": {
                "managed_root": path_string(&self.playwright_root),
                "expected_package_json": path_string(&self.playwright_package_json),
                "package_ready": self.playwright_package_json.is_file(),
                "installer_runs_in_this_tool": false,
            },
            "dx_chrome_extension": {
                "managed_root": path_string(&self.dx_extension_root),
                "expected_manifest": path_string(&self.dx_extension_manifest),
                "manifest_ready": self.dx_extension_manifest.is_file(),
            },
            "managed_chrome_profile": {
                "managed_root": path_string(&self.managed_profile_root),
                "root_ready": self.managed_profile_root.is_dir(),
                "real_browser_profiles_touched": false,
            },
            "receipt": {
                "path": path_string(&self.receipt_path),
                "ready": receipt_written_by_this_run || self.receipt_path.is_file(),
                "schema": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
            }
        })
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

fn asset_readiness_next_actions(
    blockers: &[&'static str],
    warnings: &[&'static str],
) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if blockers.iter().any(|blocker| {
        matches!(
            *blocker,
            "missing_managed_base_root"
                | "missing_managed_plugin_root"
                | "missing_managed_playwright_root"
                | "missing_managed_chrome_profile_root"
        )
    }) {
        actions.push(
            "Run prepare_agent_plugin_runtime with create_managed_roots=true before preparing assets.",
        );
    }
    if blockers.contains(&"missing_asset_provisioning_receipt") {
        actions.push("Run prepare_agent_plugin_managed_assets with write_asset_receipt=true.");
    }
    if blockers.contains(&"missing_dx_chrome_extension_manifest") {
        actions.push(
            "Provide dx_chrome_extension_source_root and set copy_dx_chrome_extension=true to copy a local unpacked DX Chrome extension.",
        );
    }
    if blockers.contains(&"missing_playwright_package") {
        actions.push(
            "Install Playwright into the managed Playwright root through an explicit installer or manual operator step before invoking managed Chrome.",
        );
    }
    if warnings.contains(&"dx_extension_copy_skipped_files") {
        actions.push(
            "Inspect skipped DX extension files before rerunning with overwrite_existing_files=true.",
        );
    }
    if actions.is_empty() {
        actions.push(
            "Run inspect_agent_plugin_runtime_status with include_observability_profiles=true and complete the final Windows validation pass.",
        );
    }
    actions
}

fn dx_extension_source(input: &AgentPluginAssetProvisionerToolInput) -> Result<PathBuf, String> {
    input
        .dx_chrome_extension_source_root
        .as_ref()
        .map(|source| PathBuf::from(source.as_str()))
        .ok_or_else(|| {
            "dx_chrome_extension_source_root is required to copy the DX Chrome extension"
                .to_string()
        })
}

fn collect_extension_files(root: &Path) -> Result<(Vec<PathBuf>, bool, Vec<Value>), String> {
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut truncated = false;
    collect_extension_files_inner(root, root, &mut files, &mut skipped, &mut truncated)?;
    Ok((files, truncated, skipped))
}

fn collect_extension_files_inner(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    skipped: &mut Vec<Value>,
    truncated: &mut bool,
) -> Result<(), String> {
    if *truncated {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "Failed to read DX extension directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        if files.len() >= MAX_EXTENSION_FILES {
            *truncated = true;
            return Ok(());
        }
        let entry = entry.map_err(|error| read_dir_entry_error(directory, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to inspect DX extension path {}: {error}",
                path.display()
            )
        })?;
        if file_type.is_symlink() {
            let relative = path.strip_prefix(root).unwrap_or(path.as_path());
            skipped.push(skip_value(relative, "symlink_skipped"));
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if EXCLUDED_SOURCE_DIRS
                .iter()
                .any(|excluded| *excluded == name.as_ref())
            {
                let relative = path.strip_prefix(root).unwrap_or(path.as_path());
                skipped.push(skip_value(relative, "excluded_directory"));
                continue;
            }
            collect_extension_files_inner(root, &path, files, skipped, truncated)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn skip_value(path: &Path, reason: &str) -> Value {
    serde_json::json!({
        "relative_path": relative_path_string(path),
        "reason": reason,
    })
}

fn read_dir_entry_error(directory: &Path, error: io::Error) -> String {
    format!(
        "Failed to read DX extension directory entry under {}: {error}",
        directory.display()
    )
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

fn relative_path_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
