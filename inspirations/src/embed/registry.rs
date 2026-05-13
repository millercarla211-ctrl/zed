use std::path::{Path, PathBuf};

use crate::provider_catalog::ProviderCatalogBridge;
use crate::runtime::RuntimeBroker;
use crate::workspace::dx_project_statuses;

use super::types::{
    AdjacentProject, AdjacentProjectKind, FlowLibraryBlueprint, FlowSubsystem, ForgeStrategy,
    HostSurface, IntegrationMode, LongContextStrategy, ProviderAuthKind, ProviderStrategy,
    SearchStrategy, SerializerStrategy,
};

pub struct FlowEmbeddingRegistry {
    root: PathBuf,
    broker: RuntimeBroker,
}

impl FlowEmbeddingRegistry {
    pub fn detect() -> Self {
        Self::from_root(PathBuf::from("."))
    }

    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            broker: RuntimeBroker::detect(),
        }
    }

    pub fn broker(&self) -> &RuntimeBroker {
        &self.broker
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn adjacent_projects(&self) -> Vec<AdjacentProject> {
        let definitions = [
            (
                "metasearch",
                AdjacentProjectKind::Metasearch,
                "Metasearch engine for agents and retrieval-heavy flows.",
            ),
            (
                "serializer",
                AdjacentProjectKind::Serializer,
                "DX serializer (TOON-based) for token-efficient prompt packing.",
            ),
            (
                "rlm",
                AdjacentProjectKind::Rlm,
                "Long-context recursive model orchestration for large prompts and files.",
            ),
            (
                "providers",
                AdjacentProjectKind::Providers,
                "Remote provider auth, model access, and account orchestration.",
            ),
            (
                "forge",
                AdjacentProjectKind::Forge,
                "Multi-remote version control for code, media, and other large assets.",
            ),
        ];

        definitions
            .into_iter()
            .map(|(key, kind, purpose)| {
                let root_dir = self.root.join(key);
                AdjacentProject {
                    key: key.to_string(),
                    kind,
                    root_dir: root_dir.to_string_lossy().into_owned(),
                    detected: root_dir.exists(),
                    purpose: purpose.to_string(),
                }
            })
            .collect()
    }

    pub fn blueprint(&self, host: HostSurface) -> FlowLibraryBlueprint {
        let adjacent_projects = self.adjacent_projects();
        let provider_present = adjacent_projects
            .iter()
            .any(|item| item.kind == AdjacentProjectKind::Providers && item.detected);
        let serializer_present = adjacent_projects
            .iter()
            .any(|item| item.kind == AdjacentProjectKind::Serializer && item.detected);
        let metasearch_present = adjacent_projects
            .iter()
            .any(|item| item.kind == AdjacentProjectKind::Metasearch && item.detected);
        let rlm_present = adjacent_projects
            .iter()
            .any(|item| item.kind == AdjacentProjectKind::Rlm && item.detected);
        let forge_present = adjacent_projects
            .iter()
            .any(|item| item.kind == AdjacentProjectKind::Forge && item.detected);

        let integration_mode = integration_mode_for(host);
        let core_subsystems = core_subsystems_for(host);
        let optional_subsystems = optional_subsystems_for(host);

        let provider_strategy = ProviderStrategy {
            folder_present: provider_present,
            auth_kinds: vec![
                ProviderAuthKind::ApiKey,
                ProviderAuthKind::OAuth,
                ProviderAuthKind::BrowserCookie,
                ProviderAuthKind::LiteLlm,
                ProviderAuthKind::ModelsDevCatalog,
                ProviderAuthKind::UserPremium,
                ProviderAuthKind::FreeOfferPool,
                ProviderAuthKind::LocalFallback,
            ],
            auto_switch_local_and_remote: true,
            notes: vec![
                "Use local models first when latency, privacy, and cost win.".to_string(),
                "Aggregate scattered free offers and user premium accounts behind one policy layer."
                    .to_string(),
                "Use models.dev for catalog metadata and LiteLLM-style remote normalization where useful."
                    .to_string(),
            ],
        };

        let serializer_strategy = SerializerStrategy {
            folder_present: serializer_present,
            format_name: "dx-serializer".to_string(),
            uses_rkyv: true,
            uses_memmap: true,
            notes: vec![
                "Keep model weights in native formats and use the serializer only for prompts, tools, and structured context."
                    .to_string(),
                "Archive prompt indexes with rkyv and mmap them for low-latency reuse.".to_string(),
                "Treat serializer output as a token-saving transport layer for long prompts and editor integrations."
                    .to_string(),
            ],
        };

        let search_strategy = SearchStrategy {
            folder_present: metasearch_present,
            notes: vec![
                "Expose metasearch as an optional retrieval subsystem for agents and remote tools."
                    .to_string(),
                "Keep it decoupled so editor forks can embed only the search slice they need."
                    .to_string(),
            ],
        };

        let long_context_strategy = LongContextStrategy {
            folder_present: rlm_present,
            notes: vec![
                "Use RLM for long files, agent context packing, and recursive execution over oversized inputs."
                    .to_string(),
                "Pair RLM with serializer-based compression and prompt caches for editor and codebase workflows."
                    .to_string(),
            ],
        };

        let forge_strategy = ForgeStrategy {
            folder_present: forge_present,
            supports_multi_remote: true,
            notes: vec![
                "Use Forge as the media and multi-remote version-control layer for code, audio, video, 3D assets, and large binaries."
                    .to_string(),
                "Treat GitHub, GitLab, Bitbucket, YouTube, Sketchfab, SoundCloud, object storage, and other remotes as pluggable sync targets."
                    .to_string(),
            ],
        };

        FlowLibraryBlueprint {
            host,
            integration_mode,
            device_profile: self.broker.device_profile().clone(),
            workspace_projects: dx_project_statuses(),
            core_subsystems,
            optional_subsystems,
            adjacent_projects,
            provider_strategy,
            provider_catalog_plan: ProviderCatalogBridge::default_plan(),
            serializer_strategy,
            search_strategy,
            long_context_strategy,
            forge_strategy,
            notes: host_notes(host),
        }
    }
}

fn integration_mode_for(host: HostSurface) -> IntegrationMode {
    match host {
        HostSurface::Dx | HostSurface::FlowApp => IntegrationMode::FullRuntime,
        HostSurface::CodexFork | HostSurface::ZedFork | HostSurface::ZeroclawFork => {
            IntegrationMode::FeatureSlice
        }
        HostSurface::BrowserWasm => IntegrationMode::WasmBridge,
        HostSurface::Tauri => IntegrationMode::RustCrate,
        HostSurface::Flutter | HostSurface::AndroidNative | HostSurface::IosNative => {
            IntegrationMode::FfiBridge
        }
        HostSurface::Desktop
        | HostSurface::Vps
        | HostSurface::RaspberryPi
        | HostSurface::Watch
        | HostSurface::Tv
        | HostSurface::Tablet
        | HostSurface::CustomRustHost => IntegrationMode::RustCrate,
    }
}

fn core_subsystems_for(host: HostSurface) -> Vec<FlowSubsystem> {
    match host {
        HostSurface::Dx => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::DeviceProfiling,
            FlowSubsystem::LocalInference,
            FlowSubsystem::WakeWords,
            FlowSubsystem::VoiceDictation,
            FlowSubsystem::Grammar,
            FlowSubsystem::ProviderRouting,
            FlowSubsystem::Serializer,
            FlowSubsystem::LongContext,
            FlowSubsystem::ForgeSync,
        ],
        HostSurface::BrowserWasm => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::BrowserRuntime,
            FlowSubsystem::Serializer,
            FlowSubsystem::PromptCache,
            FlowSubsystem::ProviderRouting,
        ],
        HostSurface::FlowApp => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::DeviceProfiling,
            FlowSubsystem::LocalInference,
            FlowSubsystem::WakeWords,
            FlowSubsystem::VoiceDictation,
            FlowSubsystem::Grammar,
            FlowSubsystem::ProviderRouting,
            FlowSubsystem::Serializer,
        ],
        HostSurface::CodexFork | HostSurface::ZedFork | HostSurface::ZeroclawFork => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::LocalInference,
            FlowSubsystem::ProviderRouting,
            FlowSubsystem::Serializer,
            FlowSubsystem::LongContext,
        ],
        HostSurface::AndroidNative
        | HostSurface::IosNative
        | HostSurface::Flutter
        | HostSurface::Tauri => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::DeviceProfiling,
            FlowSubsystem::LocalInference,
            FlowSubsystem::VoiceDictation,
            FlowSubsystem::WakeWords,
            FlowSubsystem::Grammar,
        ],
        _ => vec![
            FlowSubsystem::RuntimeBroker,
            FlowSubsystem::DeviceProfiling,
            FlowSubsystem::LocalInference,
            FlowSubsystem::WakeWords,
            FlowSubsystem::VoiceDictation,
            FlowSubsystem::Grammar,
            FlowSubsystem::ProviderRouting,
            FlowSubsystem::Serializer,
        ],
    }
}

fn optional_subsystems_for(host: HostSurface) -> Vec<FlowSubsystem> {
    match host {
        HostSurface::BrowserWasm => vec![
            FlowSubsystem::Metasearch,
            FlowSubsystem::LongContext,
            FlowSubsystem::CommunityIndex,
        ],
        HostSurface::Watch | HostSurface::Tv => vec![
            FlowSubsystem::ProviderRouting,
            FlowSubsystem::Serializer,
            FlowSubsystem::PromptCache,
        ],
        _ => vec![
            FlowSubsystem::RemoteAuth,
            FlowSubsystem::Metasearch,
            FlowSubsystem::LongContext,
            FlowSubsystem::PromptCache,
            FlowSubsystem::DeviceControl,
            FlowSubsystem::Conversion,
            FlowSubsystem::CommunityIndex,
            FlowSubsystem::ForgeSync,
        ],
    }
}

fn host_notes(host: HostSurface) -> Vec<String> {
    match host {
        HostSurface::Dx => vec![
            "DX should consume Flow as the local/remote AI runtime layer, not reimplement it."
                .to_string(),
            "Use Flow as the place where model routing, voice, grammar, wake words, and provider policy live."
                .to_string(),
            "DX should also treat Forge, metasearch, serializer, RLM, and providers as adjacent subsystems in one coordinated stack."
                .to_string(),
        ],
        HostSurface::CodexFork | HostSurface::ZedFork | HostSurface::ZeroclawFork => vec![
            "Editor forks should embed only missing feature slices and keep their existing provider/editor plumbing where it is already stable."
                .to_string(),
            "Prefer Rust crate integration over duplicate binaries where the host is already Rust-native."
                .to_string(),
        ],
        HostSurface::BrowserWasm => vec![
            "Browser support should prioritize ONNX/WebGPU/WebAssembly-friendly paths rather than assuming native desktop runtimes."
                .to_string(),
            "Keep the browser surface stateless enough to swap between local in-browser inference and remote provider fallback."
                .to_string(),
        ],
        HostSurface::AndroidNative | HostSurface::IosNative | HostSurface::Flutter | HostSurface::Tauri => vec![
            "Mobile and shell apps should consume a smaller Rust core through FFI or direct Rust linkage."
                .to_string(),
            "Treat voice capture, wake words, grammar, and prompt compression as embeddable slices."
                .to_string(),
        ],
        HostSurface::Vps | HostSurface::RaspberryPi => vec![
            "Server and edge targets should keep local inference optional and favor feature-gated runtimes."
                .to_string(),
            "Low-memory and long-running behavior matter more than maximum throughput on these targets."
                .to_string(),
        ],
        HostSurface::Watch | HostSurface::Tv | HostSurface::Tablet => vec![
            "Thin-device targets should use wake words, dictation, provider routing, and lightweight local models first."
                .to_string(),
        ],
        HostSurface::FlowApp | HostSurface::Desktop | HostSurface::CustomRustHost => vec![
            "The standalone Flow app should remain the reference implementation of the full runtime."
                .to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_adjacent_project_slots() {
        let registry = FlowEmbeddingRegistry::from_root(".");
        let projects = registry.adjacent_projects();
        assert!(projects.iter().any(|item| item.key == "providers"));
        assert!(projects.iter().any(|item| item.key == "serializer"));
    }

    #[test]
    fn browser_blueprint_prefers_wasm_bridge() {
        let registry = FlowEmbeddingRegistry::from_root(".");
        let blueprint = registry.blueprint(HostSurface::BrowserWasm);
        assert_eq!(blueprint.integration_mode, IntegrationMode::WasmBridge);
        assert!(
            blueprint
                .core_subsystems
                .contains(&FlowSubsystem::BrowserRuntime)
        );
    }
}
