use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::runtime::FlowLocalRuntimeSummary;

use super::export::VALIDATED_RELEASE_COMMANDS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowReleaseTaskStatus {
    Ready,
    PendingExternal,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowReleaseFileRecord {
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowReleaseTask {
    pub key: String,
    pub status: FlowReleaseTaskStatus,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowReleaseSummary {
    pub project: String,
    pub crate_version: String,
    pub generated_at_unix_ms: u128,
    pub repo_root: String,
    pub device_tier: String,
    pub selected_text_model: Option<String>,
    pub selected_stt_model: Option<String>,
    pub selected_tts_model: Option<String>,
    pub production_bundle_ready: bool,
    pub production_bundle_files: Vec<FlowReleaseFileRecord>,
    pub browser_release_artifacts: Vec<FlowReleaseFileRecord>,
    pub validated_commands: Vec<String>,
    pub external_tasks: Vec<FlowReleaseTask>,
    pub notes: Vec<String>,
}

impl FlowReleaseSummary {
    pub fn for_repo(
        summary: &FlowLocalRuntimeSummary,
        repo_root: impl AsRef<Path>,
    ) -> Result<Self> {
        let repo_root = repo_root.as_ref();
        let production_bundle_files = collect_records(
            repo_root,
            &[
                "configs/production/dx-desktop.json",
                "configs/production/browser-extension.json",
                "configs/production/zed-fork.json",
                "configs/production/codex-fork.json",
                "configs/production/zeroclaw-fork.json",
                "configs/production/manifest.json",
                "configs/production/README.txt",
            ],
        )?;
        let browser_release_artifacts = collect_records(
            repo_root,
            &[
                "extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip",
                "extensions/flow-webext/artifacts/flow-webext-firefox-v0.1.0.zip",
                "extensions/flow-webext/artifacts/flow-webext-safari-v0.1.0.zip",
            ],
        )?;

        let production_bundle_ready = production_bundle_files.iter().all(|file| file.exists);

        Ok(Self {
            project: "flow".to_string(),
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            repo_root: repo_root.display().to_string(),
            device_tier: format!("{:?}", summary.device_profile.tier),
            selected_text_model: summary.chat.model_key.clone(),
            selected_stt_model: summary.speech_to_text.model_key.clone(),
            selected_tts_model: summary.text_to_speech.model_key.clone(),
            production_bundle_ready,
            production_bundle_files,
            browser_release_artifacts,
            validated_commands: VALIDATED_RELEASE_COMMANDS
                .iter()
                .map(|command| (*command).to_string())
                .collect(),
            external_tasks: vec![
                FlowReleaseTask {
                    key: "firebase-project-linking".to_string(),
                    status: FlowReleaseTaskStatus::PendingExternal,
                    note: "Run firebase login, select the production Firebase project, and apply the project env values outside the repository.".to_string(),
                },
                FlowReleaseTask {
                    key: "chromium-store-publish".to_string(),
                    status: FlowReleaseTaskStatus::PendingExternal,
                    note: "Upload the packaged Chromium zip to the Chrome Web Store or Edge Add-ons dashboard.".to_string(),
                },
                FlowReleaseTask {
                    key: "firefox-amo-publish".to_string(),
                    status: FlowReleaseTaskStatus::PendingExternal,
                    note: "Upload the packaged Firefox zip to addons.mozilla.org with the reviewed listing assets.".to_string(),
                },
                FlowReleaseTask {
                    key: "safari-xcode-package".to_string(),
                    status: FlowReleaseTaskStatus::PendingExternal,
                    note: "Wrap the Safari WebExtension assets in Xcode, sign with the Apple team, and submit through App Store Connect.".to_string(),
                },
            ],
            notes: vec![
                "The repository scope is code-complete and validated; the remaining tasks are vendor-side release operations.".to_string(),
                "This summary is intended for client handoff, operator review, and downstream host integration.".to_string(),
            ],
        })
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# Flow Release Summary".to_string(),
            String::new(),
            format!("- crate version: `{}`", self.crate_version),
            format!("- repo root: `{}`", self.repo_root),
            format!("- device tier: `{}`", self.device_tier),
            format!(
                "- selected text model: `{}`",
                self.selected_text_model.as_deref().unwrap_or("none")
            ),
            format!(
                "- selected STT model: `{}`",
                self.selected_stt_model.as_deref().unwrap_or("none")
            ),
            format!(
                "- selected TTS model: `{}`",
                self.selected_tts_model.as_deref().unwrap_or("none")
            ),
            format!(
                "- production bundle ready: `{}`",
                self.production_bundle_ready
            ),
            String::new(),
            "## Production Bundle Files".to_string(),
        ];

        for file in &self.production_bundle_files {
            lines.push(format!(
                "- `{}`: {}",
                file.path,
                if file.exists { "ready" } else { "missing" }
            ));
        }

        lines.push(String::new());
        lines.push("## Browser Release Artifacts".to_string());
        for file in &self.browser_release_artifacts {
            lines.push(format!(
                "- `{}`: {}",
                file.path,
                if file.exists { "ready" } else { "missing" }
            ));
        }

        lines.push(String::new());
        lines.push("## Validated Commands".to_string());
        for command in &self.validated_commands {
            lines.push(format!("- `{}`", command));
        }

        lines.push(String::new());
        lines.push("## External Release Tasks".to_string());
        for task in &self.external_tasks {
            lines.push(format!(
                "- `{}`: {:?} - {}",
                task.key, task.status, task.note
            ));
        }

        lines.join("\n")
    }
}

pub fn export_release_summary(
    summary: &FlowLocalRuntimeSummary,
    repo_root: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<FlowReleaseSummary> {
    let repo_root = repo_root.as_ref();
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let release_summary = FlowReleaseSummary::for_repo(summary, repo_root)?;
    fs::write(
        output_dir.join("flow-release-summary.json"),
        release_summary.to_pretty_json()?,
    )?;
    fs::write(
        output_dir.join("FLOW_RELEASE_HANDOFF.md"),
        release_summary.to_markdown(),
    )?;

    Ok(release_summary)
}

fn collect_records(
    repo_root: &Path,
    relative_paths: &[&str],
) -> Result<Vec<FlowReleaseFileRecord>> {
    relative_paths
        .iter()
        .map(|relative_path| collect_record(repo_root.join(relative_path), relative_path))
        .collect()
}

fn collect_record(absolute_path: PathBuf, relative_path: &str) -> Result<FlowReleaseFileRecord> {
    let metadata = fs::metadata(&absolute_path).ok();
    let sha256 = if metadata.is_some() {
        read_sha256_sidecar(&absolute_path)?
    } else {
        None
    };

    Ok(FlowReleaseFileRecord {
        path: relative_path.replace('\\', "/"),
        exists: metadata.is_some(),
        bytes: metadata.map(|item| item.len()),
        sha256,
    })
}

fn read_sha256_sidecar(path: &Path) -> Result<Option<String>> {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(None);
    };
    let sidecar = path.with_file_name(format!("{file_name}.sha256"));
    if !sidecar.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(sidecar)?;
    Ok(contents.split_whitespace().next().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        ComputeBackend, DeviceProfile, DeviceTier, FlowLocalRuntime, GraphicsDevice,
    };

    fn low_end_runtime_summary() -> FlowLocalRuntimeSummary {
        FlowLocalRuntime::for_device_profile(DeviceProfile {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            cpu_model: "Test CPU".to_string(),
            physical_cores: 4,
            logical_cores: 8,
            total_memory_bytes: 6 * 1024 * 1024 * 1024,
            available_memory_bytes: 4 * 1024 * 1024 * 1024,
            battery_powered: None,
            thermal_class: None,
            graphics: vec![GraphicsDevice {
                name: "Integrated GPU".to_string(),
                vendor: Some("intel".to_string()),
                vram_bytes: None,
                integrated: true,
                backends: vec![ComputeBackend::Cpu],
            }],
            tier: DeviceTier::Low,
        })
        .unwrap()
        .summary()
        .clone()
    }

    #[test]
    fn release_summary_detects_expected_files() {
        let root = std::env::temp_dir().join(format!(
            "flow_release_summary_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        fs::create_dir_all(root.join("configs/production")).unwrap();
        fs::create_dir_all(root.join("extensions/flow-webext/artifacts")).unwrap();
        fs::write(root.join("configs/production/manifest.json"), "{}").unwrap();
        fs::write(
            root.join("extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip"),
            b"zip",
        )
        .unwrap();
        fs::write(
            root.join("extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip.sha256"),
            "abc123  flow-webext-chromium-v0.1.0.zip",
        )
        .unwrap();

        let summary = FlowReleaseSummary::for_repo(&low_end_runtime_summary(), &root).unwrap();
        assert_eq!(summary.selected_text_model.as_deref(), Some("qwen3-0.6b"));
        assert!(
            summary
                .browser_release_artifacts
                .iter()
                .any(|artifact| artifact.sha256.as_deref() == Some("abc123"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn export_release_summary_writes_handoff_files() {
        let root = std::env::temp_dir().join(format!(
            "flow_release_export_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let output = root.join("release");

        fs::create_dir_all(root.join("configs/production")).unwrap();
        fs::create_dir_all(root.join("extensions/flow-webext/artifacts")).unwrap();

        let summary = export_release_summary(&low_end_runtime_summary(), &root, &output).unwrap();
        assert!(!summary.notes.is_empty());
        assert!(output.join("flow-release-summary.json").exists());
        assert!(output.join("FLOW_RELEASE_HANDOFF.md").exists());

        let _ = fs::remove_dir_all(root);
    }
}
