pub(super) const DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM: &str = "fnv1a64";
pub(super) const DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_PREFIX: &str = "fnv1a64:";

pub(super) fn active_source_digest(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_PREFIX}{hash:016x}")
}
