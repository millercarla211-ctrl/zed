use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use memmap2::Mmap;
use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize, access, deserialize,
    rancor::Error as RkyvError, to_bytes,
};
use serde::Serialize;
use serde_json::Value;

use crate::storage::FlowPackStore;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct SerializedPromptEnvelope {
    pub format: String,
    pub kind: String,
    pub toon: String,
    pub input_sha256: String,
    pub updated_at_unix_ms: u64,
    pub metadata: Vec<(String, String)>,
}

pub struct DxSerializer;

impl DxSerializer {
    pub fn encode_json(kind: &str, value: &Value) -> Result<SerializedPromptEnvelope> {
        let input_bytes = serde_json::to_vec(value)?;
        let toon = ::serializer::encode_default(value)
            .map_err(|error| anyhow::anyhow!("Failed to encode TOON payload: {}", error))?;

        Ok(SerializedPromptEnvelope {
            format: "dx-serializer/toon".to_string(),
            kind: kind.to_string(),
            toon,
            input_sha256: FlowPackStore::sha256_bytes(&input_bytes),
            updated_at_unix_ms: now_unix_ms(),
            metadata: vec![
                ("transport".to_string(), "toon".to_string()),
                ("index".to_string(), "rkyv".to_string()),
                ("cache".to_string(), "memmap2".to_string()),
            ],
        })
    }

    pub fn encode_struct<T: Serialize>(kind: &str, value: &T) -> Result<SerializedPromptEnvelope> {
        let json = serde_json::to_value(value)?;
        Self::encode_json(kind, &json)
    }

    pub fn decode_json(envelope: &SerializedPromptEnvelope) -> Result<Value> {
        ::serializer::decode_default(&envelope.toon)
            .map_err(|error| anyhow::anyhow!("Failed to decode TOON payload: {}", error))
    }

    pub fn write_archive(path: &Path, envelope: &SerializedPromptEnvelope) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let bytes = to_bytes::<RkyvError>(envelope)?;
        fs::write(path, bytes.as_ref())?;
        Ok(())
    }

    pub fn read_archive(path: &Path) -> Result<SerializedPromptEnvelope> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }.context("Failed to mmap serialized prompt")?;
        let archived = access::<ArchivedSerializedPromptEnvelope, RkyvError>(&mmap[..])?;
        Ok(deserialize::<SerializedPromptEnvelope, RkyvError>(
            archived,
        )?)
    }

    pub fn archive_path(root: &Path, key: &str) -> PathBuf {
        root.join(format!("{key}.prompt.rkyv"))
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_structured_prompt_through_toon_and_rkyv() {
        let payload = serde_json::json!({
            "system": "You are Flow",
            "tools": [
                {"name": "search", "enabled": true},
                {"name": "run_local_model", "enabled": true}
            ]
        });

        let envelope = DxSerializer::encode_json("tool-schema", &payload).unwrap();
        let decoded = DxSerializer::decode_json(&envelope).unwrap();
        assert_eq!(decoded, payload);
    }
}
