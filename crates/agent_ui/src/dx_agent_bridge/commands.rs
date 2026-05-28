use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow};
use serde_json::{Value, json};

use super::{
    MAX_RECEIPT_BYTES, bridge_command_label, clear_snapshot_cache, is_safe_platform_arg,
    is_secret_like_arg, redact_action_scalar, string_field,
};

const MAX_FAILED_COMMAND_STDERR_BYTES: usize = 2048;
const MAX_FAILED_COMMAND_STDERR_CHARS: usize = 500;

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
        let stderr = failed_command_stderr_display(&output.stderr);
        return Err(anyhow!(
            "`{}` failed: {}",
            bridge_command_label(&cli_path, &args),
            stderr
        ));
    }

    Ok(output)
}

fn failed_command_stderr_display(stderr: &[u8]) -> String {
    let truncated_bytes = stderr.len() > MAX_FAILED_COMMAND_STDERR_BYTES;
    let visible_len = stderr.len().min(MAX_FAILED_COMMAND_STDERR_BYTES);
    let decoded = String::from_utf8_lossy(&stderr[..visible_len]);
    let compact = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated_chars = compact.chars().count() > MAX_FAILED_COMMAND_STDERR_CHARS;

    if !truncated_bytes && !truncated_chars {
        return compact;
    }

    let mut display = compact
        .chars()
        .take(MAX_FAILED_COMMAND_STDERR_CHARS.saturating_sub(3))
        .collect::<String>();
    display.push_str("...");
    display
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
