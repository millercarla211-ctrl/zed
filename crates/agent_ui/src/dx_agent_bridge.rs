use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::App;
use serde_json::{Value, json};
use settings::SettingsStore;

const DEFAULT_DX_CLI: &str = "dx";
const DEFAULT_AGENT_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\agents";
const DEFAULT_PROVIDER_CATALOG_PATH: &str = r"G:\Dx\.dx\catalog\agents\provider-model-catalog.rkyv";
const SNAPSHOT_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

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

#[derive(Clone)]
pub(crate) enum DxAgentPublicCommand {
    Contract,
    Status,
    Run,
    ReceiptsList,
    SocialList,
    SocialConnect { platform: String },
    SocialDisconnect { platform: String },
    AutomationsList,
    ProvidersList,
    ModelsList,
    ProviderCatalogRegenerate,
}

impl DxAgentPublicCommand {
    fn args(&self) -> Vec<String> {
        match self {
            Self::Contract => dx_agents_args(&["contract"]),
            Self::Status => dx_agents_args(&["status"]),
            Self::Run => dx_agents_args(&["run"]),
            Self::ReceiptsList => dx_agents_args(&["receipts", "list"]),
            Self::SocialList => dx_agents_args(&["social", "list"]),
            Self::SocialConnect { platform } => {
                dx_agents_platform_args("connect", platform.as_str())
            }
            Self::SocialDisconnect { platform } => {
                dx_agents_platform_args("disconnect", platform.as_str())
            }
            Self::AutomationsList => dx_agents_args(&["automate", "list"]),
            Self::ProvidersList => dx_agents_args(&["providers", "list"]),
            Self::ModelsList => dx_agents_args(&["models", "list"]),
            Self::ProviderCatalogRegenerate => {
                dx_agents_args(&["providers", "catalog", "regenerate"])
            }
        }
    }

    fn is_safe(&self) -> bool {
        match self {
            Self::SocialConnect { platform } | Self::SocialDisconnect { platform } => {
                is_safe_platform_arg(platform)
            }
            _ => true,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DxAgentMetadataCommand {
    ImportSummary,
    ReleaseGate,
    ReceiptsInbox,
}

impl DxAgentMetadataCommand {
    fn args(self) -> Vec<String> {
        match self {
            Self::ImportSummary => dx_agents_args(&["import-summary"]),
            Self::ReleaseGate => dx_agents_args(&["release-gate"]),
            Self::ReceiptsInbox => dx_agents_args(&["receipts"]),
        }
    }

    fn receipt_filename(self) -> &'static str {
        match self {
            Self::ImportSummary => "import-summary-latest.json",
            Self::ReleaseGate => "release-gate-latest.json",
            Self::ReceiptsInbox => "receipts-inbox-latest.json",
        }
    }

    fn expected_schema(self) -> &'static str {
        match self {
            Self::ImportSummary => "dx.agents.zed.import_summary.v1",
            Self::ReleaseGate => "dx.agents.zed.release_gate.v1",
            Self::ReceiptsInbox => "dx.agents.zed.receipts.v1",
        }
    }
}

pub(crate) fn run_dx_agent_public_command(
    command: DxAgentPublicCommand,
    cli_path: String,
    dx_home: Option<PathBuf>,
    receipt_root: PathBuf,
) -> Result<()> {
    if !command.is_safe() {
        return Err(anyhow!("unsupported DX Agents public bridge command"));
    }

    let args = command.args();
    let command_label = bridge_command_label(&cli_path, &args);
    if let Err(error) = run_bridge_command(cli_path, args, dx_home) {
        let _ = write_action_error_receipt(&receipt_root, &command_label, &error);
        clear_snapshot_cache();
        return Err(error);
    }
    clear_action_error_receipt(&receipt_root);
    clear_snapshot_cache();
    Ok(())
}

pub(crate) fn run_dx_agent_metadata_command(
    command: DxAgentMetadataCommand,
    cli_path: String,
    dx_home: Option<PathBuf>,
    receipt_root: PathBuf,
) -> Result<()> {
    let args = command.args();
    let command_label = bridge_command_label(&cli_path, &args);
    let output = match run_bridge_command(cli_path, args, dx_home) {
        Ok(output) => output,
        Err(error) => {
            let _ = write_action_error_receipt(&receipt_root, &command_label, &error);
            clear_snapshot_cache();
            return Err(error);
        }
    };
    write_json_receipt(
        &receipt_root.join(command.receipt_filename()),
        &output.stdout,
        command.expected_schema(),
    )?;
    clear_action_error_receipt(&receipt_root);
    clear_snapshot_cache();
    Ok(())
}

fn dx_agents_args(args: &[&str]) -> Vec<String> {
    let mut command = Vec::with_capacity(args.len() + 2);
    command.push("agents".to_string());
    command.extend(args.iter().map(|arg| (*arg).to_string()));
    command.push("--json".to_string());
    command
}

fn dx_agents_platform_args(action: &str, platform: &str) -> Vec<String> {
    vec![
        "agents".to_string(),
        "social".to_string(),
        action.to_string(),
        "--platform".to_string(),
        platform.to_string(),
        "--json".to_string(),
    ]
}

fn run_bridge_command(
    cli_path: String,
    args: Vec<String>,
    dx_home: Option<PathBuf>,
) -> Result<Output> {
    if args.iter().any(|arg| is_secret_like_arg(arg)) {
        return Err(anyhow!(
            "DX Agents bridge commands cannot include secret-like arguments"
        ));
    }

    let mut command = Command::new(&cli_path);
    command.args(&args);
    if let Some(dx_home) = dx_home {
        command.env("DX_HOME", dx_home);
    }

    let output = command
        .output()
        .with_context(|| format!("failed to run `{}`", bridge_command_label(&cli_path, &args)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "`{}` failed: {}",
            bridge_command_label(&cli_path, &args),
            stderr.trim()
        ));
    }

    Ok(output)
}

fn write_json_receipt(path: &Path, stdout: &[u8], expected_schema: &str) -> Result<()> {
    if u64::try_from(stdout.len()).unwrap_or(u64::MAX) > MAX_RECEIPT_BYTES {
        return Err(anyhow!("DX Agents metadata response is too large"));
    }

    let value: Value = serde_json::from_slice(stdout)
        .context("DX Agents metadata command returned invalid JSON")?;
    let schema_version = string_field(&value, &["schema_version"])
        .ok_or_else(|| anyhow!("DX Agents metadata JSON is missing schema_version"))?;
    if schema_version != expected_schema {
        return Err(anyhow!(
            "DX Agents metadata JSON schema mismatch: expected {expected_schema}, got {schema_version}"
        ));
    }

    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("DX Agents metadata receipt path has no parent"))?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create DX Agents metadata receipt directory `{}`",
            parent.display()
        )
    })?;

    let mut bytes =
        serde_json::to_vec_pretty(&value).context("failed to serialize DX Agents metadata JSON")?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| {
        format!(
            "failed to write DX Agents metadata receipt `{}`",
            path.display()
        )
    })?;

    Ok(())
}

fn write_action_error_receipt(
    receipt_root: &Path,
    command: &str,
    error: &anyhow::Error,
) -> Result<()> {
    let path = receipt_root.join("action-error-latest.json");
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("DX Agents action error receipt path has no parent"))?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create DX Agents action error receipt directory `{}`",
            parent.display()
        )
    })?;

    let generated_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let value = json!({
        "schema_version": "dx.agents.zed.action_error.v1",
        "command": redact_action_scalar(command),
        "status": "missing_config",
        "generated_at": generated_at_ms.to_string(),
        "generated_at_ms": generated_at_ms,
        "error": redact_action_scalar(&error.to_string()),
        "next_action": "review_dx_agents_cli_path_or_receipt_root",
        "redaction": {
            "exports_secret_values": false,
            "exports_provider_credentials": false,
            "exports_receipt_bodies": false
        }
    });
    let mut bytes =
        serde_json::to_vec_pretty(&value).context("failed to serialize DX Agents action error")?;
    bytes.push(b'\n');
    fs::write(&path, bytes).with_context(|| {
        format!(
            "failed to write DX Agents action error receipt `{}`",
            path.display()
        )
    })?;
    Ok(())
}

fn clear_action_error_receipt(receipt_root: &Path) {
    let path = receipt_root.join("action-error-latest.json");
    if path.is_file() {
        let _ = fs::remove_file(path);
    }
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

fn connected_accounts_summary(value: &Value) -> DxConnectedAccountsSummary {
    let needs_connection = usize_field(value, &["needs_connection"]).unwrap_or_default();
    DxConnectedAccountsSummary {
        supported: usize_field(value, &["supported"]).unwrap_or_default(),
        configured: usize_field(value, &["configured"]).unwrap_or_default(),
        connected: usize_field(value, &["connected"]).unwrap_or_default(),
        needs_connection,
        needs_auth: usize_field(value, &["needs_auth"]).unwrap_or(needs_connection),
        qr_connect_supported: usize_field(value, &["qr_connect_supported"]).unwrap_or_default(),
    }
}

fn social_accounts(value: &Value) -> Vec<DxAgentSocialAccount> {
    array_field(value, &["accounts"])
        .map(|accounts| {
            accounts
                .iter()
                .take(12)
                .map(|account| DxAgentSocialAccount {
                    platform: string_field(account, &["platform"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    label: string_field(account, &["label"])
                        .unwrap_or_else(|| "Account".to_string()),
                    status: string_field(account, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    configured: bool_field(account, &["configured"]).unwrap_or(false),
                    connected: bool_field(account, &["connected"]).unwrap_or(false),
                    qr_connect_supported: bool_field(account, &["qr_connect_supported"])
                        .unwrap_or(false),
                    actions: social_row_actions(account),
                    next_action: string_field(account, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
enum DxAgentSocialActionKind {
    Connect,
    Disconnect,
}

fn social_action_summary(
    value: Option<&Value>,
    root_exists: bool,
    kind: DxAgentSocialActionKind,
) -> DxAgentSocialActionSummary {
    let action = match kind {
        DxAgentSocialActionKind::Connect => "connect",
        DxAgentSocialActionKind::Disconnect => "disconnect",
    };
    let command = match kind {
        DxAgentSocialActionKind::Connect => "dx agents social connect --json",
        DxAgentSocialActionKind::Disconnect => "dx agents social disconnect --json",
    };
    let waiting_status = match kind {
        DxAgentSocialActionKind::Connect => "waiting_for_social_connect_receipt",
        DxAgentSocialActionKind::Disconnect => "waiting_for_social_disconnect_receipt",
    };
    let account = value.and_then(|value| value.get("account"));
    let flow = value.and_then(|value| value.get("flow"));

    DxAgentSocialActionSummary {
        action,
        present: value.is_some(),
        status: value
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| {
                if root_exists {
                    waiting_status.to_string()
                } else {
                    "missing_receipt_root".to_string()
                }
            }),
        platform: account
            .and_then(|account| string_field(account, &["platform"]))
            .unwrap_or_else(|| "unknown".to_string()),
        label: account
            .and_then(|account| string_field(account, &["label"]))
            .unwrap_or_else(|| "Social account".to_string()),
        connected: account.and_then(|account| bool_field(account, &["connected"])),
        connect_supported: flow
            .and_then(|flow| bool_field(flow, &["connect_supported"]))
            .unwrap_or(false),
        disconnect_supported: flow
            .and_then(|flow| bool_field(flow, &["disconnect_supported"]))
            .unwrap_or(false),
        qr_supported: flow
            .and_then(|flow| bool_field(flow, &["qr_supported"]))
            .unwrap_or(false),
        link_supported: flow
            .and_then(|flow| bool_field(flow, &["link_supported"]))
            .unwrap_or(false),
        connect_method: flow
            .and_then(|flow| string_field(flow, &["connect_method"]))
            .unwrap_or_else(|| "none".to_string()),
        manual_revoke_required: flow
            .and_then(|flow| bool_field(flow, &["manual_revoke_required"]))
            .unwrap_or(false),
        explicit_user_action_required: flow
            .and_then(|flow| bool_field(flow, &["explicit_user_action_required"]))
            .unwrap_or(false),
        safe_config_state: flow
            .and_then(|flow| string_field(flow, &["safe_config_state"]))
            .unwrap_or_else(|| "unknown".to_string()),
        next_action: value
            .and_then(|value| string_field(value, &["next_action"]))
            .unwrap_or_else(|| command.to_string()),
    }
}

fn automations(value: &Value) -> Vec<DxAgentAutomation> {
    array_field(value, &["automations"])
        .map(|automations| {
            automations
                .iter()
                .take(12)
                .map(|automation| DxAgentAutomation {
                    id: string_field(automation, &["id"])
                        .unwrap_or_else(|| "automation".to_string()),
                    status: string_field(automation, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    enabled: bool_field(automation, &["enabled"]).unwrap_or(false),
                    schedule_kind: string_field(automation, &["schedule_kind"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    source: string_field(automation, &["source"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    actions: automation_row_actions(automation),
                    next_action: string_field(automation, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn social_row_actions(value: &Value) -> Vec<DxAgentRowAction> {
    row_actions(value, |id, command, receipt_filename, refresh_command| {
        is_dx_agents_command(refresh_command, "social list --json")
            && match id {
                "connect" => {
                    receipt_filename == "social-connect-latest.json"
                        && is_social_action_command(command, "connect")
                }
                "disconnect" => {
                    receipt_filename == "social-disconnect-latest.json"
                        && is_social_action_command(command, "disconnect")
                }
                "refresh" => {
                    receipt_filename == "social-list-latest.json"
                        && is_dx_agents_command(command, "social list --json")
                }
                _ => false,
            }
    })
}

fn automation_row_actions(value: &Value) -> Vec<DxAgentRowAction> {
    row_actions(value, |id, command, receipt_filename, refresh_command| {
        is_dx_agents_command(refresh_command, "automate list --json")
            && match id {
                "run" => {
                    receipt_filename == "run-latest.json"
                        && is_dx_agents_command(command, "run --json")
                }
                "refresh" => {
                    receipt_filename == "automate-list-latest.json"
                        && is_dx_agents_command(command, "automate list --json")
                }
                _ => false,
            }
    })
}

fn row_actions<F>(value: &Value, is_allowed: F) -> Vec<DxAgentRowAction>
where
    F: Fn(&str, &str, &str, &str) -> bool,
{
    array_field(value, &["actions"])
        .map(|actions| {
            actions
                .iter()
                .take(8)
                .filter_map(|action| row_action(action, &is_allowed))
                .collect()
        })
        .unwrap_or_default()
}

fn row_action<F>(value: &Value, is_allowed: &F) -> Option<DxAgentRowAction>
where
    F: Fn(&str, &str, &str, &str) -> bool,
{
    let id = string_field(value, &["id"])?;
    let command = string_field(value, &["command"])?;
    let public_command = string_field(value, &["public_command"])
        .unwrap_or_else(|| public_command_for_runtime(&command));
    let receipt_filename = string_field(value, &["receipt_filename"])?;
    let refresh_command = string_field(value, &["refresh_command"])?;
    let public_refresh_command = string_field(value, &["public_refresh_command"])
        .unwrap_or_else(|| public_command_for_runtime(&refresh_command));
    let secrets_exposed = bool_field(value, &["secrets_exposed"]).unwrap_or(true);
    let writes_receipt = bool_field(value, &["writes_receipt"]).unwrap_or(false);

    if !writes_receipt
        || secrets_exposed
        || is_secret_like_arg(&command)
        || is_secret_like_arg(&public_command)
        || is_secret_like_arg(&receipt_filename)
        || is_secret_like_arg(&refresh_command)
        || is_secret_like_arg(&public_refresh_command)
        || !is_public_dx_agents_command(&public_command)
        || !is_public_dx_agents_command(&public_refresh_command)
        || !is_allowed(&id, &command, &receipt_filename, &refresh_command)
        || !is_allowed(
            &id,
            &public_command,
            &receipt_filename,
            &public_refresh_command,
        )
    {
        return None;
    }

    Some(DxAgentRowAction {
        label: string_field(value, &["label"]).unwrap_or_else(|| id.clone()),
        id,
        command,
        public_command,
        enabled: bool_field(value, &["enabled"]).unwrap_or(false),
        user_action_required: bool_field(value, &["user_action_required"]).unwrap_or(false),
        writes_receipt,
        receipt_filename,
        refresh_command,
        public_refresh_command,
        secrets_exposed,
    })
}

fn is_social_action_command(command: &str, action: &str) -> bool {
    let runtime_prefix = format!("dx-agents agents social {action}");
    let public_prefix = format!("dx agents social {action}");
    [runtime_prefix.as_str(), public_prefix.as_str()]
        .into_iter()
        .any(|prefix| social_action_command_matches_prefix(command, prefix))
}

fn social_action_command_matches_prefix(command: &str, prefix: &str) -> bool {
    if command == format!("{prefix} --json") {
        return true;
    }

    let platform_prefix = format!("{prefix} --platform ");
    command
        .strip_prefix(&platform_prefix)
        .and_then(|value| value.strip_suffix(" --json"))
        .is_some_and(|platform| is_safe_platform_arg(platform))
}

fn providers(value: &Value) -> Vec<DxAgentProvider> {
    array_field(value, &["providers"])
        .map(|providers| {
            providers
                .iter()
                .take(24)
                .map(|provider| DxAgentProvider {
                    id: string_field(provider, &["id"]).unwrap_or_else(|| "provider".to_string()),
                    display_name: string_field(provider, &["display_name"])
                        .unwrap_or_else(|| "Provider".to_string()),
                    status: string_field(provider, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    configured: bool_field(provider, &["configured"]).unwrap_or(false),
                    active: bool_field(provider, &["active"]).unwrap_or(false),
                    local: bool_field(provider, &["local"]).unwrap_or(false),
                    compatibility: string_array_field(provider, &["compatibility"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn models(value: &Value) -> Vec<DxAgentModel> {
    array_field(value, &["models"])
        .map(|models| {
            models
                .iter()
                .take(24)
                .map(|model| DxAgentModel {
                    id: string_field(model, &["id"]).unwrap_or_else(|| "model".to_string()),
                    provider_id: string_field(model, &["provider_id"])
                        .unwrap_or_else(|| "provider".to_string()),
                    model_id: string_field(model, &["model_id"])
                        .unwrap_or_else(|| "model".to_string()),
                    status: string_field(model, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    active: bool_field(model, &["active"]).unwrap_or(false),
                    compatibility: string_array_field(model, &["compatibility"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn catalog_summary(
    provider_value: Option<&Value>,
    model_value: Option<&Value>,
    default_path: PathBuf,
) -> DxAgentCatalogSummary {
    let catalog = provider_value
        .and_then(|value| value.get("catalog"))
        .or_else(|| model_value.and_then(|value| value.get("catalog")));
    let path = catalog
        .and_then(|catalog| string_field(catalog, &["binary_cache_path"]))
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_path);
    DxAgentCatalogSummary {
        present: catalog
            .and_then(|catalog| bool_field(catalog, &["binary_cache_present"]))
            .unwrap_or_else(|| path.is_file()),
        stale: catalog
            .and_then(|catalog| bool_field(catalog, &["binary_cache_stale"]))
            .unwrap_or(true),
        provider_count: catalog
            .and_then(|catalog| usize_field(catalog, &["provider_count"]))
            .unwrap_or_default(),
        model_count: catalog
            .and_then(|catalog| usize_field(catalog, &["model_count"]))
            .unwrap_or_default(),
        source_hash: catalog.and_then(|catalog| string_field(catalog, &["source_hash"])),
        safe_regeneration_command: catalog
            .and_then(|catalog| string_field(catalog, &["safe_regeneration_command"]))
            .unwrap_or_else(|| "dx agents providers catalog regenerate --json".to_string()),
        path,
    }
}

fn contract_summary(value: Option<&Value>, root_exists: bool) -> DxAgentContractSummary {
    let provider_catalog = value.and_then(|value| value.get("provider_catalog"));
    let redaction = value.and_then(|value| value.get("redaction"));
    let status = value
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_contract_receipt".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });

    let exports_secret_values = redaction
        .and_then(|value| bool_field(value, &["exports_secret_values"]))
        .unwrap_or(false);
    let exports_account_targets = redaction
        .and_then(|value| bool_field(value, &["exports_account_targets"]))
        .unwrap_or(false);
    let exports_automation_bodies = redaction
        .and_then(|value| bool_field(value, &["exports_automation_bodies"]))
        .unwrap_or(false);
    let exports_tool_payloads = redaction
        .and_then(|value| bool_field(value, &["exports_tool_payloads"]))
        .unwrap_or(false);
    let exports_task_payloads = redaction
        .and_then(|value| bool_field(value, &["exports_task_payloads"]))
        .unwrap_or(false);
    let exports_transcripts = redaction
        .and_then(|value| bool_field(value, &["exports_transcripts"]))
        .unwrap_or(false);
    let exports_provider_credentials = redaction
        .and_then(|value| bool_field(value, &["exports_provider_credentials"]))
        .unwrap_or(false);
    let redaction_requires_review = exports_secret_values
        || exports_account_targets
        || exports_automation_bodies
        || exports_tool_payloads
        || exports_task_payloads
        || exports_transcripts
        || exports_provider_credentials;
    let redaction_summary = if redaction_requires_review {
        "review required".to_string()
    } else if redaction.is_some() {
        "metadata only".to_string()
    } else {
        "unknown".to_string()
    };

    DxAgentContractSummary {
        present: value.is_some(),
        status,
        command_count: value
            .and_then(|value| value_at(value, &["commands"]))
            .and_then(|value| value.as_object())
            .map(|commands| commands.len())
            .unwrap_or_default(),
        receipt_count: value
            .and_then(|value| array_field(value, &["receipts"]))
            .map(|receipts| receipts.len())
            .unwrap_or_default(),
        provider_catalog_source: provider_catalog
            .and_then(|value| string_field(value, &["source_format"]))
            .unwrap_or_else(|| "unknown".to_string()),
        provider_catalog_receipt_count: provider_catalog
            .and_then(|value| array_field(value, &["json_receipts"]))
            .map(|receipts| receipts.len())
            .unwrap_or_default(),
        safe_regeneration_command: provider_catalog
            .and_then(|value| string_field(value, &["safe_regeneration_command"]))
            .unwrap_or_else(|| "dx agents providers catalog regenerate --json".to_string()),
        redaction_summary,
        redaction_requires_review,
        next_action: value
            .and_then(|value| string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents contract --json".to_string()),
        commands: value
            .map(|value| string_values_field(value, &["commands"]))
            .unwrap_or_default(),
        receipt_notes: receipt_notes(value),
    }
}

fn receipt_notes(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|value| array_field(value, &["receipts"]))
        .map(|receipts| {
            receipts
                .iter()
                .filter_map(|receipt| {
                    let name = string_field(receipt, &["name"])?;
                    let command = string_field(receipt, &["command"]).unwrap_or_default();
                    if command.is_empty() {
                        Some(name)
                    } else {
                        Some(format!("{name}: {command}"))
                    }
                })
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

fn import_summary(value: Option<&Value>, root_exists: bool) -> DxAgentImportSummary {
    let release_gate = value.and_then(|value| value.get("release_gate"));
    let action_map = value.and_then(|value| value.get("action_map"));
    let recovery_controls = value.and_then(|value| value.get("recovery_controls"));
    let recovery_counts = recovery_control_counts(recovery_controls, action_map);
    let freshness_policy = value.and_then(|value| value.get("freshness_policy"));
    let status = value
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_import_summary".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let next_action = release_gate
        .and_then(|value| string_field(value, &["next_action"]))
        .or_else(|| action_map.and_then(|value| string_field(value, &["next_action"])))
        .or_else(|| recovery_controls.and_then(|value| string_field(value, &["next_action"])))
        .or_else(|| value.and_then(|value| string_field(value, &["next_action"])))
        .unwrap_or_else(|| "dx agents import-summary --json".to_string());

    DxAgentImportSummary {
        present: value.is_some(),
        status,
        operator_summary: value
            .and_then(|value| string_field(value, &["operator_summary"]))
            .unwrap_or_default(),
        release_gate_status: release_gate
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        release_gate_warning_count: release_gate
            .and_then(|value| usize_field(value, &["warning_count"]))
            .unwrap_or_default(),
        release_gate_failed_count: release_gate
            .and_then(|value| usize_field(value, &["failed_count"]))
            .unwrap_or_default(),
        action_map_status: action_map
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        no_command_fanout: value
            .and_then(|value| bool_field(value, &["no_command_fanout"]))
            .or_else(|| action_map.and_then(|value| bool_field(value, &["no_command_fanout"])))
            .or_else(|| {
                recovery_controls.and_then(|value| bool_field(value, &["no_command_fanout"]))
            })
            .unwrap_or(false),
        recovery_controls_status: recovery_controls
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_render_first: recovery_controls
            .and_then(|value| string_field(value, &["render_first"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_counts,
        recovery_states: recovery_controls
            .map(|value| string_array_field(value, &["states"]))
            .unwrap_or_default(),
        recovery_fixture_count: recovery_controls
            .and_then(|value| usize_field(value, &["fixture_count"]))
            .unwrap_or_default(),
        freshness_state: freshness_policy
            .and_then(|value| string_field(value, &["latest_freshness_state"]))
            .unwrap_or_else(|| "unknown".to_string()),
        next_action,
        warning_reasons: release_gate
            .map(|value| string_array_field(value, &["warning_reasons"]))
            .unwrap_or_default(),
        blocking_reasons: release_gate
            .map(|value| string_array_field(value, &["blocking_reasons"]))
            .unwrap_or_default(),
        recovery_commands: value
            .map(|value| string_values_field(value, &["recovery_commands"]))
            .unwrap_or_default(),
    }
}

fn release_gate(value: Option<&Value>, root_exists: bool) -> DxAgentReleaseGateSummary {
    let recovery_controls = value.and_then(|value| value.get("recovery_controls"));
    let recovery_counts = recovery_control_counts(recovery_controls, None);
    let status = value
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if root_exists {
                "waiting_for_release_gate".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let next_action = value
        .and_then(|value| string_field(value, &["next_action"]))
        .or_else(|| recovery_controls.and_then(|value| string_field(value, &["next_action"])))
        .unwrap_or_else(|| "dx agents release-gate --json".to_string());

    DxAgentReleaseGateSummary {
        present: value.is_some(),
        status,
        operator_summary: value
            .and_then(|value| string_field(value, &["operator_summary"]))
            .unwrap_or_default(),
        acceptance_count: value
            .and_then(|value| usize_field(value, &["acceptance_count"]))
            .unwrap_or_default(),
        passed_count: value
            .and_then(|value| usize_field(value, &["passed_count"]))
            .unwrap_or_default(),
        warning_count: value
            .and_then(|value| usize_field(value, &["warning_count"]))
            .unwrap_or_default(),
        failed_count: value
            .and_then(|value| usize_field(value, &["failed_count"]))
            .unwrap_or_default(),
        packet_count: value
            .and_then(|value| usize_field(value, &["packet_count"]))
            .unwrap_or_default(),
        fixture_family_count: value
            .and_then(|value| usize_field(value, &["fixture_family_count"]))
            .unwrap_or_default(),
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        retained_run_overflow_count: value
            .and_then(|value| usize_field(value, &["retained_run_overflow_count"]))
            .unwrap_or_default(),
        import_manifest_status: value
            .and_then(|value| string_field(value, &["import_manifest_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        smoke_status: value
            .and_then(|value| string_field(value, &["smoke_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        receipt_inbox_status: value
            .and_then(|value| string_field(value, &["receipt_inbox_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        retention_preview_status: value
            .and_then(|value| string_field(value, &["retention_preview_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        action_map_status: value
            .and_then(|value| string_field(value, &["action_map_status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        no_command_fanout: value
            .and_then(|value| bool_field(value, &["no_command_fanout"]))
            .or_else(|| {
                recovery_controls.and_then(|value| bool_field(value, &["no_command_fanout"]))
            })
            .unwrap_or(false),
        recovery_controls_status: recovery_controls
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_render_first: recovery_controls
            .and_then(|value| string_field(value, &["render_first"]))
            .unwrap_or_else(|| "unknown".to_string()),
        recovery_counts,
        recovery_fixture_count: recovery_controls
            .and_then(|value| usize_field(value, &["fixture_count"]))
            .unwrap_or_default(),
        next_action,
        warning_reasons: value
            .map(|value| string_array_field(value, &["warning_reasons"]))
            .unwrap_or_default(),
        blocking_reasons: value
            .map(|value| string_array_field(value, &["blocking_reasons"]))
            .unwrap_or_default(),
        acceptance_rows: release_gate_acceptance_rows(value),
    }
}

fn recovery_control_counts(
    recovery_controls: Option<&Value>,
    fallback_action_map: Option<&Value>,
) -> DxAgentRecoveryControlCounts {
    DxAgentRecoveryControlCounts {
        required_intent_count: recovery_controls
            .and_then(|value| usize_field(value, &["required_intent_count"]))
            .or_else(|| {
                fallback_action_map.and_then(|value| usize_field(value, &["required_intent_count"]))
            })
            .unwrap_or_default(),
        action_count: recovery_controls
            .and_then(|value| usize_field(value, &["action_count"]))
            .or_else(|| fallback_action_map.and_then(|value| usize_field(value, &["action_count"])))
            .unwrap_or_default(),
        check_count: recovery_controls
            .and_then(|value| usize_field(value, &["check_count"]))
            .or_else(|| fallback_action_map.and_then(|value| usize_field(value, &["check_count"])))
            .unwrap_or_default(),
    }
}

fn release_gate_acceptance_rows(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|value| array_field(value, &["acceptance"]))
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let label = string_field(row, &["label"])?;
                    let status = string_field(row, &["status"]).unwrap_or_else(|| "unknown".into());
                    Some(format!("{label}: {status}"))
                })
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

fn receipt_inbox(value: Option<&Value>, root_exists: bool) -> DxAgentReceiptInboxSummary {
    let receipt_dir_present = value.and_then(|value| bool_field(value, &["receipt_dir_present"]));
    let status = value
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if receipt_dir_present == Some(false) || !root_exists {
                "missing_config".to_string()
            } else {
                "waiting_for_receipt_inbox".to_string()
            }
        });
    let status = if receipt_dir_present == Some(false) {
        "missing_config".to_string()
    } else {
        status
    };

    DxAgentReceiptInboxSummary {
        present: value.is_some(),
        status,
        receipt_dir_present,
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        latest_count: value
            .and_then(|value| usize_field(value, &["latest_count"]))
            .unwrap_or_default(),
        malformed_count: value
            .and_then(|value| usize_field(value, &["malformed_count"]))
            .unwrap_or_default(),
        missing_latest_count: value
            .and_then(|value| usize_field(value, &["missing_latest_count"]))
            .unwrap_or_default(),
        stale_count: value
            .and_then(|value| usize_field(value, &["stale_count"]))
            .unwrap_or_default(),
        expired_count: value
            .and_then(|value| usize_field(value, &["expired_count"]))
            .unwrap_or_default(),
        last_error: value.and_then(|value| safe_string_field(value, &["last_error"])),
        next_action: value
            .and_then(|value| safe_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents receipts --json".to_string()),
    }
}

fn action_error(value: Option<&Value>) -> DxAgentActionErrorSummary {
    let redaction = value.and_then(|value| value.get("redaction"));
    let exports_secret_values = redaction
        .and_then(|value| bool_field(value, &["exports_secret_values"]))
        .unwrap_or(value.is_some());
    let exports_provider_credentials = redaction
        .and_then(|value| bool_field(value, &["exports_provider_credentials"]))
        .unwrap_or(value.is_some());
    let exports_receipt_bodies = redaction
        .and_then(|value| bool_field(value, &["exports_receipt_bodies"]))
        .unwrap_or(value.is_some());
    let redaction_requires_review =
        exports_secret_values || exports_provider_credentials || exports_receipt_bodies;
    let redaction_summary = if value.is_none() {
        "No failed DX Agents action".to_string()
    } else if redaction_requires_review {
        "Action-error receipt redaction requires review".to_string()
    } else {
        "Action-error receipt is redacted metadata only".to_string()
    };

    DxAgentActionErrorSummary {
        present: value.is_some(),
        status: value
            .and_then(|value| safe_string_field(value, &["status"]))
            .unwrap_or_else(|| "ready".to_string()),
        command: value
            .and_then(|value| safe_string_field(value, &["command"]))
            .unwrap_or_default(),
        error: value.and_then(|value| safe_string_field(value, &["error"])),
        generated_at: value
            .and_then(|value| safe_string_field(value, &["generated_at"]))
            .unwrap_or_default(),
        next_action: value
            .and_then(|value| safe_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents status --json".to_string()),
        redaction_summary,
        redaction_requires_review,
    }
}

fn receipt_index_summary(value: Option<&Value>, root_exists: bool) -> DxAgentReceiptIndexSummary {
    let receipt_root_present = value.and_then(|value| bool_field(value, &["receipt_root_present"]));
    let status = value
        .and_then(|value| string_field(value, &["status"]))
        .unwrap_or_else(|| {
            if receipt_root_present == Some(false) {
                "missing_config".to_string()
            } else if root_exists {
                "waiting_for_receipts_list".to_string()
            } else {
                "missing_receipt_root".to_string()
            }
        });
    let status = if receipt_root_present == Some(false) {
        "missing_config".to_string()
    } else {
        status
    };

    DxAgentReceiptIndexSummary {
        present: value.is_some(),
        status,
        receipt_root_present,
        receipt_count: value
            .and_then(|value| usize_field(value, &["receipt_count"]))
            .unwrap_or_default(),
        returned_receipt_count: value
            .and_then(|value| usize_field(value, &["returned_receipt_count"]))
            .unwrap_or_default(),
        active_task_count: value
            .and_then(|value| usize_field(value, &["active_task_count"]))
            .unwrap_or_default(),
        latest_receipt_path: value.and_then(|value| {
            safe_string_field(value, &["latest_receipt_path"])
                .filter(|path| !path.trim().is_empty())
        }),
        last_error: value.and_then(|value| safe_string_field(value, &["last_error"])),
        next_action: value
            .and_then(|value| safe_string_field(value, &["next_action"]))
            .unwrap_or_else(|| "dx agents receipts list --json".to_string()),
    }
}

fn receipts(value: &Value) -> Vec<DxAgentReceipt> {
    array_field(value, &["receipts"])
        .map(|receipts| receipts.iter().take(12).filter_map(receipt_row).collect())
        .unwrap_or_default()
}

fn receipt_row(value: &Value) -> Option<DxAgentReceipt> {
    let safe_to_render = bool_field(value, &["safe_to_render"]).unwrap_or(false);
    let metadata_redacted = bool_field(value, &["metadata_redacted"]).unwrap_or(false);
    let command = safe_string_field(value, &["command"]).unwrap_or_default();
    let task_id = safe_string_field(value, &["task_id"]).unwrap_or_default();
    let last_error = safe_string_field(value, &["last_error"]);
    let next_action = safe_string_field(value, &["next_action"]).unwrap_or_default();

    Some(DxAgentReceipt {
        id: safe_string_field(value, &["id"])?,
        kind: safe_string_field(value, &["kind"]).unwrap_or_else(|| "receipt".to_string()),
        schema_version: safe_string_field(value, &["schema_version"]).unwrap_or_default(),
        command: if safe_to_render {
            command
        } else {
            String::new()
        },
        generated_at: safe_string_field(value, &["generated_at"]).unwrap_or_default(),
        task_id: if safe_to_render {
            task_id
        } else {
            String::new()
        },
        task_state: if safe_to_render {
            safe_string_field(value, &["task_state"]).unwrap_or_default()
        } else {
            String::new()
        },
        status: safe_string_field(value, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        active_task: bool_field(value, &["active_task"]).unwrap_or(false),
        safe_to_render,
        metadata_redacted,
        receipt_path: safe_string_field(value, &["receipt_path"]).unwrap_or_default(),
        size_bytes: usize_field(value, &["size_bytes"]).unwrap_or_default(),
        modified_at: safe_string_field(value, &["modified_at"]).unwrap_or_default(),
        last_error,
        next_action,
        provider_status: safe_string_field(value, &["provider_status"]),
        model_status: safe_string_field(value, &["model_status"]),
        duration_state: safe_string_field(value, &["duration_state"]),
        retry_supported: bool_field(value, &["retry_supported"]),
        cancel_supported: bool_field(value, &["cancel_supported"]),
        social_connected: usize_field(value, &["social_connected"]),
        social_needs_auth: usize_field(value, &["social_needs_auth"]),
        automation_enabled: usize_field(value, &["automation_enabled"]),
        automation_warning: usize_field(value, &["automation_warning"]),
    })
}

fn read_json(path: &Path) -> Option<Value> {
    let metadata = path.metadata().ok()?;
    if metadata.len() > MAX_RECEIPT_BYTES {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let mut source = String::new();
    file.read_to_string(&mut source).ok()?;
    serde_json::from_str(&source).ok()
}

fn read_first_json(root: &Path, names: &[&str]) -> Option<Value> {
    names.iter().find_map(|name| read_json(&root.join(name)))
}

fn latest_receipts(root: &Path, root_exists: bool) -> Vec<String> {
    if !root_exists {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut receipts = entries
        .flatten()
        .take(64)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return None;
            }
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let label = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some((modified, label))
        })
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    receipts
        .into_iter()
        .take(5)
        .map(|(_, label)| label)
        .collect()
}

fn dx_home_from_receipt_root(receipt_root: &Path) -> Option<PathBuf> {
    receipt_root
        .ancestors()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(".dx"))
        .and_then(Path::parent)
        .map(Path::to_path_buf)
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

fn redact_action_scalar(value: &str) -> String {
    if is_secret_like_arg(value) {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
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

fn string_values_field(value: &Value, path: &[&str]) -> Vec<String> {
    value_at(value, path)
        .and_then(|value| value.as_object())
        .map(|values| {
            values
                .values()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .take(8)
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

fn is_secret_like_arg(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    DX_AGENT_SECRET_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

const DX_AGENT_SECRET_MARKERS: &[&str] = &[
    "sk-",
    "secret",
    "token",
    "password",
    "passwd",
    "cookie",
    "authorization",
    "bearer ",
    "api_key",
    "apikey",
    "provider_key",
    "access_key",
    "access_token",
    "refresh_token",
    "private-token",
    "xoxb-",
    "xoxp-",
];

fn public_command_for_runtime(command: &str) -> String {
    command
        .strip_prefix("dx-agents agents ")
        .map(|args| format!("dx agents {args}"))
        .or_else(|| {
            command
                .strip_prefix("dx-agents providers ")
                .map(|args| format!("dx agents providers {args}"))
        })
        .or_else(|| {
            command
                .strip_prefix("dx-agents models ")
                .map(|args| format!("dx agents models {args}"))
        })
        .unwrap_or_else(|| command.to_string())
}

fn is_public_dx_agents_command(command: &str) -> bool {
    command.starts_with("dx agents ")
}

fn is_dx_agents_command(command: &str, args: &str) -> bool {
    command == format!("dx-agents agents {args}") || command == format!("dx agents {args}")
}

fn is_safe_platform_arg(platform: &str) -> bool {
    !platform.trim().is_empty()
        && platform.len() <= 64
        && !is_secret_like_arg(platform)
        && platform
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn bridge_command_label(cli_path: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(cli_path.to_string());
    parts.extend(args.iter().cloned());
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{action_error, import_summary, is_secret_like_arg, release_gate};
    use serde_json::json;

    #[test]
    fn dx_agent_secret_marker_guard_covers_bridge_receipt_scalars() {
        for value in [
            "sk-should-not-render",
            "provider_key",
            "bearer should-not-render",
            "authorization header",
            "private-token-value",
            "refresh_token",
            "password",
        ] {
            assert!(is_secret_like_arg(value), "{value} should be secret-like");
        }

        assert!(!is_secret_like_arg("telegram"));
        assert!(!is_secret_like_arg("dx agents status --json"));
    }

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
