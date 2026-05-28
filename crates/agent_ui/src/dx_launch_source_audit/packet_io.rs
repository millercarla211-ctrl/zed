use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_AUDIT_BYTES: u64 = 512 * 1024;

pub(super) fn read_json_packet(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect source audit packet: {error}"))?;
    if metadata.len() > MAX_AUDIT_BYTES {
        return Err(format!(
            "Source audit packet is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut buffer = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_AUDIT_BYTES + 1).read_to_end(&mut buffer))
        .map_err(|error| format!("Unable to read source audit packet: {error}"))?;
    if buffer.len() as u64 > MAX_AUDIT_BYTES {
        return Err(format!(
            "Source audit packet is too large to render safely: {} bytes",
            buffer.len()
        ));
    }

    serde_json::from_slice(&buffer)
        .map_err(|error| format!("Unable to parse source audit packet: {error}"))
}

pub(super) fn packet_schema(packet: &Value) -> String {
    packet
        .get("schema")
        .or_else(|| packet.get("schema_version"))
        .and_then(Value::as_str)
        .unwrap_or("missing")
        .to_string()
}
