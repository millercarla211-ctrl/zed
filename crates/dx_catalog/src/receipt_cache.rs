use crate::{DxCatalogError, Result};
use memmap2::{Mmap, MmapOptions};
use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Infallible, Serialize as RkyvSerialize, archived_root,
    ser::{Serializer, serializers::AllocSerializer},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

pub const DX_RECEIPT_CACHE_SCHEMA_VERSION: u16 = 1;
pub const DX_RECEIPT_CACHE_MAGIC: [u8; 8] = *b"DXRCP001";
pub const DX_RECEIPT_CACHE_ARTIFACT_VERSION: u16 = 1;

const HEADER_LEN: usize = 64;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxReceiptCacheManifest {
    pub schema_version: u16,
    pub generated_unix_ms: u64,
    pub source_revision: String,
    pub roots: Vec<DxReceiptCacheRoot>,
    pub entries: Vec<DxReceiptCacheEntry>,
}

impl DxReceiptCacheManifest {
    pub fn empty(source_revision: impl Into<String>) -> Self {
        Self {
            schema_version: DX_RECEIPT_CACHE_SCHEMA_VERSION,
            generated_unix_ms: 0,
            source_revision: source_revision.into(),
            roots: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn root(&self, id: &str) -> Option<&DxReceiptCacheRoot> {
        self.roots.iter().find(|root| root.id == id)
    }

    pub fn entries_for_root(&self, root_id: &str) -> impl Iterator<Item = &DxReceiptCacheEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.root_id == root_id)
    }

    pub fn entries_for_kind(
        &self,
        kind: DxReceiptCacheEntryKind,
    ) -> impl Iterator<Item = &DxReceiptCacheEntry> {
        self.entries.iter().filter(move |entry| entry.kind == kind)
    }

    pub fn latest_entry_for_root(&self, root_id: &str) -> Option<&DxReceiptCacheEntry> {
        self.entries_for_root(root_id)
            .max_by_key(|entry| entry.latest_unix_ms().unwrap_or(0))
    }

    pub fn summary(&self) -> DxReceiptCacheSummary {
        let mut summary = DxReceiptCacheSummary {
            root_count: capped_len(self.roots.len()),
            present_root_count: 0,
            missing_root_count: 0,
            entry_count: capped_len(self.entries.len()),
            malformed_entry_count: 0,
            fresh_entry_count: 0,
            stale_entry_count: 0,
            expired_entry_count: 0,
            unknown_entry_count: 0,
            latest_receipt_unix_ms: None,
        };

        for root in &self.roots {
            if root.present {
                summary.present_root_count = summary.present_root_count.saturating_add(1);
            } else {
                summary.missing_root_count = summary.missing_root_count.saturating_add(1);
            }
            summary.latest_receipt_unix_ms =
                max_optional_unix_ms(summary.latest_receipt_unix_ms, root.latest_unix_ms);
        }

        for entry in &self.entries {
            match entry.freshness {
                DxReceiptCacheFreshness::Fresh => {
                    summary.fresh_entry_count = summary.fresh_entry_count.saturating_add(1)
                }
                DxReceiptCacheFreshness::Stale => {
                    summary.stale_entry_count = summary.stale_entry_count.saturating_add(1)
                }
                DxReceiptCacheFreshness::Expired => {
                    summary.expired_entry_count = summary.expired_entry_count.saturating_add(1)
                }
                DxReceiptCacheFreshness::Malformed => {
                    summary.malformed_entry_count = summary.malformed_entry_count.saturating_add(1)
                }
                DxReceiptCacheFreshness::Unknown => {
                    summary.unknown_entry_count = summary.unknown_entry_count.saturating_add(1)
                }
            }
            summary.latest_receipt_unix_ms =
                max_optional_unix_ms(summary.latest_receipt_unix_ms, entry.latest_unix_ms());
        }

        summary
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxReceiptCacheSummary {
    pub root_count: u32,
    pub present_root_count: u32,
    pub missing_root_count: u32,
    pub entry_count: u32,
    pub malformed_entry_count: u32,
    pub fresh_entry_count: u32,
    pub stale_entry_count: u32,
    pub expired_entry_count: u32,
    pub unknown_entry_count: u32,
    pub latest_receipt_unix_ms: Option<u64>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxReceiptCacheRoot {
    pub id: String,
    pub path: String,
    pub present: bool,
    pub receipt_count: u32,
    pub latest_unix_ms: Option<u64>,
    pub notes: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxReceiptCacheEntry {
    pub id: String,
    pub root_id: String,
    pub kind: DxReceiptCacheEntryKind,
    pub relative_path: String,
    pub schema_version: Option<String>,
    pub status: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub modified_unix_ms: Option<u64>,
    pub size_bytes: u64,
    pub freshness: DxReceiptCacheFreshness,
}

impl DxReceiptCacheEntry {
    pub fn latest_unix_ms(&self) -> Option<u64> {
        max_optional_unix_ms(self.modified_unix_ms, self.generated_unix_ms)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DxReceiptCacheEntryKind {
    Agents,
    Launch,
    Tokens,
    Forge,
    Sources,
    Media,
    Rlm,
    Serializer,
    Deploy,
    RuntimeProof,
    Other,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DxReceiptCacheFreshness {
    Fresh,
    Stale,
    Expired,
    Malformed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptCacheArtifactHeader {
    pub version: u16,
    pub header_len: u16,
    pub root_count: u32,
    pub entry_count: u32,
    pub generated_unix_ms: u64,
    pub payload_len: u64,
}

impl ReceiptCacheArtifactHeader {
    pub fn for_manifest(manifest: &DxReceiptCacheManifest, payload_len: usize) -> Result<Self> {
        Ok(Self {
            version: DX_RECEIPT_CACHE_ARTIFACT_VERSION,
            header_len: HEADER_LEN as u16,
            root_count: manifest.roots.len() as u32,
            entry_count: manifest.entries.len() as u32,
            generated_unix_ms: manifest.generated_unix_ms,
            payload_len: payload_len as u64,
        })
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(DxCatalogError::HeaderTooShort { len: bytes.len() });
        }

        let mut found = [0; 8];
        found.copy_from_slice(&bytes[0..8]);
        if found != DX_RECEIPT_CACHE_MAGIC {
            return Err(DxCatalogError::InvalidMagic { found });
        }

        let version = read_u16(bytes, 8);
        if version != DX_RECEIPT_CACHE_ARTIFACT_VERSION {
            return Err(DxCatalogError::UnsupportedArtifactVersion { version });
        }

        let header_len = read_u16(bytes, 10);
        if usize::from(header_len) != HEADER_LEN {
            return Err(DxCatalogError::UnsupportedHeaderLength { header_len });
        }

        let root_count = read_u32(bytes, 12);
        let entry_count = read_u32(bytes, 16);
        let generated_unix_ms = read_u64(bytes, 24);
        let payload_len = read_u64(bytes, 32);

        let payload_len_usize = usize::try_from(payload_len)
            .map_err(|_| DxCatalogError::PayloadTooLarge { payload_len })?;
        let expected_len = HEADER_LEN + payload_len_usize;
        if bytes.len() < expected_len {
            return Err(DxCatalogError::PayloadTooShort {
                expected_len,
                actual_len: bytes.len(),
            });
        }

        Ok(Self {
            version,
            header_len,
            root_count,
            entry_count,
            generated_unix_ms,
            payload_len,
        })
    }

    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0; HEADER_LEN];
        bytes[0..8].copy_from_slice(&DX_RECEIPT_CACHE_MAGIC);
        bytes[8..10].copy_from_slice(&self.version.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.header_len.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.root_count.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.entry_count.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.generated_unix_ms.to_le_bytes());
        bytes[32..40].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptCacheArtifactRef<'a> {
    header: ReceiptCacheArtifactHeader,
    payload: &'a [u8],
}

impl<'a> ReceiptCacheArtifactRef<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = ReceiptCacheArtifactHeader::decode(bytes)?;
        let payload_len =
            usize::try_from(header.payload_len).map_err(|_| DxCatalogError::PayloadTooLarge {
                payload_len: header.payload_len,
            })?;
        let payload = &bytes[HEADER_LEN..HEADER_LEN + payload_len];
        Ok(Self { header, payload })
    }

    pub fn header(&self) -> ReceiptCacheArtifactHeader {
        self.header
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

#[derive(Debug)]
pub struct MappedReceiptCacheArtifact {
    path: PathBuf,
    mmap: Mmap,
    header: ReceiptCacheArtifactHeader,
}

impl MappedReceiptCacheArtifact {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        // SAFETY: The mapping is read-only and intended for DX-managed cache
        // artifacts that are rewritten atomically by their producer.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let header = {
            let artifact = ReceiptCacheArtifactRef::parse(&mmap)?;
            artifact.header()
        };
        Ok(Self { path, mmap, header })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> ReceiptCacheArtifactHeader {
        self.header
    }

    pub fn artifact_ref(&self) -> Result<ReceiptCacheArtifactRef<'_>> {
        ReceiptCacheArtifactRef::parse(&self.mmap)
    }

    pub fn payload(&self) -> Result<&[u8]> {
        Ok(self.artifact_ref()?.payload())
    }

    pub fn read_manifest(&self) -> Result<DxReceiptCacheManifest> {
        deserialize_trusted_receipt_cache_payload(self.payload()?)
    }
}

pub fn serialize_receipt_cache_payload(manifest: &DxReceiptCacheManifest) -> Result<Vec<u8>> {
    let mut serializer = AllocSerializer::<4096>::default();
    serializer
        .serialize_value(manifest)
        .map_err(|error| DxCatalogError::Serialize(format!("{error:?}")))?;
    Ok(serializer.into_serializer().into_inner().to_vec())
}

pub fn deserialize_trusted_receipt_cache_payload(payload: &[u8]) -> Result<DxReceiptCacheManifest> {
    if payload.is_empty() {
        return Err(DxCatalogError::EmptyPayload);
    }

    // SAFETY: This path is for receipt cache payloads produced by
    // `serialize_receipt_cache_payload` and wrapped in the DX receipt cache
    // artifact header. External bytes must be validated before they use this
    // trusted reader.
    let archived = unsafe { archived_root::<DxReceiptCacheManifest>(payload) };
    let mut deserializer = Infallible;
    let manifest = match archived.deserialize(&mut deserializer) {
        Ok(manifest) => manifest,
        Err(error) => match error {},
    };
    Ok(manifest)
}

pub fn write_receipt_cache_artifact(
    path: impl AsRef<Path>,
    manifest: &DxReceiptCacheManifest,
) -> Result<()> {
    let payload = serialize_receipt_cache_payload(manifest)?;
    let header = ReceiptCacheArtifactHeader::for_manifest(manifest, payload.len())?;
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
    bytes.extend_from_slice(&header.encode());
    bytes.extend_from_slice(&payload);
    fs::write(path, bytes)?;
    Ok(())
}

pub fn read_receipt_cache_artifact(path: impl AsRef<Path>) -> Result<DxReceiptCacheManifest> {
    MappedReceiptCacheArtifact::open(path)?.read_manifest()
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn capped_len(len: usize) -> u32 {
    len.min(u32::MAX as usize) as u32
}

fn max_optional_unix_ms(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}
