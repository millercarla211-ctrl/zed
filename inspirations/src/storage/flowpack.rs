use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use memmap2::Mmap;
use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize, access, deserialize,
    rancor::Error as RkyvError, to_bytes,
};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use sha2::{Digest, Sha256};

use crate::runtime::{ArtifactBundle, BenchmarkRecord, DeviceProfile};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct PromptCacheKey {
    pub model_id: String,
    pub tokenizer_hash: String,
    pub system_prompt_hash: String,
    pub tool_schema_hash: String,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct PromptCacheEntry {
    pub key: PromptCacheKey,
    pub prompt_hash: String,
    pub token_count: usize,
    pub tokens: Vec<u32>,
    pub updated_at_unix_ms: u64,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct PromptCacheIndex {
    pub entries: Vec<PromptCacheEntry>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct FlowPackManifest {
    pub version: u32,
    pub created_at_unix_ms: u64,
    pub device: DeviceProfile,
    pub artifact: ArtifactBundle,
    pub benchmarks: Vec<BenchmarkRecord>,
}

pub struct FlowPackStore;

impl FlowPackStore {
    pub fn write_flowpack(
        root: &Path,
        device: &DeviceProfile,
        artifact: &ArtifactBundle,
        benchmarks: &[BenchmarkRecord],
    ) -> Result<()> {
        fs::create_dir_all(root)?;

        let manifest = FlowPackManifest {
            version: 1,
            created_at_unix_ms: now_unix_ms(),
            device: device.clone(),
            artifact: artifact.clone(),
            benchmarks: benchmarks.to_vec(),
        };

        let manifest_path = Self::manifest_path(root);
        let index_path = Self::index_path(root);

        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).context("Failed to encode flowpack manifest")?,
        )?;

        let bytes = to_bytes::<RkyvError>(&manifest)?;
        fs::write(index_path, bytes.as_ref())?;

        Ok(())
    }

    pub fn read_flowpack(root: &Path) -> Result<FlowPackManifest> {
        let index_path = Self::index_path(root);
        let file = File::open(&index_path)?;
        let mmap = unsafe { Mmap::map(&file) }.context("Failed to mmap flowpack index")?;
        let archived = access::<ArchivedFlowPackManifest, RkyvError>(&mmap[..])?;
        Ok(deserialize::<FlowPackManifest, RkyvError>(archived)?)
    }

    pub fn write_prompt_cache(root: &Path, cache: &PromptCacheIndex) -> Result<()> {
        fs::create_dir_all(root)?;
        let bytes = to_bytes::<RkyvError>(cache)?;
        fs::write(Self::prompt_cache_path(root), bytes.as_ref())?;
        Ok(())
    }

    pub fn read_prompt_cache(root: &Path) -> Result<PromptCacheIndex> {
        let cache_path = Self::prompt_cache_path(root);
        let file = File::open(&cache_path)?;
        let mmap = unsafe { Mmap::map(&file) }.context("Failed to mmap prompt cache")?;
        let archived = access::<ArchivedPromptCacheIndex, RkyvError>(&mmap[..])?;
        Ok(deserialize::<PromptCacheIndex, RkyvError>(archived)?)
    }

    pub fn manifest_path(root: &Path) -> PathBuf {
        root.join("manifest.json")
    }

    pub fn index_path(root: &Path) -> PathBuf {
        root.join("index.rkyv")
    }

    pub fn prompt_cache_path(root: &Path) -> PathBuf {
        root.join("prompt-cache.rkyv")
    }

    pub fn sha256_file(path: &Path) -> Result<String> {
        let bytes = fs::read(path)?;
        Ok(Self::sha256_bytes(&bytes))
    }

    pub fn sha256_bytes(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        hex::encode(digest)
    }

    pub fn prompt_cache_key(
        model_id: &str,
        tokenizer_hash: &str,
        system_prompt_hash: &str,
        tool_schema_hash: &str,
    ) -> PromptCacheKey {
        PromptCacheKey {
            model_id: model_id.to_string(),
            tokenizer_hash: tokenizer_hash.to_string(),
            system_prompt_hash: system_prompt_hash.to_string(),
            tool_schema_hash: tool_schema_hash.to_string(),
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
