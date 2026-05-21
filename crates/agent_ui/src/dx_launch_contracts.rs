use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DX_LAUNCH_EXAMPLES_ROOT: &str = r"G:\Dx\cli\fixtures\launch-examples";
const IMPORT_MANIFEST_FILE: &str = "import-manifest.json";
const HANDOFF_FILE: &str = "handoff.json";
const IMPORT_MANIFEST_SCHEMA: &str = "dx.launch.import_manifest.v1";
const HANDOFF_SCHEMA: &str = "dx.launch.handoff.v1";
const DX_LAUNCH_IMPORT_MANIFEST_COMMAND: &str = "dx launch import-manifest --json";
const DX_LAUNCH_HANDOFF_COMMAND: &str = "dx launch handoff --json";
const LAUNCH_CONTRACT_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_PACKET_BYTES: u64 = 256 * 1024;

#[derive(Clone)]
pub(crate) struct DxLaunchContractSnapshot {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub handoff_path: PathBuf,
    pub manifest_present: bool,
    pub handoff_present: bool,
    pub status: String,
    pub operator_summary: String,
    pub packet_count: usize,
    pub fixture_family_count: usize,
    pub command_count: usize,
    pub action_count: usize,
    pub metadata_only_count: usize,
    pub command_fanout_count: usize,
    pub confirmation_action_count: usize,
    pub no_command_fanout: bool,
    pub startup_commands: Vec<String>,
    pub detail_commands: Vec<String>,
    pub diagnostics_commands: Vec<String>,
    pub first_packets: Vec<String>,
    pub first_action: Option<String>,
    pub refresh_command: Option<String>,
    pub cached_receipt_path: Option<String>,
    pub last_error: Option<String>,
    pub next_action: String,
    pub redaction_requires_review: bool,
}

static LAUNCH_CONTRACT_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchContractSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_contract_snapshot() -> DxLaunchContractSnapshot {
    let cache = LAUNCH_CONTRACT_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= LAUNCH_CONTRACT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_launch_contracts();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_launch_contracts()
}

fn scan_launch_contracts() -> DxLaunchContractSnapshot {
    let root = PathBuf::from(DX_LAUNCH_EXAMPLES_ROOT);
    let manifest_path = root.join(IMPORT_MANIFEST_FILE);
    let handoff_path = root.join(HANDOFF_FILE);
    let manifest_present = manifest_path.is_file();
    let handoff_present = handoff_path.is_file();

    let manifest = read_json_packet(&manifest_path);
    let handoff = read_json_packet(&handoff_path);
    let manifest_ref = manifest.as_ref().ok();
    let handoff_ref = handoff.as_ref().ok();
    let mut errors = Vec::new();

    if !manifest_present {
        errors.push(format!("Missing {}", manifest_path.display()));
    } else if let Err(error) = manifest.as_ref() {
        errors.push(error.clone());
    }

    if !handoff_present {
        errors.push(format!("Missing {}", handoff_path.display()));
    } else if let Err(error) = handoff.as_ref() {
        errors.push(error.clone());
    }

    if manifest_ref.and_then(|value| string_field(value, "schema_version"))
        != Some(IMPORT_MANIFEST_SCHEMA)
    {
        errors.push("Launch import manifest schema is missing or unexpected.".to_string());
    }

    if handoff_ref.and_then(|value| string_field(value, "schema_version")) != Some(HANDOFF_SCHEMA) {
        errors.push("Launch handoff schema is missing or unexpected.".to_string());
    }

    let packet_count = manifest_ref
        .and_then(|value| usize_field(value, "packet_count"))
        .or_else(|| manifest_ref.map(|value| array_len(value, "packets")))
        .unwrap_or_default();
    let fixture_family_count = manifest_ref
        .and_then(|value| usize_field(value, "fixture_family_count"))
        .or_else(|| manifest_ref.map(|value| array_len(value, "fixture_families")))
        .unwrap_or_default();
    let command_count = handoff_ref
        .and_then(|value| value.pointer("/schemas/command_count"))
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
        .unwrap_or_default();
    let action_count = handoff_ref
        .and_then(|value| value.pointer("/action_map/action_count"))
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
        .unwrap_or_default();
    let packets = manifest_ref
        .and_then(|value| value.get("packets"))
        .and_then(Value::as_array);
    let metadata_only_count = packets
        .map(|packets| {
            packets
                .iter()
                .filter(|packet| bool_field(packet, "metadata_only"))
                .count()
        })
        .unwrap_or_default();
    let packet_fanout_count = packets
        .map(|packets| {
            packets
                .iter()
                .filter(|packet| bool_field(packet, "command_fanout"))
                .count()
        })
        .unwrap_or_default();
    let first_packets = packets
        .map(|packets| {
            packets
                .iter()
                .take(4)
                .filter_map(|packet| string_field(packet, "command").map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let actions = handoff_ref
        .and_then(|value| value.pointer("/action_map/actions"))
        .and_then(Value::as_array);
    let confirmation_action_count = actions
        .map(|actions| {
            actions
                .iter()
                .filter(|action| bool_field(action, "confirmation_required"))
                .count()
        })
        .unwrap_or_default();
    let action_fanout_count = actions
        .map(|actions| {
            actions
                .iter()
                .filter(|action| bool_field(action, "command_fanout"))
                .count()
        })
        .unwrap_or_default();
    let command_fanout_count = packet_fanout_count + action_fanout_count;
    let first_action = actions
        .and_then(|actions| actions.first())
        .and_then(|action| string_field(action, "command"))
        .map(ToString::to_string);
    let no_command_fanout = handoff_ref
        .and_then(|value| value.get("no_command_fanout"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && command_fanout_count == 0;
    let redaction_requires_review = manifest_ref.is_some_and(redaction_requires_review)
        || handoff_ref.is_some_and(redaction_requires_review);
    let startup_commands = pointer_string_array(handoff_ref, "/polling/startup_commands");
    let detail_commands = pointer_string_array(handoff_ref, "/polling/detail_commands");
    let diagnostics_commands = pointer_string_array(handoff_ref, "/polling/diagnostics_commands");
    let refresh_command =
        pointer_string(handoff_ref, "/polling/foreground_refresh_command").map(ToString::to_string);
    let cached_receipt_path =
        pointer_string(handoff_ref, "/polling/cached_receipt_path").map(ToString::to_string);
    let last_error = errors.first().cloned();
    let status = if !manifest_present || !handoff_present {
        "missing"
    } else if !errors.is_empty() || redaction_requires_review || !no_command_fanout {
        "warning"
    } else {
        manifest_ref
            .and_then(|value| string_field(value, "status"))
            .unwrap_or("ready")
    };
    let operator_summary = manifest_ref
        .and_then(|value| string_field(value, "operator_summary"))
        .or_else(|| handoff_ref.and_then(|value| string_field(value, "operator_summary")))
        .unwrap_or("Launch handoff packets are not available.")
        .to_string();
    let next_action = if !errors.is_empty() {
        DX_LAUNCH_IMPORT_MANIFEST_COMMAND
    } else {
        manifest_ref
            .and_then(|value| string_field(value, "next_action"))
            .or_else(|| handoff_ref.and_then(|value| string_field(value, "next_action")))
            .unwrap_or(DX_LAUNCH_HANDOFF_COMMAND)
    };

    DxLaunchContractSnapshot {
        root,
        manifest_path,
        handoff_path,
        manifest_present,
        handoff_present,
        status: status.to_string(),
        operator_summary,
        packet_count,
        fixture_family_count,
        command_count,
        action_count,
        metadata_only_count,
        command_fanout_count,
        confirmation_action_count,
        no_command_fanout,
        startup_commands,
        detail_commands,
        diagnostics_commands,
        first_packets,
        first_action,
        refresh_command,
        cached_receipt_path,
        last_error,
        next_action: next_action.to_string(),
        redaction_requires_review,
    }
}

fn read_json_packet(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect launch contract packet: {error}"))?;
    if metadata.len() > MAX_PACKET_BYTES {
        return Err(format!(
            "Launch contract packet is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read launch contract packet: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse launch contract packet: {error}"))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn usize_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn array_len(value: &Value, field: &str) -> usize {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn pointer_string<'a>(value: Option<&'a Value>, pointer: &str) -> Option<&'a str> {
    value
        .and_then(|value| value.pointer(pointer))
        .and_then(Value::as_str)
}

fn pointer_string_array(value: Option<&Value>, pointer: &str) -> Vec<String> {
    value
        .and_then(|value| value.pointer(pointer))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(8)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn redaction_requires_review(value: &Value) -> bool {
    let Some(redaction) = value.get("redaction") else {
        return true;
    };

    [
        "exports_source_file_contents",
        "exports_source_file_paths",
        "exports_secret_values",
        "exports_receipt_bodies",
        "exports_prompts",
        "exports_transcripts",
        "exports_command_payloads",
    ]
    .into_iter()
    .any(|field| {
        redaction
            .get(field)
            .and_then(Value::as_bool)
            .unwrap_or(true)
    })
}
