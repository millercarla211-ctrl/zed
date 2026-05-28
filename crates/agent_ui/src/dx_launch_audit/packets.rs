use super::packet_fields::string_field;
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_PACKET_BYTES: u64 = 256 * 1024;

pub(super) fn read_checked_packet(path: &Path, expected_schema: &str) -> Result<Value, String> {
    let packet = read_json_packet(path)?;
    let schema = string_field(&packet, "schema_version").unwrap_or("missing");
    if schema != expected_schema {
        return Err(format!(
            "{} uses schema {schema}, expected {expected_schema}",
            path.display()
        ));
    }
    Ok(packet)
}

fn read_json_packet(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect launch audit packet: {error}"))?;
    if metadata.len() > MAX_PACKET_BYTES {
        return Err(format!(
            "Launch audit packet is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut buffer = Vec::new();
    File::open(path)
        .map_err(|error| format!("Unable to read launch audit packet: {error}"))?
        .take(MAX_PACKET_BYTES + 1)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("Unable to read launch audit packet: {error}"))?;
    if buffer.len() as u64 > MAX_PACKET_BYTES {
        return Err(format!(
            "Launch audit packet is too large to render safely: {} bytes",
            buffer.len()
        ));
    }

    serde_json::from_slice(&buffer)
        .map_err(|error| format!("Unable to parse launch audit packet: {error}"))
}
