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

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read source audit packet: {error}"))?;
    serde_json::from_str(&contents)
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
