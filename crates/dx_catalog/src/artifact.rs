use crate::{DxCatalog, DxCatalogError, Result};
use memmap2::{Mmap, MmapOptions};
use rkyv::{
    Deserialize as RkyvDeserialize, Infallible, archived_root,
    ser::{Serializer, serializers::AllocSerializer},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

pub const DX_CATALOG_MAGIC: [u8; 8] = *b"DXCAT001";
pub const DX_CATALOG_ARTIFACT_VERSION: u16 = 1;

const HEADER_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogArtifactHeader {
    pub version: u16,
    pub header_len: u16,
    pub provider_count: u32,
    pub model_count: u32,
    pub source_count: u32,
    pub generated_unix_ms: u64,
    pub payload_len: u64,
}

impl CatalogArtifactHeader {
    pub fn for_catalog(catalog: &DxCatalog, payload_len: usize) -> Result<Self> {
        Ok(Self {
            version: DX_CATALOG_ARTIFACT_VERSION,
            header_len: HEADER_LEN as u16,
            provider_count: catalog.providers.len() as u32,
            model_count: catalog.models.len() as u32,
            source_count: catalog.sources.len() as u32,
            generated_unix_ms: catalog.generated_unix_ms,
            payload_len: payload_len as u64,
        })
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(DxCatalogError::HeaderTooShort { len: bytes.len() });
        }

        let mut found = [0; 8];
        found.copy_from_slice(&bytes[0..8]);
        if found != DX_CATALOG_MAGIC {
            return Err(DxCatalogError::InvalidMagic { found });
        }

        let version = read_u16(bytes, 8);
        if version != DX_CATALOG_ARTIFACT_VERSION {
            return Err(DxCatalogError::UnsupportedArtifactVersion { version });
        }

        let header_len = read_u16(bytes, 10);
        if usize::from(header_len) != HEADER_LEN {
            return Err(DxCatalogError::UnsupportedHeaderLength { header_len });
        }

        let provider_count = read_u32(bytes, 12);
        let model_count = read_u32(bytes, 16);
        let source_count = read_u32(bytes, 20);
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
            provider_count,
            model_count,
            source_count,
            generated_unix_ms,
            payload_len,
        })
    }

    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0; HEADER_LEN];
        bytes[0..8].copy_from_slice(&DX_CATALOG_MAGIC);
        bytes[8..10].copy_from_slice(&self.version.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.header_len.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.provider_count.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.model_count.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.source_count.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.generated_unix_ms.to_le_bytes());
        bytes[32..40].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogArtifactRef<'a> {
    header: CatalogArtifactHeader,
    payload: &'a [u8],
}

impl<'a> CatalogArtifactRef<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = CatalogArtifactHeader::decode(bytes)?;
        let payload_len =
            usize::try_from(header.payload_len).map_err(|_| DxCatalogError::PayloadTooLarge {
                payload_len: header.payload_len,
            })?;
        let payload = &bytes[HEADER_LEN..HEADER_LEN + payload_len];
        Ok(Self { header, payload })
    }

    pub fn header(&self) -> CatalogArtifactHeader {
        self.header
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

#[derive(Debug)]
pub struct MappedCatalogArtifact {
    path: PathBuf,
    mmap: Mmap,
    header: CatalogArtifactHeader,
}

impl MappedCatalogArtifact {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        // SAFETY: The mapping is read-only and the file handle is kept alive until
        // mapping creation succeeds. Callers should store catalog artifacts under
        // DX-managed cache paths and rewrite them atomically.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let artifact = CatalogArtifactRef::parse(&mmap)?;
        Ok(Self {
            path,
            mmap,
            header: artifact.header(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> CatalogArtifactHeader {
        self.header
    }

    pub fn artifact_ref(&self) -> Result<CatalogArtifactRef<'_>> {
        CatalogArtifactRef::parse(&self.mmap)
    }

    pub fn payload(&self) -> Result<&[u8]> {
        Ok(self.artifact_ref()?.payload())
    }

    pub fn read_catalog(&self) -> Result<DxCatalog> {
        deserialize_trusted_catalog_payload(self.payload()?)
    }
}

pub fn serialize_catalog_payload(catalog: &DxCatalog) -> Result<Vec<u8>> {
    let mut serializer = AllocSerializer::<4096>::default();
    serializer
        .serialize_value(catalog)
        .map_err(|error| DxCatalogError::Serialize(format!("{error:?}")))?;
    Ok(serializer.into_serializer().into_inner())
}

pub fn deserialize_trusted_catalog_payload(payload: &[u8]) -> Result<DxCatalog> {
    if payload.is_empty() {
        return Err(DxCatalogError::EmptyPayload);
    }

    // SAFETY: This is for catalog payloads produced by `serialize_catalog_payload`
    // and wrapped by the DX catalog artifact header. Untrusted external catalog
    // bytes must go through a bytecheck-backed validator before this path is used.
    let archived = unsafe { archived_root::<DxCatalog>(payload) };
    let mut deserializer = Infallible;
    let catalog = archived
        .deserialize(&mut deserializer)
        .map_err(|error| match error {})?;
    Ok(catalog)
}

pub fn write_catalog_artifact(path: impl AsRef<Path>, catalog: &DxCatalog) -> Result<()> {
    let payload = serialize_catalog_payload(catalog)?;
    let header = CatalogArtifactHeader::for_catalog(catalog, payload.len())?;
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

pub fn read_catalog_artifact(path: impl AsRef<Path>) -> Result<DxCatalog> {
    MappedCatalogArtifact::open(path)?.read_catalog()
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
