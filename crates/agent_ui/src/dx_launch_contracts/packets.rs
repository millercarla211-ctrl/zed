use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_PACKET_BYTES: u64 = 256 * 1024;

pub(super) fn read_json_packet(path: &Path) -> Result<Value, String> {
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
