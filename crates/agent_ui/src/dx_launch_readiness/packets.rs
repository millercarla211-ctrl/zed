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
        .map_err(|error| format!("Unable to inspect launch readiness packet: {error}"))?;
    if metadata.len() > MAX_PACKET_BYTES {
        return Err(format!(
            "Launch readiness packet is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read launch readiness packet: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse launch readiness packet: {error}"))
}
