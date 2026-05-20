mod agent_browser_payload_queue_inspect_tool;
mod agent_browser_payload_tool;
mod agent_chrome_payload_queue_inspect_tool;
mod agent_chrome_payload_tool;
mod agent_chrome_playwright_adapter_tool;
mod agent_chrome_playwright_execution_inspect_tool;
mod agent_chrome_playwright_invoke_tool;
mod agent_chrome_runner_gate_tool;
mod agent_pc_use_inspect_tool;
mod agent_pc_use_payload_queue_inspect_tool;
mod agent_pc_use_payload_tool;
mod agent_pc_use_runner_gate_tool;
mod agent_pc_use_runner_receipt_inspect_tool;
mod agent_pc_use_target_manifest_tool;
mod agent_pc_use_target_snapshot_tool;
mod agent_pc_use_ui_snapshot_contract_tool;
mod agent_pc_use_ui_snapshot_tool;
mod agent_plugin_asset_provisioner_tool;
mod agent_plugin_bootstrap_tool;
mod agent_plugin_catalog_tool;
mod agent_plugin_contracts;
mod agent_plugin_runtime_status_tool;
mod apply_code_action_tool;
mod context_server_registry;
mod copy_path_tool;
mod create_directory_tool;
mod delete_path_tool;
mod diagnostics_tool;
mod dx_catalog_provider_settings_registration_tool;
mod dx_catalog_provider_settings_tool;
mod dx_metasearch_status_tool;
mod dx_metasearch_tool;
mod edit_file_tool;
mod edit_session;
#[cfg(all(test, feature = "unit-eval"))]
mod evals;
mod fetch_tool;
mod find_path_tool;
mod find_references_tool;
mod get_code_actions_tool;
mod go_to_definition_tool;
mod grep_tool;
mod list_directory_tool;
mod move_path_tool;
mod read_file_tool;
mod rename_tool;
mod skill_tool;
mod spawn_agent_tool;
mod symbol_locator;
mod terminal_tool;
mod tool_permissions;
mod update_plan_tool;
mod web_search_tool;
mod write_file_tool;

use crate::AgentTool;
use language_model::{LanguageModelRequestTool, LanguageModelToolSchemaFormat};
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, Error as _},
};

/// Deserialize a value that may have been provided as a JSON-encoded string
/// instead of the structured value. Some models occasionally stringify nested
/// arguments, so we accept either form.
pub(crate) fn deserialize_maybe_stringified<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ValueOrJsonString<T> {
        Value(T),
        String(String),
    }

    match ValueOrJsonString::<T>::deserialize(deserializer)? {
        ValueOrJsonString::Value(value) => Ok(value),
        ValueOrJsonString::String(string) => serde_json::from_str::<T>(&string).map_err(|error| {
            D::Error::custom(format!("failed to parse stringified value: {error}"))
        }),
    }
}

pub use agent_browser_payload_queue_inspect_tool::*;
pub use agent_browser_payload_tool::*;
pub use agent_chrome_payload_queue_inspect_tool::*;
pub use agent_chrome_payload_tool::*;
pub use agent_chrome_playwright_adapter_tool::*;
pub use agent_chrome_playwright_execution_inspect_tool::*;
pub use agent_chrome_playwright_invoke_tool::*;
pub use agent_chrome_runner_gate_tool::*;
pub use agent_pc_use_inspect_tool::*;
pub use agent_pc_use_payload_queue_inspect_tool::*;
pub use agent_pc_use_payload_tool::*;
pub use agent_pc_use_runner_gate_tool::*;
pub use agent_pc_use_runner_receipt_inspect_tool::*;
pub use agent_pc_use_target_manifest_tool::*;
pub use agent_pc_use_target_snapshot_tool::*;
pub use agent_pc_use_ui_snapshot_contract_tool::*;
pub use agent_pc_use_ui_snapshot_tool::*;
pub use agent_plugin_asset_provisioner_tool::*;
pub use agent_plugin_bootstrap_tool::*;
pub use agent_plugin_catalog_tool::*;
pub use agent_plugin_runtime_status_tool::*;
pub use apply_code_action_tool::*;
pub use context_server_registry::*;
pub use copy_path_tool::*;
pub use create_directory_tool::*;
pub use delete_path_tool::*;
pub use diagnostics_tool::*;
pub use dx_catalog_provider_settings_registration_tool::*;
pub use dx_catalog_provider_settings_tool::*;
pub use dx_metasearch_status_tool::*;
pub use dx_metasearch_tool::*;
pub use edit_file_tool::*;
pub use fetch_tool::*;
pub use find_path_tool::*;
pub use find_references_tool::*;
pub use get_code_actions_tool::*;
pub use go_to_definition_tool::*;
pub use grep_tool::*;
pub use list_directory_tool::*;
pub use move_path_tool::*;
pub use read_file_tool::*;
pub use rename_tool::*;
pub use skill_tool::*;
pub use spawn_agent_tool::*;
pub use symbol_locator::*;
pub use terminal_tool::*;
pub use tool_permissions::*;
pub use update_plan_tool::*;
pub use web_search_tool::*;
pub use write_file_tool::*;

macro_rules! tools {
    ($($tool:ty),* $(,)?) => {
        /// Every built-in tool name, determined at compile time.
        pub const ALL_TOOL_NAMES: &[&str] = &[
            $(<$tool>::NAME,)*
        ];

        const _: () = {
            const fn str_eq(a: &str, b: &str) -> bool {
                let a = a.as_bytes();
                let b = b.as_bytes();
                if a.len() != b.len() {
                    return false;
                }
                let mut i = 0;
                while i < a.len() {
                    if a[i] != b[i] {
                        return false;
                    }
                    i += 1;
                }
                true
            }

            const NAMES: &[&str] = ALL_TOOL_NAMES;
            let mut i = 0;
            while i < NAMES.len() {
                let mut j = i + 1;
                while j < NAMES.len() {
                    if str_eq(NAMES[i], NAMES[j]) {
                        panic!("Duplicate tool name in tools! macro");
                    }
                    j += 1;
                }
                i += 1;
            }
        };

        /// Returns whether the tool with the given name supports the given provider.
        pub fn tool_supports_provider(name: &str, provider: &language_model::LanguageModelProviderId) -> bool {
            $(
                if name == <$tool>::NAME {
                    return <$tool>::supports_provider(provider);
                }
            )*
            false
        }

        /// A list of all built-in tools
        pub fn built_in_tools() -> impl Iterator<Item = LanguageModelRequestTool> {
            fn language_model_tool<T: AgentTool>() -> LanguageModelRequestTool {
                LanguageModelRequestTool {
                    name: T::NAME.to_string(),
                    description: T::description().to_string(),
                    input_schema: T::input_schema(LanguageModelToolSchemaFormat::JsonSchema).to_value(),
                    use_input_streaming: T::supports_input_streaming(),
                }
            }
            [
                $(
                    language_model_tool::<$tool>(),
                )*
            ]
            .into_iter()
        }
    };
}

tools! {
    AgentPluginAssetProvisionerTool,
    AgentPluginBootstrapTool,
    AgentBrowserPayloadTool,
    AgentBrowserPayloadStageTool,
    AgentBrowserPayloadQueueTool,
    AgentBrowserPayloadQueueInspectTool,
    AgentChromePayloadTool,
    AgentChromePayloadQueueTool,
    AgentChromePayloadQueueInspectTool,
    AgentChromeRunnerGateTool,
    AgentChromePlaywrightAdapterTool,
    AgentChromePlaywrightInvokeTool,
    AgentChromePlaywrightExecutionInspectTool,
    AgentPcUseInspectTool,
    AgentPcUsePayloadTool,
    AgentPcUsePayloadStageTool,
    AgentPcUsePayloadQueueTool,
    AgentPcUsePayloadQueueInspectTool,
    AgentPcUseRunnerGateTool,
    AgentPcUseRunnerReceiptInspectTool,
    AgentPcUseTargetManifestTool,
    AgentPcUseTargetSnapshotTool,
    AgentPcUseUiSnapshotTool,
    AgentPcUseUiSnapshotContractTool,
    AgentPluginCatalogTool,
    AgentPluginRuntimeStatusTool,
    ApplyCodeActionTool,
    CopyPathTool,
    CreateDirectoryTool,
    DeletePathTool,
    DiagnosticsTool,
    DxMetasearchStatusTool,
    DxMetasearchTool,
    DxCatalogProviderSettingsRegistrationTool,
    DxCatalogProviderSettingsTool,
    EditFileTool,
    FetchTool,
    FindPathTool,
    FindReferencesTool,
    GetCodeActionsTool,
    GoToDefinitionTool,
    GrepTool,
    ListDirectoryTool,
    MovePathTool,
    ReadFileTool,
    RenameTool,
    SkillTool,
    SpawnAgentTool,
    TerminalTool,
    UpdatePlanTool,
    WebSearchTool,
    WriteFileTool,
}
