use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::App;
use serde_json::Value;
use settings::SettingsStore;

const DEFAULT_DX_AGENTS_CLI: &str = "dx-agents";
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
    pub automations: Vec<DxAgentAutomation>,
    pub providers: Vec<DxAgentProvider>,
    pub models: Vec<DxAgentModel>,
    pub catalog: DxAgentCatalogSummary,
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
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxAgentAutomation {
    pub id: String,
    pub status: String,
    pub enabled: bool,
    pub schedule_kind: String,
    pub source: String,
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

pub(crate) fn dx_agent_cli_path(cx: &App) -> String {
    dx_agent_settings(cx).cli_path
}

pub(crate) fn dx_agent_dx_home(cx: &App) -> Option<PathBuf> {
    dx_home_from_receipt_root(&dx_agent_settings(cx).receipt_root)
}

pub(crate) fn dx_agent_cli_actions_allowed(cx: &App) -> bool {
    let settings = dx_agent_settings(cx);
    settings.enabled && settings.cli_actions_allowed
}

pub(crate) fn run_dx_agent_command(
    cli_path: String,
    args: Vec<String>,
    dx_home: Option<PathBuf>,
) -> Result<()> {
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

    Ok(())
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
            .unwrap_or_else(|| DEFAULT_DX_AGENTS_CLI.to_string()),
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
    let automation_value = read_json(&settings.receipt_root.join("automate-list-latest.json"));
    let provider_value = read_json(&settings.receipt_root.join("providers-list-latest.json"));
    let model_value = read_json(&settings.receipt_root.join("models-list-latest.json"));

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
    let active_task_count = status_value
        .as_ref()
        .and_then(|value| usize_field(value, &["active_task_count"]))
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
        latest_receipts: latest_receipts(&settings.receipt_root, root_exists),
        show_managed_providers: settings.show_managed_providers,
        show_in_agent_rail: settings.show_in_agent_rail,
    }
}

fn connected_accounts_summary(value: &Value) -> DxConnectedAccountsSummary {
    DxConnectedAccountsSummary {
        supported: usize_field(value, &["supported"]).unwrap_or_default(),
        configured: usize_field(value, &["configured"]).unwrap_or_default(),
        connected: usize_field(value, &["connected"]).unwrap_or_default(),
        needs_connection: usize_field(value, &["needs_connection"]).unwrap_or_default(),
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
                    next_action: string_field(account, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
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
                    next_action: string_field(automation, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
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
            .unwrap_or_else(|| "dx-agents providers catalog regenerate --json".to_string()),
        path,
    }
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

fn is_secret_like_arg(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("password")
        || lower.contains("cookie")
}

fn bridge_command_label(cli_path: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(cli_path.to_string());
    parts.extend(args.iter().cloned());
    parts.join(" ")
}
