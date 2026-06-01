use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use gpui::App;
use serde_json::Value;
use settings::SettingsStore;

const DEFAULT_DX_CLI: &str = "dx";
const DEFAULT_AGENT_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\agents";
const DEFAULT_PROVIDER_CATALOG_PATH: &str = r"G:\Dx\.dx\catalog\agents\provider-model-catalog.rkyv";
const SNAPSHOT_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

mod command_safety;
mod commands;
mod local_file_labels;
mod local_files;
mod receipts;
mod runtime;

use self::command_safety::{
    bridge_command_label, is_dx_agents_command, is_public_dx_agents_command, is_safe_platform_arg,
    is_secret_like_arg, public_command_for_runtime, redact_action_scalar,
};
use self::local_files::{dx_home_from_receipt_root, latest_receipts, read_first_json, read_json};

pub(crate) use self::commands::{
    DxAgentMetadataCommand, DxAgentPublicCommand, run_dx_agent_metadata_command,
    run_dx_agent_public_command,
};

use self::{
    receipts::{
        action_error, contract_summary, import_summary, receipt_inbox, receipt_index_summary,
        receipts, release_gate,
    },
    runtime::{
        DxAgentSocialActionKind, automations, catalog_summary, connected_accounts_summary, models,
        providers, social_accounts, social_action_summary,
    },
};

#[derive(Clone)]
pub(crate) struct DxAgentBridgeSnapshot {
    pub enabled: bool,
    pub cli_actions_allowed: bool,
    pub cli_path: String,
    pub receipt_root: PathBuf,
    pub root_exists: bool,
    pub status: String,
    pub connected_accounts_summary: DxConnectedAccountsSummary,
    pub automation_count: usize,
    pub active_task_count: usize,
    pub last_error: Option<String>,
    pub next_action: String,
    pub social_accounts: Vec<DxAgentSocialAccount>,
    pub social_connect: DxAgentSocialActionSummary,
    pub social_disconnect: DxAgentSocialActionSummary,
    pub automations: Vec<DxAgentAutomation>,
    pub providers: Vec<DxAgentProvider>,
    pub models: Vec<DxAgentModel>,
    pub catalog: DxAgentCatalogSummary,
    pub contract_summary: DxAgentContractSummary,
    pub import_summary: DxAgentImportSummary,
    pub release_gate: DxAgentReleaseGateSummary,
    pub action_error: DxAgentActionErrorSummary,
    pub receipt_inbox: DxAgentReceiptInboxSummary,
    pub receipt_index: DxAgentReceiptIndexSummary,
    pub receipts: Vec<DxAgentReceipt>,
    pub latest_receipts: Vec<String>,
    pub show_managed_providers: bool,
    pub show_in_agent_rail: bool,
}

#[derive(Clone, Default)]
pub(crate) struct DxConnectedAccountsSummary {
    pub supported: usize,
    pub configured: usize,
    pub connected: usize,
    pub needs_connection: usize,
    pub needs_auth: usize,
    pub qr_connect_supported: usize,
}

#[derive(Clone)]
pub(crate) struct DxAgentSocialAccount {
    pub platform: String,
    pub label: String,
    pub status: String,
    pub configured: bool,
    pub connected: bool,
    pub qr_connect_supported: bool,
    pub actions: Vec<DxAgentRowAction>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentRowAction {
    pub id: String,
    pub label: String,
    pub command: String,
    pub public_command: String,
    pub enabled: bool,
    pub user_action_required: bool,
    pub writes_receipt: bool,
    pub receipt_filename: String,
    pub refresh_command: String,
    pub public_refresh_command: String,
    pub secrets_exposed: bool,
}

#[derive(Clone)]
pub(crate) struct DxAgentSocialActionSummary {
    pub action: &'static str,
    pub present: bool,
    pub status: String,
    pub platform: String,
    pub label: String,
    pub connected: Option<bool>,
    pub connect_supported: bool,
    pub disconnect_supported: bool,
    pub qr_supported: bool,
    pub link_supported: bool,
    pub connect_method: String,
    pub manual_revoke_required: bool,
    pub explicit_user_action_required: bool,
    pub safe_config_state: String,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentAutomation {
    pub id: String,
    pub status: String,
    pub enabled: bool,
    pub schedule_kind: String,
    pub source: String,
    pub actions: Vec<DxAgentRowAction>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentProvider {
    pub id: String,
    pub display_name: String,
    pub status: String,
    pub configured: bool,
    pub active: bool,
    pub local: bool,
    pub compatibility: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxAgentModel {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub status: String,
    pub active: bool,
    pub compatibility: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxAgentCatalogSummary {
    pub path: PathBuf,
    pub present: bool,
    pub stale: bool,
    pub provider_count: usize,
    pub model_count: usize,
    pub source_hash: Option<String>,
    pub safe_regeneration_command: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentContractSummary {
    pub present: bool,
    pub status: String,
    pub command_count: usize,
    pub receipt_count: usize,
    pub provider_catalog_source: String,
    pub provider_catalog_receipt_count: usize,
    pub safe_regeneration_command: String,
    pub redaction_summary: String,
    pub redaction_requires_review: bool,
    pub next_action: String,
    pub commands: Vec<String>,
    pub receipt_notes: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct DxAgentRecoveryControlCounts {
    pub required_intent_count: usize,
    pub action_count: usize,
    pub check_count: usize,
}

impl DxAgentRecoveryControlCounts {
    pub(crate) fn label(&self) -> String {
        if self.required_intent_count == 0 && self.action_count == 0 && self.check_count == 0 {
            "counts unavailable".to_string()
        } else {
            format!(
                "{} intent(s) / {} action(s) / {} check(s)",
                self.required_intent_count, self.action_count, self.check_count
            )
        }
    }
}

#[derive(Clone)]
pub(crate) struct DxAgentImportSummary {
    pub present: bool,
    pub status: String,
    pub operator_summary: String,
    pub release_gate_status: String,
    pub release_gate_warning_count: usize,
    pub release_gate_failed_count: usize,
    pub action_map_status: String,
    pub no_command_fanout: bool,
    pub recovery_controls_status: String,
    pub recovery_render_first: String,
    pub recovery_counts: DxAgentRecoveryControlCounts,
    pub recovery_states: Vec<String>,
    pub recovery_fixture_count: usize,
    pub freshness_state: String,
    pub next_action: String,
    pub warning_reasons: Vec<String>,
    pub blocking_reasons: Vec<String>,
    pub recovery_commands: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxAgentReleaseGateSummary {
    pub present: bool,
    pub status: String,
    pub operator_summary: String,
    pub acceptance_count: usize,
    pub passed_count: usize,
    pub warning_count: usize,
    pub failed_count: usize,
    pub packet_count: usize,
    pub fixture_family_count: usize,
    pub receipt_count: usize,
    pub retained_run_overflow_count: usize,
    pub import_manifest_status: String,
    pub smoke_status: String,
    pub receipt_inbox_status: String,
    pub retention_preview_status: String,
    pub action_map_status: String,
    pub no_command_fanout: bool,
    pub recovery_controls_status: String,
    pub recovery_render_first: String,
    pub recovery_counts: DxAgentRecoveryControlCounts,
    pub recovery_fixture_count: usize,
    pub next_action: String,
    pub warning_reasons: Vec<String>,
    pub blocking_reasons: Vec<String>,
    pub acceptance_rows: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxAgentReceiptIndexSummary {
    pub present: bool,
    pub status: String,
    pub receipt_root_present: Option<bool>,
    pub receipt_count: usize,
    pub returned_receipt_count: usize,
    pub active_task_count: usize,
    pub latest_receipt_path: Option<String>,
    pub last_error: Option<String>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentReceiptInboxSummary {
    pub present: bool,
    pub status: String,
    pub receipt_dir_present: Option<bool>,
    pub receipt_count: usize,
    pub latest_count: usize,
    pub malformed_count: usize,
    pub missing_latest_count: usize,
    pub stale_count: usize,
    pub expired_count: usize,
    pub last_error: Option<String>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentActionErrorSummary {
    pub present: bool,
    pub status: String,
    pub command: String,
    pub error: Option<String>,
    pub generated_at: String,
    pub next_action: String,
    pub redaction_summary: String,
    pub redaction_requires_review: bool,
}

#[derive(Clone)]
pub(crate) struct DxAgentReceipt {
    pub id: String,
    pub kind: String,
    pub schema_version: String,
    pub command: String,
    pub generated_at: String,
    pub task_id: String,
    pub task_state: String,
    pub status: String,
    pub active_task: bool,
    pub safe_to_render: bool,
    pub metadata_redacted: bool,
    pub receipt_path: String,
    pub size_bytes: usize,
    pub modified_at: String,
    pub last_error: Option<String>,
    pub next_action: String,
    pub provider_status: Option<String>,
    pub model_status: Option<String>,
    pub duration_state: Option<String>,
    pub retry_supported: Option<bool>,
    pub cancel_supported: Option<bool>,
    pub social_connected: Option<usize>,
    pub social_needs_auth: Option<usize>,
    pub automation_enabled: Option<usize>,
    pub automation_warning: Option<usize>,
}

static SNAPSHOT_CACHE: OnceLock<Mutex<Option<(Instant, String, DxAgentBridgeSnapshot)>>> =
    OnceLock::new();

pub(crate) fn dx_agent_bridge_snapshot(cx: &App) -> DxAgentBridgeSnapshot {
    let settings = dx_agent_settings(cx);
    let cache_key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        settings.enabled,
        settings.cli_actions_allowed,
        settings.cli_path,
        settings.receipt_root.display(),
        settings.provider_catalog_path.display(),
        settings.show_managed_providers,
        settings.show_in_agent_rail
    );
    let now = Instant::now();
    let cache = SNAPSHOT_CACHE.get_or_init(|| Mutex::new(None));

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_key, snapshot)) = cache.as_ref() {
            if cached_key == &cache_key && now.duration_since(*cached_at) <= SNAPSHOT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = read_bridge_snapshot(settings);
        *cache = Some((now, cache_key, snapshot.clone()));
        return snapshot;
    }

    read_bridge_snapshot(settings)
}

pub(crate) fn dx_agent_dx_home(cx: &App) -> Option<PathBuf> {
    dx_home_from_receipt_root(&dx_agent_settings(cx).receipt_root)
}

pub(crate) fn dx_agent_receipt_root(cx: &App) -> PathBuf {
    dx_agent_settings(cx).receipt_root
}

pub(crate) fn dx_agent_cli_actions_allowed(cx: &App) -> bool {
    let settings = dx_agent_settings(cx);
    settings.enabled && settings.cli_actions_allowed
}

pub(crate) fn dx_agent_cli_path(cx: &App) -> String {
    dx_agent_settings(cx).cli_path
}

fn clear_snapshot_cache() {
    if let Some(cache) = SNAPSHOT_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            *cache = None;
        }
    }
}

#[derive(Clone)]
struct DxAgentSettingsSnapshot {
    enabled: bool,
    cli_actions_allowed: bool,
    cli_path: String,
    receipt_root: PathBuf,
    provider_catalog_path: PathBuf,
    show_managed_providers: bool,
    show_in_agent_rail: bool,
}

fn dx_agent_settings(cx: &App) -> DxAgentSettingsSnapshot {
    let merged = cx.global::<SettingsStore>().merged_settings();
    let settings = merged
        .agent
        .as_ref()
        .and_then(|agent| agent.dx_agents.as_ref());
    DxAgentSettingsSnapshot {
        enabled: settings
            .and_then(|settings| settings.enabled)
            .unwrap_or(true),
        cli_actions_allowed: settings
            .and_then(|settings| settings.allow_cli_actions)
            .unwrap_or(true),
        cli_path: settings
            .and_then(|settings| settings.cli_path.clone())
            .filter(|path| !path.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DX_CLI.to_string()),
        receipt_root: settings
            .and_then(|settings| settings.receipt_root.clone())
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_RECEIPT_ROOT)),
        provider_catalog_path: settings
            .and_then(|settings| settings.provider_catalog_path.clone())
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_CATALOG_PATH)),
        show_managed_providers: settings
            .and_then(|settings| settings.show_managed_providers)
            .unwrap_or(true),
        show_in_agent_rail: settings
            .and_then(|settings| settings.show_in_agent_rail)
            .unwrap_or(true),
    }
}

fn read_bridge_snapshot(settings: DxAgentSettingsSnapshot) -> DxAgentBridgeSnapshot {
    let root_exists = settings.receipt_root.is_dir();
    let status_value = read_json(&settings.receipt_root.join("status-latest.json"));
    let social_value = read_json(&settings.receipt_root.join("social-list-latest.json"));
    let social_connect_value = read_json(&settings.receipt_root.join("social-connect-latest.json"));
    let social_disconnect_value =
        read_json(&settings.receipt_root.join("social-disconnect-latest.json"));
    let automation_value = read_json(&settings.receipt_root.join("automate-list-latest.json"));
    let provider_value = read_json(&settings.receipt_root.join("providers-list-latest.json"));
    let model_value = read_json(&settings.receipt_root.join("models-list-latest.json"));
    let receipts_value = read_json(&settings.receipt_root.join("receipts-list-latest.json"));
    let contract_value = read_first_json(&settings.receipt_root, &["contract-latest.json"]);
    let import_summary_value = read_first_json(
        &settings.receipt_root,
        &["import-summary-latest.json", "import_summary-latest.json"],
    );
    let release_gate_value = read_first_json(
        &settings.receipt_root,
        &["release-gate-latest.json", "release_gate-latest.json"],
    );
    let action_error_value = read_first_json(
        &settings.receipt_root,
        &["action-error-latest.json", "action_error-latest.json"],
    );
    let receipt_inbox_value = read_first_json(
        &settings.receipt_root,
        &["receipts-inbox-latest.json", "receipts_inbox-latest.json"],
    );

    let status = status_value
        .as_ref()
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_status_receipt".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let connected_accounts_summary = status_value
        .as_ref()
        .and_then(|value| value.get("connected_accounts_summary"))
        .map(connected_accounts_summary)
        .unwrap_or_default();
    let automation_count = status_value
        .as_ref()
        .and_then(|value| usize_field(value, &["automation_count"]))
        .or_else(|| {
            automation_value
                .as_ref()
                .and_then(|value| usize_field(value, &["automation_count"]))
        })
        .unwrap_or_default();
    let receipt_index = receipt_index_summary(receipts_value.as_ref(), root_exists);
    let receipts = receipts_value.as_ref().map(receipts).unwrap_or_default();
    let active_task_count = status_value
        .as_ref()
        .and_then(|value| usize_field(value, &["active_task_count"]))
        .or_else(|| {
            receipts_value
                .as_ref()
                .and_then(|value| usize_field(value, &["active_task_count"]))
        })
        .unwrap_or_default();
    let last_error = status_value
        .as_ref()
        .and_then(|value| string_field(value, &["last_error"]));
    let next_action = status_value
        .as_ref()
        .and_then(|value| string_field(value, &["next_action"]))
        .unwrap_or_else(|| {
            if root_exists {
                "Run the DX Agents status command to refresh the bridge receipt.".to_string()
            } else {
                format!(
                    "Create or refresh DX Agents receipts at {}.",
                    settings.receipt_root.display()
                )
            }
        });

    DxAgentBridgeSnapshot {
        enabled: settings.enabled,
        cli_actions_allowed: settings.cli_actions_allowed,
        cli_path: settings.cli_path,
        receipt_root: settings.receipt_root.clone(),
        root_exists,
        status,
        connected_accounts_summary,
        automation_count,
        active_task_count,
        last_error,
        next_action,
        social_accounts: social_value
            .as_ref()
            .map(social_accounts)
            .unwrap_or_default(),
        social_connect: social_action_summary(
            social_connect_value.as_ref(),
            root_exists,
            DxAgentSocialActionKind::Connect,
        ),
        social_disconnect: social_action_summary(
            social_disconnect_value.as_ref(),
            root_exists,
            DxAgentSocialActionKind::Disconnect,
        ),
        automations: automation_value
            .as_ref()
            .map(automations)
            .unwrap_or_default(),
        providers: provider_value.as_ref().map(providers).unwrap_or_default(),
        models: model_value.as_ref().map(models).unwrap_or_default(),
        catalog: catalog_summary(
            provider_value.as_ref(),
            model_value.as_ref(),
            settings.provider_catalog_path.clone(),
        ),
        contract_summary: contract_summary(contract_value.as_ref(), root_exists),
        import_summary: import_summary(import_summary_value.as_ref(), root_exists),
        release_gate: release_gate(release_gate_value.as_ref(), root_exists),
        action_error: action_error(action_error_value.as_ref()),
        receipt_inbox: receipt_inbox(receipt_inbox_value.as_ref(), root_exists),
        receipt_index,
        receipts,
        latest_receipts: latest_receipts(&settings.receipt_root, root_exists),
        show_managed_providers: settings.show_managed_providers,
        show_in_agent_rail: settings.show_in_agent_rail,
    }
}

fn array_field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    value_at(value, path)?.as_array()
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)?.as_str().map(ToString::to_string)
}

fn safe_string_field(value: &Value, path: &[&str]) -> Option<String> {
    string_field(value, path).map(|value| redact_action_scalar(&value))
}

fn string_array_field(value: &Value, path: &[&str]) -> Vec<String> {
    array_field(value, path)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn bool_field(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path)?.as_bool()
}

fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |value, key| value.get(*key))
}

#[cfg(test)]
mod tests {
    use super::{action_error, import_summary, release_gate};
    use serde_json::json;

    #[test]
    fn dx_agent_action_error_redaction_flags_drive_review_state() {
        let safe = json!({
            "schema_version": "dx.agents.zed.action_error.v1",
            "command": "dx agents status --json",
            "status": "missing_config",
            "error": "failed to run dx agents status --json",
            "generated_at": "1779310640000",
            "next_action": "review_dx_agents_cli_path_or_receipt_root",
            "redaction": {
                "exports_secret_values": false,
                "exports_provider_credentials": false,
                "exports_receipt_bodies": false
            }
        });
        let safe_summary = action_error(Some(&safe));
        assert!(safe_summary.present);
        assert!(!safe_summary.redaction_requires_review);
        assert_eq!(
            safe_summary.redaction_summary,
            "Action-error receipt is redacted metadata only"
        );

        let unsafe_packet = json!({
            "schema_version": "dx.agents.zed.action_error.v1",
            "command": "dx agents status --json",
            "status": "missing_config",
            "redaction": {
                "exports_secret_values": true,
                "exports_provider_credentials": false,
                "exports_receipt_bodies": false
            }
        });
        let unsafe_summary = action_error(Some(&unsafe_packet));
        assert!(unsafe_summary.redaction_requires_review);
        assert_eq!(
            unsafe_summary.redaction_summary,
            "Action-error receipt redaction requires review"
        );
    }

    #[test]
    fn dx_agent_import_summary_uses_flat_recovery_control_counts() {
        let packet = json!({
            "status": "ready",
            "release_gate": {
                "status": "ready",
                "warning_count": 0,
                "failed_count": 0
            },
            "action_map": {
                "status": "ready",
                "action_count": 99,
                "required_intent_count": 88,
                "check_count": 77,
                "no_command_fanout": true
            },
            "recovery_controls": {
                "status": "ready",
                "render_first": "agent_bridge_recovery_controls",
                "fixture_count": 3,
                "action_count": 10,
                "required_intent_count": 6,
                "check_count": 5,
                "no_command_fanout": true
            },
            "freshness_policy": {
                "latest_freshness_state": "fresh"
            }
        });

        let summary = import_summary(Some(&packet), true);

        assert_eq!(summary.recovery_counts.required_intent_count, 6);
        assert_eq!(summary.recovery_counts.action_count, 10);
        assert_eq!(summary.recovery_counts.check_count, 5);
        assert_eq!(
            summary.recovery_counts.label(),
            "6 intent(s) / 10 action(s) / 5 check(s)"
        );
    }

    #[test]
    fn dx_agent_release_gate_uses_flat_recovery_control_counts() {
        let packet = json!({
            "status": "ready",
            "acceptance_count": 10,
            "passed_count": 10,
            "warning_count": 0,
            "failed_count": 0,
            "action_map_status": "ready",
            "no_command_fanout": true,
            "recovery_controls": {
                "status": "ready",
                "render_first": "agent_bridge_recovery_controls",
                "fixture_count": 3,
                "action_count": 10,
                "required_intent_count": 6,
                "check_count": 5,
                "no_command_fanout": true
            }
        });

        let summary = release_gate(Some(&packet), true);

        assert_eq!(summary.recovery_counts.required_intent_count, 6);
        assert_eq!(summary.recovery_counts.action_count, 10);
        assert_eq!(summary.recovery_counts.check_count, 5);
        assert_eq!(
            summary.recovery_counts.label(),
            "6 intent(s) / 10 action(s) / 5 check(s)"
        );
    }
}
