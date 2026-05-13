use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::storage::FlowPackStore;
use crate::utils::detect_device_profile;

use super::catalog::{default_activation_config, default_model_catalog};
use super::types::{
    ActivationConfig, ArtifactBundle, ArtifactFile, BenchmarkRecord, BrokerRequest, ConversionJob,
    DeviceProfile, DeviceTier, ExecutionPlan, Modality, ModelManifest, PublishRecord,
    PublishStatus, RuntimeKind, RuntimeLaunch,
};

pub struct RuntimeBroker {
    device_profile: DeviceProfile,
    catalog: Vec<ModelManifest>,
    activation: ActivationConfig,
}

impl RuntimeBroker {
    pub fn detect() -> Self {
        Self {
            device_profile: detect_device_profile(),
            catalog: default_model_catalog(),
            activation: default_activation_config(),
        }
    }

    pub fn from_parts(
        device_profile: DeviceProfile,
        catalog: Vec<ModelManifest>,
        activation: ActivationConfig,
    ) -> Self {
        Self {
            device_profile,
            catalog,
            activation,
        }
    }

    pub fn device_profile(&self) -> &DeviceProfile {
        &self.device_profile
    }

    pub fn activation(&self) -> &ActivationConfig {
        &self.activation
    }

    pub fn catalog(&self) -> &[ModelManifest] {
        &self.catalog
    }

    pub fn models_for(&self, modality: Modality) -> Vec<&ModelManifest> {
        let mut models = self
            .catalog
            .iter()
            .filter(|manifest| manifest.modality == modality)
            .collect::<Vec<_>>();
        models.sort_by_key(|manifest| {
            (
                !self.model_is_local(manifest),
                manifest.minimum_memory_bytes,
                manifest.key.clone(),
            )
        });
        models
    }

    pub fn build_plan(&self, request: BrokerRequest) -> ExecutionPlan {
        let modality = request.modality;
        let requested_model = request.preferred_model.clone();

        let candidates = self.models_for(modality);
        let selected = requested_model
            .as_ref()
            .and_then(|key| {
                candidates
                    .iter()
                    .copied()
                    .find(|candidate| &candidate.key == key)
            })
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|candidate| self.model_fits_device(candidate))
            })
            .or_else(|| candidates.first().copied());

        let mut reasons = vec![format!(
            "Device tier {} prefers the fastest local runtime for {:?}.",
            self.tier_label(self.device_profile.tier),
            modality
        )];

        if let Some(selected) = selected {
            if self.model_is_local(selected) {
                reasons.push(format!(
                    "Selected '{}' because a local artifact already exists.",
                    selected.display_name
                ));

                let artifact = self.build_artifact_bundle(selected).ok();
                let publish_record = request.allow_publish.then(|| self.plan_publish(selected));
                let conversion_job = None;

                return ExecutionPlan {
                    modality,
                    requested_model,
                    selected_model: Some(selected.key.clone()),
                    selected_runtime: Some(selected.preferred_runtime),
                    launch: self.launch_kind(selected.preferred_runtime),
                    device_tier: self.device_profile.tier,
                    estimated_memory_bytes: Some(selected.minimum_memory_bytes),
                    reasons,
                    artifact,
                    conversion_job,
                    publish_record,
                    unsupported_reason: None,
                };
            }

            if request.allow_conversion && !selected.conversion_lanes.is_empty() {
                reasons.push(format!(
                    "No local artifact exists for '{}', so Flow will schedule a local conversion job.",
                    selected.display_name
                ));

                return ExecutionPlan {
                    modality,
                    requested_model,
                    selected_model: Some(selected.key.clone()),
                    selected_runtime: Some(RuntimeKind::ConversionOrchestrator),
                    launch: RuntimeLaunch::Conversion,
                    device_tier: self.device_profile.tier,
                    estimated_memory_bytes: Some(selected.minimum_memory_bytes),
                    reasons,
                    artifact: None,
                    conversion_job: Some(self.plan_conversion(selected)),
                    publish_record: request.allow_publish.then(|| self.plan_publish(selected)),
                    unsupported_reason: None,
                };
            }

            reasons.push(format!(
                "Flow knows about '{}', but there is no local artifact and conversion was disabled.",
                selected.display_name
            ));

            return ExecutionPlan {
                modality,
                requested_model,
                selected_model: Some(selected.key.clone()),
                selected_runtime: Some(RuntimeKind::Unsupported),
                launch: RuntimeLaunch::Unsupported,
                device_tier: self.device_profile.tier,
                estimated_memory_bytes: Some(selected.minimum_memory_bytes),
                reasons,
                artifact: None,
                conversion_job: None,
                publish_record: None,
                unsupported_reason: Some("No local artifact available.".to_string()),
            };
        }

        reasons.push("No catalog entry matches this modality yet.".to_string());
        ExecutionPlan {
            modality,
            requested_model,
            selected_model: None,
            selected_runtime: Some(RuntimeKind::Unsupported),
            launch: RuntimeLaunch::Unsupported,
            device_tier: self.device_profile.tier,
            estimated_memory_bytes: None,
            reasons,
            artifact: None,
            conversion_job: None,
            publish_record: None,
            unsupported_reason: Some("Unsupported modality.".to_string()),
        }
    }

    pub fn materialize_flowpack(
        &self,
        root: &Path,
        plan: &ExecutionPlan,
        benchmarks: &[BenchmarkRecord],
    ) -> Result<()> {
        if let Some(artifact) = &plan.artifact {
            FlowPackStore::write_flowpack(root, self.device_profile(), artifact, benchmarks)?;
        }
        Ok(())
    }

    fn model_is_local(&self, manifest: &ModelManifest) -> bool {
        if matches!(manifest.modality, Modality::SpeechToText) {
            return speech_to_text_artifact_ready(manifest);
        }

        manifest
            .local_path
            .as_ref()
            .map(Path::new)
            .map(Path::exists)
            .unwrap_or(false)
    }

    fn model_fits_device(&self, manifest: &ModelManifest) -> bool {
        if manifest.key == "nemotron-speech-streaming-en-0.6b-int8"
            && !self.device_is_nvidia_performance()
        {
            return false;
        }

        self.device_profile.available_memory_bytes >= manifest.minimum_memory_bytes
            || self.device_profile.total_memory_bytes >= manifest.minimum_memory_bytes
    }

    fn device_is_nvidia_performance(&self) -> bool {
        matches!(
            self.device_profile.tier,
            DeviceTier::Performance | DeviceTier::Workstation
        ) || self.device_profile.graphics.iter().any(|gpu| {
            gpu.vendor
                .as_deref()
                .is_some_and(|vendor| vendor.eq_ignore_ascii_case("nvidia"))
        })
    }

    fn plan_conversion(&self, manifest: &ModelManifest) -> ConversionJob {
        let lane = manifest.conversion_lanes[0];
        let command_preview = match lane {
            super::types::ConversionLane::Gguf => vec![
                "python".to_string(),
                "-m".to_string(),
                "transformers.models.gguf.convert".to_string(),
                manifest.repo_id.clone(),
            ],
            super::types::ConversionLane::Onnx => vec![
                "python".to_string(),
                "-m".to_string(),
                "transformers.onnx".to_string(),
                "--model".to_string(),
                manifest.repo_id.clone(),
            ],
            super::types::ConversionLane::NativeSafetensors => vec![
                "hf_transfer".to_string(),
                manifest.repo_id.clone(),
                "--native".to_string(),
            ],
        };

        ConversionJob {
            model_key: manifest.key.clone(),
            source_repo: manifest.repo_id.clone(),
            lane,
            target_format: manifest.artifact_format,
            command_preview,
            publish_after_validation: !manifest.local_only,
        }
    }

    fn plan_publish(&self, manifest: &ModelManifest) -> PublishRecord {
        let status = if manifest.local_only {
            PublishStatus::LocalOnly
        } else if manifest.gated || !manifest.redistributable {
            PublishStatus::Refused
        } else {
            PublishStatus::Planned
        };

        let reason = match status {
            PublishStatus::LocalOnly => Some("Artifact is marked local_only.".to_string()),
            PublishStatus::Refused => Some(
                "Artifact is gated or not redistributable, so auto-publish is disabled."
                    .to_string(),
            ),
            _ => None,
        };

        PublishRecord {
            model_key: manifest.key.clone(),
            destination_repo: format!("hf://user/{}", manifest.key),
            status,
            reason,
            checksum: None,
            verified: false,
            local_only: manifest.local_only,
        }
    }

    fn build_artifact_bundle(&self, manifest: &ModelManifest) -> Result<ArtifactBundle> {
        let local_path = manifest
            .local_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model does not have a local path"))?;

        let path = PathBuf::from(local_path);
        let files = if path.is_dir() {
            std::fs::read_dir(&path)?
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let metadata = entry.metadata().ok()?;
                    metadata.is_file().then_some((entry.path(), metadata.len()))
                })
                .map(|(entry_path, bytes)| ArtifactFile {
                    path: entry_path.to_string_lossy().into_owned(),
                    bytes: Some(bytes),
                    sha256: FlowPackStore::sha256_file(&entry_path).ok(),
                })
                .collect::<Vec<_>>()
        } else {
            vec![ArtifactFile {
                path: path.to_string_lossy().into_owned(),
                bytes: std::fs::metadata(&path).ok().map(|metadata| metadata.len()),
                sha256: FlowPackStore::sha256_file(&path).ok(),
            }]
        };

        Ok(ArtifactBundle {
            model_key: manifest.key.clone(),
            upstream_repo: manifest.repo_id.clone(),
            upstream_revision: Some("local-working-copy".to_string()),
            root_dir: path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_string_lossy()
                .into_owned(),
            artifact_format: manifest.artifact_format,
            quantization: manifest.quantization.clone(),
            license: manifest.license.clone(),
            runtime: manifest.preferred_runtime,
            files,
            redistributable: manifest.redistributable,
            gated: manifest.gated,
            local_only: manifest.local_only,
        })
    }

    fn launch_kind(&self, runtime: RuntimeKind) -> RuntimeLaunch {
        match runtime {
            RuntimeKind::WhisperCppSubprocess
            | RuntimeKind::StableDiffusionCppSubprocess
            | RuntimeKind::PythonWorker => RuntimeLaunch::Subprocess,
            RuntimeKind::ConversionOrchestrator => RuntimeLaunch::Conversion,
            RuntimeKind::Unsupported => RuntimeLaunch::Unsupported,
            _ => RuntimeLaunch::Embedded,
        }
    }

    fn tier_label(&self, tier: DeviceTier) -> &'static str {
        match tier {
            DeviceTier::Low => "low",
            DeviceTier::Balanced => "balanced",
            DeviceTier::Performance => "performance",
            DeviceTier::Workstation => "workstation",
        }
    }
}

fn speech_to_text_artifact_ready(manifest: &ModelManifest) -> bool {
    match manifest.key.as_str() {
        "moonshine-tiny" => {
            Path::new("models/stt/encoder_model.onnx").exists()
                && Path::new("models/stt/decoder_model_merged.onnx").exists()
                && Path::new("models/stt/tokenizer.json").exists()
        }
        "parakeet-tdt-0.6b-v3-int8" | "nemotron-speech-streaming-en-0.6b-int8" => manifest
            .local_path
            .as_deref()
            .and_then(|path| {
                let path = Path::new(path);
                if path.is_dir() {
                    Some(path)
                } else {
                    path.parent()
                }
            })
            .is_some_and(|root| {
                let encoder = root.join("encoder.int8.onnx");
                let decoder = root.join("decoder.int8.onnx");
                let joiner = root.join("joiner.int8.onnx");
                let tokens = root.join("tokens.txt");
                encoder.exists() && decoder.exists() && joiner.exists() && tokens.exists()
            }),
        _ => manifest
            .local_path
            .as_ref()
            .map(Path::new)
            .map(Path::exists)
            .unwrap_or(false),
    }
}

pub fn benchmark_record(
    model_key: &str,
    runtime: RuntimeKind,
    modality: Modality,
    load_time_ms: u64,
    tokens_per_second: Option<u64>,
    samples_per_second: Option<u64>,
    device_tier: DeviceTier,
) -> BenchmarkRecord {
    let measured_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();

    BenchmarkRecord {
        model_key: model_key.to_string(),
        runtime,
        modality,
        load_time_ms,
        tokens_per_second,
        samples_per_second,
        measured_at_unix_ms,
        device_tier,
    }
}
