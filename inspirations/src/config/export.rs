use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::runtime::FlowLocalRuntimeSummary;

use super::{FlowIntegrationTarget, FlowProductionConfig};

pub const VALIDATED_RELEASE_COMMANDS: [&str; 8] = [
    "cargo check",
    "cargo test",
    "cargo build",
    "cargo check -p flow-browser-core",
    "cargo check --features example-binaries --examples",
    "npm run typecheck (extensions/flow-webext)",
    "npm run build:all (extensions/flow-webext)",
    "npm run package:all (extensions/flow-webext)",
];

pub const BROWSER_RELEASE_ARTIFACTS: [&str; 3] = [
    "extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip",
    "extensions/flow-webext/artifacts/flow-webext-firefox-v0.1.0.zip",
    "extensions/flow-webext/artifacts/flow-webext-safari-v0.1.0.zip",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowProductionBundleEntry {
    pub target: FlowIntegrationTarget,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowProductionBundleManifest {
    pub project: String,
    pub crate_version: String,
    pub generated_at_unix_ms: u128,
    pub device_tier: String,
    pub selected_text_model: Option<String>,
    pub selected_stt_model: Option<String>,
    pub selected_tts_model: Option<String>,
    pub all_models_ready: bool,
    pub missing_model_paths: Vec<String>,
    pub entries: Vec<FlowProductionBundleEntry>,
    pub browser_release_artifacts: Vec<String>,
    pub validated_commands: Vec<String>,
    pub notes: Vec<String>,
}

impl FlowProductionBundleManifest {
    pub fn for_summary(
        summary: &FlowLocalRuntimeSummary,
        entries: Vec<FlowProductionBundleEntry>,
    ) -> Self {
        Self {
            project: "flow".to_string(),
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            device_tier: format!("{:?}", summary.device_profile.tier),
            selected_text_model: summary.chat.model_key.clone(),
            selected_stt_model: summary.speech_to_text.model_key.clone(),
            selected_tts_model: summary.text_to_speech.model_key.clone(),
            all_models_ready: summary.all_ready(),
            missing_model_paths: summary.missing_model_paths(),
            entries,
            browser_release_artifacts: BROWSER_RELEASE_ARTIFACTS
                .into_iter()
                .map(str::to_string)
                .collect(),
            validated_commands: VALIDATED_RELEASE_COMMANDS
                .into_iter()
                .map(str::to_string)
                .collect(),
            notes: vec![
                "This bundle contains low-end-safe production defaults for every supported Flow host target.".to_string(),
                "Firebase wiring, browser-store publishing, and vendor-side signing stay external to this repository.".to_string(),
            ],
        }
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

pub fn recommended_production_configs(
    summary: &FlowLocalRuntimeSummary,
) -> Vec<FlowProductionConfig> {
    FlowIntegrationTarget::all()
        .iter()
        .copied()
        .map(|target| FlowProductionConfig::recommended_for_target(target, summary))
        .collect()
}

pub fn export_production_bundle(
    summary: &FlowLocalRuntimeSummary,
    output_dir: impl AsRef<Path>,
) -> Result<FlowProductionBundleManifest> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let mut entries = Vec::new();
    for target in FlowIntegrationTarget::all().iter().copied() {
        let config = FlowProductionConfig::recommended_for_target(target, summary);
        let filename = format!("{}.json", target.slug());
        let path = output_dir.join(&filename);
        fs::write(&path, config.to_pretty_json()?)?;
        entries.push(FlowProductionBundleEntry { target, filename });
    }

    let manifest = FlowProductionBundleManifest::for_summary(summary, entries);
    fs::write(output_dir.join("manifest.json"), manifest.to_pretty_json()?)?;
    fs::write(
        output_dir.join("README.txt"),
        build_bundle_readme(summary, &manifest),
    )?;

    Ok(manifest)
}

fn build_bundle_readme(
    summary: &FlowLocalRuntimeSummary,
    manifest: &FlowProductionBundleManifest,
) -> String {
    let mut lines = vec![
        "Flow Production Bundle".to_string(),
        "======================".to_string(),
        String::new(),
        format!("crate_version={}", manifest.crate_version),
        format!("device_tier={}", manifest.device_tier),
        format!(
            "text_model={}",
            summary.chat.model_key.as_deref().unwrap_or("none")
        ),
        format!(
            "stt_model={}",
            summary
                .speech_to_text
                .model_key
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "tts_model={}",
            summary
                .text_to_speech
                .model_key
                .as_deref()
                .unwrap_or("none")
        ),
        format!("all_models_ready={}", manifest.all_models_ready),
        String::new(),
        "Included configs:".to_string(),
    ];

    for entry in &manifest.entries {
        lines.push(format!("  - {} -> {}", entry.target.slug(), entry.filename));
    }

    if !manifest.missing_model_paths.is_empty() {
        lines.push(String::new());
        lines.push("Missing local model paths:".to_string());
        for path in &manifest.missing_model_paths {
            lines.push(format!("  - {}", path));
        }
    }

    lines.push(String::new());
    lines.push("Validated commands:".to_string());
    for command in &manifest.validated_commands {
        lines.push(format!("  - {}", command));
    }

    lines.push(String::new());
    lines.push("Browser release artifacts:".to_string());
    for artifact in &manifest.browser_release_artifacts {
        lines.push(format!("  - {}", artifact));
    }

    lines.join("\n")
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
    fn production_bundle_covers_all_targets() {
        let configs = recommended_production_configs(&low_end_runtime_summary());
        assert_eq!(configs.len(), FlowIntegrationTarget::all().len());
        assert!(
            configs
                .iter()
                .any(|config| config.target == FlowIntegrationTarget::BrowserExtension)
        );
    }

    #[test]
    fn export_production_bundle_writes_manifest_and_configs() {
        let temp_dir = std::env::temp_dir().join(format!(
            "flow_bundle_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let manifest = export_production_bundle(&low_end_runtime_summary(), &temp_dir).unwrap();

        assert!(temp_dir.join("manifest.json").exists());
        assert!(temp_dir.join("README.txt").exists());
        for entry in &manifest.entries {
            assert!(temp_dir.join(&entry.filename).exists());
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
