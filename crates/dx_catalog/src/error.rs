use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum DxCatalogError {
    Io(io::Error),
    HeaderTooShort {
        len: usize,
    },
    InvalidMagic {
        found: [u8; 8],
    },
    UnsupportedArtifactVersion {
        version: u16,
    },
    UnsupportedHeaderLength {
        header_len: u16,
    },
    PayloadTooLarge {
        payload_len: u64,
    },
    PayloadTooShort {
        expected_len: usize,
        actual_len: usize,
    },
    EmptyPayload,
    Json(serde_json::Error),
    Serialize(String),
    InvalidCatalog {
        reason: String,
    },
}

impl fmt::Display for DxCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "dx catalog I/O failed: {error}"),
            Self::HeaderTooShort { len } => {
                write!(f, "dx catalog header is too short: {len} bytes")
            }
            Self::InvalidMagic { found } => {
                write!(f, "dx catalog artifact has invalid magic bytes: {found:?}")
            }
            Self::UnsupportedArtifactVersion { version } => {
                write!(f, "dx catalog artifact version {version} is unsupported")
            }
            Self::UnsupportedHeaderLength { header_len } => {
                write!(f, "dx catalog header length {header_len} is unsupported")
            }
            Self::PayloadTooLarge { payload_len } => {
                write!(
                    f,
                    "dx catalog payload is too large for this platform: {payload_len}"
                )
            }
            Self::PayloadTooShort {
                expected_len,
                actual_len,
            } => write!(
                f,
                "dx catalog payload is truncated: expected {expected_len} bytes, got {actual_len}"
            ),
            Self::EmptyPayload => write!(f, "dx catalog archive payload is empty"),
            Self::Json(error) => write!(f, "dx catalog JSON parse failed: {error}"),
            Self::Serialize(error) => write!(f, "dx catalog serialization failed: {error}"),
            Self::InvalidCatalog { reason } => {
                write!(
                    f,
                    "dx catalog validation failed before artifact write: {reason}"
                )
            }
        }
    }
}

impl Error for DxCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for DxCatalogError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DxCatalogError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
