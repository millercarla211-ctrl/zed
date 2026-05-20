use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_source_attachment};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_SOURCE_ATTACHMENT_LATEST_FILE_NAME: &str = "latest-dx-source-attachment-receipt.json";

/// Prepare selected DX source sets as an Agent attachment manifest and optional receipt.
///
/// This tool reads workspace-managed source receipts and paths, packages them as source references,
/// and writes a compact attachment receipt. It never embeds binary media, fetches network pages,
/// runs tools, or mutates the referenced sources.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSourceAttachmentToolInput {
    /// Include visible workspace roots as directory sources.
    pub include_workspace_roots: bool,
    /// Include latest DX metasearch source-pack receipts as receipt sources.
    pub include_metasearch_source_packs: bool,
    /// Include produced media files from latest media execution receipts as file sources.
    pub include_media_outputs: bool,
    /// Include Forge restore preview directories as directory sources.
    pub include_forge_restore_previews: bool,
    /// Maximum sources to include per selected set. Clamped to 1..=12.
    pub max_sources_per_set: usize,
    /// Write a managed attachment receipt for reuse in later Agent turns.
    pub write_attachment_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxSourceAttachmentReceiptRootMode,
}

impl Default for DxSourceAttachmentToolInput {
    fn default() -> Self {
        Self {
            include_workspace_roots: true,
            include_metasearch_source_packs: true,
            include_media_outputs: true,
            include_forge_restore_previews: true,
            max_sources_per_set: 4,
            write_attachment_receipt: true,
            receipt_root_mode: DxSourceAttachmentReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSourceAttachmentReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxSourceAttachmentTool {
    project: Entity<Project>,
}

impl DxSourceAttachmentTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSourceAttachmentTool {
    type Input = DxSourceAttachmentToolInput;
    type Output = String;

    const NAME: &'static str = "prepare_dx_source_attachment";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Prepare DX source attachment".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let project = self.project.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let workspace_roots = cx.update(|cx| workspace_roots_for_project(&project, cx));
            let receipt_target = input.write_attachment_receipt.then(|| {
                DxSourceAttachmentReceiptTarget::new(
                    workspace_roots.first().cloned(),
                    input.receipt_root_mode,
                )
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("workspace_roots={}", input.include_workspace_roots),
                    format!("metasearch={}", input.include_metasearch_source_packs),
                    format!("media_outputs={}", input.include_media_outputs),
                    format!("forge_restore={}", input.include_forge_restore_previews),
                    format!("max_sources_per_set={}", input.max_sources_per_set),
                ];
                if let Some(receipt_target) = &receipt_target {
                    permission_values.push(path_string(&receipt_target.latest_path));
                    permission_values.push(path_string(&receipt_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response = dx_source_attachment::prepare_dx_source_attachment(
                DxSourceAttachmentReceiptTarget::request(
                    &input,
                    workspace_roots,
                    receipt_target.as_ref(),
                ),
            );

            if let Some(receipt_target) = receipt_target {
                response.source_attachment_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX source attachment receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Prepared DX source attachment: {} source(s)",
                response.summary.source_count
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX source attachment response: {error}")
            })
        })
    }
}

struct DxSourceAttachmentReceiptTarget {
    root_mode: DxSourceAttachmentReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSourceAttachmentReceiptTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxSourceAttachmentReceiptRootMode) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxSourceAttachmentReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-sources")
            }
            _ => data_dir().join("dx-sources"),
        };
        let receipt_dir = allowed_root.join("attachments");
        let latest_path = receipt_dir.join(DX_SOURCE_ATTACHMENT_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-source-attachment-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            receipt_dir,
            latest_path,
            archive_path,
        }
    }

    fn request(
        input: &DxSourceAttachmentToolInput,
        workspace_roots: Vec<PathBuf>,
        receipt_target: Option<&Self>,
    ) -> dx_source_attachment::DxSourceAttachmentRequest {
        dx_source_attachment::DxSourceAttachmentRequest {
            workspace_roots,
            selection: dx_source_attachment::DxSourceAttachmentSelection {
                workspace_roots: input.include_workspace_roots,
                metasearch_source_packs: input.include_metasearch_source_packs,
                media_outputs: input.include_media_outputs,
                forge_restore_previews: input.include_forge_restore_previews,
            },
            max_sources_per_set: input.max_sources_per_set,
            write_attachment_receipt: input.write_attachment_receipt,
            root_mode: receipt_target
                .map(Self::root_mode_label)
                .unwrap_or("no_receipt")
                .to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_source_attachment::DxSourceAttachment,
    ) -> Result<dx_source_attachment::DxSourceAttachmentReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_source_attachment::DX_SOURCE_ATTACHMENT_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSourceAttachmentTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "source_attachment": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_external_process": false,
                "runs_shell": false,
                "fetches_network": false,
                "embeds_binary_payloads": false,
                "mutates_workspace_sources": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this attachment receipt as the selected source manifest for the next Agent turn or serializer/RLM context preparation."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize source attachment receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX source attachment receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX source latest attachment receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX source attachment receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_source_attachment::DxSourceAttachmentReceipt {
            schema: dx_source_attachment::DX_SOURCE_ATTACHMENT_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            attachment_schema: response.schema,
            source_count: response.summary.source_count,
            estimated_tokens: response.summary.estimated_tokens,
            next_action:
                "Use the latest DX source attachment receipt as the Agent source manifest."
                    .to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX source attachment receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSourceAttachmentReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxSourceAttachmentReceiptRootMode::Workspace => "zed_data_fallback",
            DxSourceAttachmentReceiptRootMode::ZedData => "zed_data",
        }
    }
}

fn workspace_roots_for_project(project: &Entity<Project>, cx: &App) -> Vec<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .take(4)
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
        .collect()
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
