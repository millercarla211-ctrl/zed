use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::browser::{
    BrowserCapabilityProfile, BrowserExecutionPlan, BrowserExecutionRequest, BrowserHostFlavor,
    BrowserInferenceInvocation, BrowserInferenceRequest, BrowserPackResolution, BrowserTask,
    BrowserTokenStreamPlan, FlowBrowserEngine,
};
use crate::codex::{CodexFlowAdapter, CodexLocalModelStatus};
use crate::competitive::{CompetitiveScorecard, default_competitive_scorecard};
use crate::config::{
    FlowIntegrationTarget, FlowProductionBundleManifest, FlowProductionConfig, FlowReleaseSummary,
    export_production_bundle, export_release_summary, recommended_production_configs,
};
use crate::embed::{FlowEmbeddingRegistry, FlowLibraryBlueprint, HostSurface};
use crate::experience::{
    DictationAssistRequest, DictationAssistResult, FlowDictationEngine, FlowExperienceHub,
    FlowTypingAssistant, TextCommandRequest, TextCommandResult, TypingAssistRequest,
    TypingAssistResult,
};
use crate::forge_bridge::{ForgeBridge, ForgeSyncPlan};
use crate::long_context::{LongContextExecutionPlan, LongContextTask, RlmBridge};
use crate::prompt::{DxSerializer, SerializedPromptEnvelope};
use crate::provider_catalog::{ProviderCatalogBridge, ProviderCatalogPlan};
use crate::remote::{RemoteModelEndpoint, RemoteProviderRouter, SeamlessRoutePlan};
use crate::runtime::{BrokerRequest, ExecutionPlan, Modality, RuntimeBroker};
use crate::runtime::{FlowLocalRuntime, FlowLocalRuntimeSummary};
use crate::search::{MetasearchBridge, SearchRequestPlan};
use crate::workspace::{DxProjectStatus, dx_project_statuses};
use crate::zed::{ZedFlowAdapter, ZedLocalModelStatus};
use crate::zeroclaw::{ZeroClawFlowAdapter, ZeroClawLocalModelStatus};

pub struct DxFlowRuntime {
    embedding_registry: FlowEmbeddingRegistry,
    browser: FlowBrowserEngine,
    typing: FlowTypingAssistant,
    dictation: FlowDictationEngine,
}

impl DxFlowRuntime {
    pub fn detect() -> Self {
        Self {
            embedding_registry: FlowEmbeddingRegistry::detect(),
            browser: FlowBrowserEngine::detect(),
            typing: FlowTypingAssistant::new(),
            dictation: FlowDictationEngine::new(),
        }
    }

    pub fn embedding_registry(&self) -> &FlowEmbeddingRegistry {
        &self.embedding_registry
    }

    pub fn broker(&self) -> &RuntimeBroker {
        self.embedding_registry.broker()
    }

    pub fn browser(&self) -> &FlowBrowserEngine {
        &self.browser
    }

    pub fn embedding_blueprint(&self, host: HostSurface) -> FlowLibraryBlueprint {
        self.embedding_registry.blueprint(host)
    }

    pub fn create_experience_hub(&self, name: impl Into<String>) -> FlowExperienceHub {
        FlowExperienceHub::new(name)
    }

    pub fn project_stack(&self) -> Vec<DxProjectStatus> {
        dx_project_statuses()
    }

    pub fn competitive_scorecard(&self) -> CompetitiveScorecard {
        default_competitive_scorecard()
    }

    pub fn runtime_plan(&self, modality: Modality, model: Option<String>) -> ExecutionPlan {
        self.broker()
            .build_plan(BrokerRequest::new(modality).with_model(model))
    }

    pub fn create_local_runtime(&self) -> Result<FlowLocalRuntime> {
        FlowLocalRuntime::for_device_profile(self.broker().device_profile().clone())
    }

    pub fn local_runtime_summary(&self) -> Result<FlowLocalRuntimeSummary> {
        Ok(self.create_local_runtime()?.summary().clone())
    }

    pub fn create_zed_adapter(&self) -> Result<ZedFlowAdapter> {
        Ok(ZedFlowAdapter::from_runtime(self.create_local_runtime()?))
    }

    pub fn zed_local_status(&self) -> Result<ZedLocalModelStatus> {
        Ok(self.create_zed_adapter()?.local_model_status())
    }

    pub fn create_codex_adapter(&self) -> Result<CodexFlowAdapter> {
        Ok(CodexFlowAdapter::from_runtime(self.create_local_runtime()?))
    }

    pub fn codex_local_status(&self) -> Result<CodexLocalModelStatus> {
        Ok(self.create_codex_adapter()?.local_model_status())
    }

    pub fn create_zeroclaw_adapter(&self) -> Result<ZeroClawFlowAdapter> {
        Ok(ZeroClawFlowAdapter::from_runtime(
            self.create_local_runtime()?,
        ))
    }

    pub fn zeroclaw_local_status(&self) -> Result<ZeroClawLocalModelStatus> {
        Ok(self.create_zeroclaw_adapter()?.local_model_status())
    }

    pub fn production_config(&self, target: FlowIntegrationTarget) -> Result<FlowProductionConfig> {
        Ok(FlowProductionConfig::recommended_for_target(
            target,
            &self.local_runtime_summary()?,
        ))
    }

    pub fn production_config_json(&self, target: FlowIntegrationTarget) -> Result<String> {
        self.production_config(target)?.to_pretty_json()
    }

    pub fn all_production_configs(&self) -> Result<Vec<FlowProductionConfig>> {
        Ok(recommended_production_configs(
            &self.local_runtime_summary()?,
        ))
    }

    pub fn production_bundle_manifest(&self) -> Result<FlowProductionBundleManifest> {
        let summary = self.local_runtime_summary()?;
        let entries = FlowIntegrationTarget::all()
            .iter()
            .copied()
            .map(|target| crate::config::FlowProductionBundleEntry {
                target,
                filename: format!("{}.json", target.slug()),
            })
            .collect();
        Ok(FlowProductionBundleManifest::for_summary(&summary, entries))
    }

    pub fn export_production_bundle(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<FlowProductionBundleManifest> {
        export_production_bundle(&self.local_runtime_summary()?, output_dir)
    }

    pub fn release_summary(&self) -> Result<FlowReleaseSummary> {
        FlowReleaseSummary::for_repo(&self.local_runtime_summary()?, env!("CARGO_MANIFEST_DIR"))
    }

    pub fn export_release_summary(
        &self,
        output_dir: impl AsRef<Path>,
    ) -> Result<FlowReleaseSummary> {
        export_release_summary(
            &self.local_runtime_summary()?,
            env!("CARGO_MANIFEST_DIR"),
            output_dir,
        )
    }

    pub fn detect_browser_capabilities(
        &self,
        flavor: BrowserHostFlavor,
        webgpu: Option<bool>,
        wasm_threads: Option<bool>,
        cross_origin_isolated: Option<bool>,
        opfs: Option<bool>,
        indexeddb: Option<bool>,
    ) -> BrowserCapabilityProfile {
        self.browser.detect_browser_capabilities(
            flavor,
            webgpu,
            wasm_threads,
            cross_origin_isolated,
            opfs,
            indexeddb,
        )
    }

    pub fn browser_plan(
        &self,
        task: BrowserTask,
        modality: Modality,
        local_only: bool,
        preferred_model: Option<String>,
        allow_remote_fallback: bool,
        capabilities: BrowserCapabilityProfile,
    ) -> BrowserExecutionPlan {
        self.browser
            .plan_browser_execution(BrowserExecutionRequest {
                task,
                modality,
                local_only,
                preferred_model,
                allow_remote_fallback,
                capabilities,
            })
    }

    pub fn ensure_browser_pack(&self, model_key: &str) -> BrowserPackResolution {
        self.browser.ensure_browser_model_pack(model_key)
    }

    pub fn run_browser_inference(
        &self,
        request: BrowserInferenceRequest,
    ) -> BrowserInferenceInvocation {
        self.browser.run_browser_inference(request)
    }

    pub fn browser_token_stream_plan(
        &self,
        request: &BrowserInferenceRequest,
    ) -> BrowserTokenStreamPlan {
        self.browser.stream_browser_tokens(request)
    }

    pub fn agent_search_plan(&self, query: impl Into<String>) -> SearchRequestPlan {
        MetasearchBridge::for_agent_grounding(query)
    }

    pub fn model_search_plan(&self, query: impl Into<String>) -> SearchRequestPlan {
        MetasearchBridge::for_model_discovery(query)
    }

    pub fn forge_sync_plan(&self) -> ForgeSyncPlan {
        ForgeBridge::for_dx_media_pipeline()
    }

    pub fn provider_catalog_plan(&self) -> ProviderCatalogPlan {
        ProviderCatalogBridge::default_plan()
    }

    pub fn long_context_plan(&self, task: LongContextTask) -> LongContextExecutionPlan {
        match task {
            LongContextTask::SummarizeLargeDocument => RlmBridge::for_large_document_summary(),
            LongContextTask::AnalyzeCodebase
            | LongContextTask::BuildAgentContext
            | LongContextTask::RecursiveQuestionAnswering
            | LongContextTask::MultiFileReasoning => RlmBridge::for_codebase_analysis(),
        }
    }

    pub fn remote_route_plan(
        &self,
        modality: Modality,
        local_model_key: Option<String>,
        remote_candidates: Vec<RemoteModelEndpoint>,
    ) -> SeamlessRoutePlan {
        RemoteProviderRouter::plan(modality, local_model_key, remote_candidates)
    }

    pub fn serialize_prompt_json(
        &self,
        kind: &str,
        value: &Value,
    ) -> Result<SerializedPromptEnvelope> {
        DxSerializer::encode_json(kind, value)
    }

    pub fn process_typing(&self, request: TypingAssistRequest) -> Result<TypingAssistResult> {
        self.typing.process(request)
    }

    pub fn process_dictation(
        &self,
        request: DictationAssistRequest,
    ) -> Result<DictationAssistResult> {
        self.dictation.process(request)
    }

    pub fn execute_text_command(&self, request: TextCommandRequest) -> Result<TextCommandResult> {
        self.typing.execute_command(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::ProviderAuthKind;
    use crate::remote::{AccessTier, RemoteCapability};

    #[test]
    fn facade_exposes_dx_project_stack() {
        let runtime = DxFlowRuntime::detect();
        assert!(
            runtime
                .project_stack()
                .iter()
                .any(|project| project.key == "forge")
        );
    }

    #[test]
    fn facade_routes_remote_candidates() {
        let runtime = DxFlowRuntime::detect();
        let route = runtime.remote_route_plan(
            Modality::Chat,
            Some("qwen3-0.6b".to_string()),
            vec![RemoteModelEndpoint {
                provider_id: "free".to_string(),
                model_id: "free-model".to_string(),
                label: "Free".to_string(),
                access_tier: AccessTier::FreeRemote,
                auth_kind: ProviderAuthKind::OAuth,
                capabilities: vec![RemoteCapability::Chat],
            }],
        );
        assert_eq!(route.remote_candidates.len(), 1);
    }

    #[test]
    fn facade_can_build_local_runtime_summary() {
        let runtime = DxFlowRuntime::detect();
        let summary = runtime.local_runtime_summary().unwrap();
        assert!(summary.chat.model_key.is_some());
    }

    #[test]
    fn facade_can_build_zed_status() {
        let runtime = DxFlowRuntime::detect();
        let status = runtime.zed_local_status().unwrap();
        assert!(status.summary.chat.model_key.is_some());
    }

    #[test]
    fn facade_can_build_codex_status() {
        let runtime = DxFlowRuntime::detect();
        let status = runtime.codex_local_status().unwrap();
        assert!(status.summary.chat.model_key.is_some());
    }

    #[test]
    fn facade_can_build_zeroclaw_status() {
        let runtime = DxFlowRuntime::detect();
        let status = runtime.zeroclaw_local_status().unwrap();
        assert!(status.summary.chat.model_key.is_some());
    }

    #[test]
    fn facade_can_build_production_config() {
        let runtime = DxFlowRuntime::detect();
        let config = runtime
            .production_config(FlowIntegrationTarget::CodexFork)
            .unwrap();
        assert!(config.selected_text_model.is_some());
    }

    #[test]
    fn facade_can_list_all_production_configs() {
        let runtime = DxFlowRuntime::detect();
        let configs = runtime.all_production_configs().unwrap();
        assert_eq!(configs.len(), FlowIntegrationTarget::all().len());
    }

    #[test]
    fn facade_can_build_release_summary() {
        let runtime = DxFlowRuntime::detect();
        let summary = runtime.release_summary().unwrap();
        assert_eq!(summary.project, "flow");
    }
}
