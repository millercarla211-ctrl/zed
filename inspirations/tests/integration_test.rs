use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use flow::FlowLocalRuntime;
use flow::audio::{AudioLoader, MelSpectrogramConfig, compute_mel_spectrogram};
use flow::competitive::default_competitive_scorecard;
use flow::embed::{FlowEmbeddingRegistry, HostSurface, IntegrationMode};
use flow::experience::{
    AppContext, DictationAssistRequest, DictionaryEntry, FlowDictationEngine, FlowTypingAssistant,
    SnippetEntry, StylePreset, ToneStyle, TypingAssistRequest, WritingDomain,
};
use flow::forge_bridge::{ForgeBridge, ForgeRemoteKind};
use flow::long_context::RlmBridge;
use flow::prompt::DxSerializer;
use flow::provider_catalog::{CatalogSource, ProviderCatalogBridge};
use flow::remote::{AccessTier, RemoteCapability, RemoteModelEndpoint, RemoteProviderRouter};
use flow::runtime::{
    ArtifactBundle, ArtifactFile, ArtifactFormat, BrokerRequest, ComputeBackend, DeviceProfile,
    DeviceTier, GraphicsDevice, Modality, RuntimeBroker, RuntimeKind, RuntimeLaunch,
    benchmark_record,
};
use flow::storage::{FlowPackStore, PromptCacheEntry, PromptCacheIndex};
use flow::workspace::dx_project_statuses;
use flow::writing::HarperGrammarChecker;

fn make_device_profile(total_memory_bytes: u64, available_memory_bytes: u64) -> DeviceProfile {
    DeviceProfile {
        os: "windows".to_string(),
        arch: "x86_64".to_string(),
        cpu_model: "Test CPU".to_string(),
        physical_cores: 4,
        logical_cores: 8,
        total_memory_bytes,
        available_memory_bytes,
        battery_powered: None,
        thermal_class: None,
        graphics: vec![GraphicsDevice {
            name: "Integrated GPU".to_string(),
            vendor: Some("intel".to_string()),
            vram_bytes: None,
            integrated: true,
            backends: vec![ComputeBackend::Cpu, ComputeBackend::DirectMl],
        }],
        tier: if total_memory_bytes < 8 * 1024 * 1024 * 1024 {
            DeviceTier::Low
        } else {
            DeviceTier::Balanced
        },
    }
}

fn app_context(domain: WritingDomain) -> AppContext {
    AppContext {
        app_name: "Test App".to_string(),
        window_title: None,
        url: None,
        language: Some("en".to_string()),
        domain,
        workspace_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
        team_terms: vec!["Supabase".to_string()],
    }
}

fn temp_root(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("flow-{prefix}-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn broker_prefers_small_chat_model_on_low_tier_device() {
    let profile = make_device_profile(6 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024);
    let broker = RuntimeBroker::from_parts(
        profile,
        flow::runtime::default_model_catalog(),
        flow::runtime::default_activation_config(),
    );

    let mut request = BrokerRequest::new(Modality::Chat);
    request.allow_conversion = false;
    request.allow_publish = false;

    let plan = broker.build_plan(request);
    assert_eq!(plan.selected_model.as_deref(), Some("qwen3-0.6b"));
    assert_eq!(plan.launch, RuntimeLaunch::Embedded);
}

#[test]
fn embeddable_local_runtime_picks_qwen3_for_low_end_devices() {
    let runtime = FlowLocalRuntime::for_device_profile(make_device_profile(
        6 * 1024 * 1024 * 1024,
        4 * 1024 * 1024 * 1024,
    ))
    .unwrap();

    assert_eq!(runtime.default_text_model_key(), Some("qwen3-0.6b"));
    assert_eq!(
        runtime.summary().speech_to_text.model_key.as_deref(),
        Some("moonshine-tiny")
    );
    assert_eq!(
        runtime.summary().text_to_speech.model_key.as_deref(),
        Some("kokoro-int8")
    );
}

#[test]
fn flowpack_round_trip_and_prompt_cache_round_trip() {
    let root = temp_root("flowpack");
    let model_file = root.join("artifact.gguf");
    fs::write(&model_file, b"flow-artifact").unwrap();

    let artifact = ArtifactBundle {
        model_key: "qwen3-0.6b".to_string(),
        upstream_repo: "Qwen/Qwen3-0.6B".to_string(),
        upstream_revision: Some("main".to_string()),
        root_dir: root.to_string_lossy().into_owned(),
        artifact_format: ArtifactFormat::Gguf,
        quantization: Some("Q4_K_M".to_string()),
        license: Some("apache-2.0".to_string()),
        runtime: RuntimeKind::LlamaCppEmbedded,
        files: vec![ArtifactFile {
            path: model_file.to_string_lossy().into_owned(),
            bytes: Some(fs::metadata(&model_file).unwrap().len()),
            sha256: Some(FlowPackStore::sha256_file(&model_file).unwrap()),
        }],
        redistributable: true,
        gated: false,
        local_only: false,
    };

    let device = make_device_profile(8 * 1024 * 1024 * 1024, 6 * 1024 * 1024 * 1024);
    let benchmarks = vec![benchmark_record(
        "qwen3-0.6b",
        RuntimeKind::LlamaCppEmbedded,
        Modality::Chat,
        1200,
        Some(28),
        None,
        DeviceTier::Balanced,
    )];

    FlowPackStore::write_flowpack(&root, &device, &artifact, &benchmarks).unwrap();
    let manifest = FlowPackStore::read_flowpack(&root).unwrap();
    assert_eq!(manifest.artifact.model_key, "qwen3-0.6b");
    assert_eq!(manifest.benchmarks.len(), 1);

    let prompt_cache = PromptCacheIndex {
        entries: vec![PromptCacheEntry {
            key: FlowPackStore::prompt_cache_key("qwen3-0.6b", "tok", "sys", "tools"),
            prompt_hash: "hash".to_string(),
            token_count: 3,
            tokens: vec![1, 2, 3],
            updated_at_unix_ms: 1,
        }],
    };
    FlowPackStore::write_prompt_cache(&root, &prompt_cache).unwrap();
    let restored = FlowPackStore::read_prompt_cache(&root).unwrap();
    assert_eq!(restored.entries.len(), 1);
    assert_eq!(restored.entries[0].tokens, vec![1, 2, 3]);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn grammar_checker_reports_and_corrects_basic_text() {
    let checker = HarperGrammarChecker::new();
    let issues = checker.analyze("This is an test.").unwrap();
    assert!(!issues.is_empty());

    let corrected = checker.correct("This is an test.").unwrap();
    assert!(corrected.contains("a test"));
}

#[test]
fn wake_command_definitions_are_canonical() {
    let definitions = flow::runtime::wake_command_definitions();
    let commands = definitions
        .iter()
        .map(|definition| definition.command_key)
        .collect::<Vec<_>>();
    assert_eq!(commands, vec!["dx", "friday", "hello", "aladdin", "arise"]);
    assert!(
        definitions
            .iter()
            .all(|definition| definition.threshold == 68)
    );
    assert!(
        definitions
            .iter()
            .all(|definition| definition.aliases.is_empty())
    );
}

#[test]
fn mel_spectrogram_has_expected_shape() {
    let config = MelSpectrogramConfig::default();
    let samples = (0..3200)
        .map(|index| (index as f32 * 0.05).sin())
        .collect::<Vec<_>>();
    let mel = compute_mel_spectrogram(&samples, &config);

    assert_eq!(mel.nrows(), config.n_mels);
    assert!(mel.ncols() >= 1);
}

#[test]
fn audio_loader_reads_fixture_when_present() {
    let fixture = PathBuf::from("tests/fixtures/audio.mp3");
    if !fixture.exists() {
        return;
    }

    let samples = AudioLoader::load(&fixture).unwrap();
    assert!(!samples.is_empty());
}

#[test]
fn embedding_registry_builds_dx_blueprint() {
    let registry = FlowEmbeddingRegistry::from_root(".");
    let blueprint = registry.blueprint(HostSurface::Dx);

    assert_eq!(blueprint.integration_mode, IntegrationMode::FullRuntime);
    assert!(
        blueprint
            .adjacent_projects
            .iter()
            .any(|item| item.key == "providers")
    );
    assert!(
        blueprint
            .adjacent_projects
            .iter()
            .any(|item| item.key == "forge")
    );
    assert!(
        blueprint
            .provider_catalog_plan
            .sources
            .contains(&CatalogSource::ModelsDev)
    );
}

#[test]
fn dx_serializer_round_trips_json_payload() {
    let payload = serde_json::json!({
        "system": "You are dx",
        "tools": [{"name": "search"}, {"name": "provider-route"}]
    });

    let envelope = DxSerializer::encode_json("prompt", &payload).unwrap();
    let restored = DxSerializer::decode_json(&envelope).unwrap();
    assert_eq!(restored, payload);
}

#[test]
fn remote_router_prefers_free_remote_before_premium() {
    let premium = RemoteModelEndpoint {
        provider_id: "premium".to_string(),
        model_id: "premium-model".to_string(),
        label: "Premium".to_string(),
        access_tier: AccessTier::PremiumRemote,
        auth_kind: flow::embed::ProviderAuthKind::ApiKey,
        capabilities: vec![RemoteCapability::Chat],
    };
    let free = RemoteModelEndpoint {
        provider_id: "free".to_string(),
        model_id: "free-model".to_string(),
        label: "Free".to_string(),
        access_tier: AccessTier::FreeRemote,
        auth_kind: flow::embed::ProviderAuthKind::OAuth,
        capabilities: vec![RemoteCapability::Chat],
    };

    let plan = RemoteProviderRouter::plan(
        Modality::Chat,
        Some("qwen3-0.6b".to_string()),
        vec![premium, free],
    );
    assert_eq!(plan.remote_candidates[0].provider_id, "free");
}

#[test]
fn forge_bridge_covers_code_and_media_targets() {
    let plan = ForgeBridge::for_dx_media_pipeline();
    assert!(plan.remotes.contains(&ForgeRemoteKind::Github));
    assert!(plan.remotes.contains(&ForgeRemoteKind::Youtube));
}

#[test]
fn rlm_bridge_prefers_serializer_and_prompt_cache() {
    let plan = RlmBridge::for_codebase_analysis();
    assert!(plan.use_serializer);
    assert!(plan.use_prompt_cache);
}

#[test]
fn dx_workspace_registry_contains_forge() {
    let projects = dx_project_statuses();
    assert!(projects.iter().any(|project| project.key == "forge"));
}

#[test]
fn provider_catalog_plan_includes_models_dev_and_litellm() {
    let plan = ProviderCatalogBridge::default_plan();
    assert!(plan.sources.contains(&CatalogSource::ModelsDev));
    assert!(plan.sources.contains(&CatalogSource::LiteLlm));
}

#[test]
fn competitive_scorecard_has_expected_baseline() {
    let scorecard = default_competitive_scorecard();
    assert_eq!(scorecard.overall_score_out_of_100, 51);
    assert_eq!(scorecard.wispr_replacement_score_out_of_100, 52);
    assert_eq!(scorecard.grammarly_replacement_score_out_of_100, 40);
    assert_eq!(scorecard.flow_native_advantage_score_out_of_100, 57);
}

#[test]
fn typing_assistant_handles_snippets_dictionary_and_styles() {
    let assistant = FlowTypingAssistant::new();
    let result = assistant
        .process(TypingAssistRequest {
            text: "addr and supabase".to_string(),
            app_context: app_context(WritingDomain::Email),
            dictionary: vec![DictionaryEntry {
                surface: "supabase".to_string(),
                canonical: "Supabase".to_string(),
                case_sensitive: false,
                shared: true,
            }],
            snippets: vec![SnippetEntry {
                trigger: "addr".to_string(),
                expansion: "221B Baker Street".to_string(),
                shared: false,
                description: None,
            }],
            styles: vec![StylePreset {
                name: "email-professional".to_string(),
                domain: WritingDomain::Email,
                tone: ToneStyle::Professional,
                rules: Vec::new(),
            }],
            auto_correct: false,
            expand_snippets: true,
        })
        .unwrap();

    assert!(result.final_text.contains("221B Baker Street"));
    assert!(result.final_text.contains("Supabase"));
}

#[test]
fn dictation_engine_cleans_fillers_and_tags_files() {
    let engine = FlowDictationEngine::new();
    let result = engine
        .process(DictationAssistRequest {
            transcript: "um please update main.rs and actually readme.md".to_string(),
            app_context: app_context(WritingDomain::Code),
            dictionary: Vec::new(),
            snippets: Vec::new(),
            styles: Vec::new(),
            remove_fillers: true,
            auto_punctuate: false,
            format_lists: false,
            tag_workspace_files: true,
        })
        .unwrap();

    assert!(result.cleaned_text.contains("readme.md"));
    assert!(!result.cleaned_text.contains("um"));
    assert!(!result.file_tags.is_empty());
}
