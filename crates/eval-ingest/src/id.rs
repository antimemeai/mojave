use sha2::{Digest, Sha256};
use ulid::Ulid;

/// Produce a deterministic [`Ulid`] by SHA-256 hashing a single string.
///
/// The resulting ULID has the same bit-layout as a normal ULID (128 bits),
/// but the timestamp component is filled from the first 48 bits of the hash
/// and the random component from the remaining 80 bits. This guarantees
/// stable, collision-resistant IDs for the same logical input across runs.
pub fn ulid_from_str(input: &str) -> Ulid {
    let hash = Sha256::digest(input.as_bytes());
    ulid_from_hash_bytes(&hash)
}

/// Produce a deterministic [`Ulid`] by SHA-256 hashing multiple string parts.
///
/// Parts are joined with a `\x00` (null-byte) separator before hashing so that
/// `["a", "b"]` and `["ab"]` produce different IDs.
pub fn ulid_from_parts(parts: &[&str]) -> Ulid {
    let mut hasher = Sha256::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\x00");
        }
        hasher.update(part.as_bytes());
    }
    let hash = hasher.finalize();
    ulid_from_hash_bytes(&hash)
}

/// Convert the first 16 bytes of a SHA-256 digest to a [`Ulid`].
fn ulid_from_hash_bytes(hash: &[u8]) -> Ulid {
    // ULID is a 128-bit value stored in big-endian order.
    // We take the first 16 bytes of the 32-byte hash.
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    Ulid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn same_input_same_ulid() {
        let a = ulid_from_str("hello");
        let b = ulid_from_str("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn different_input_different_ulid() {
        let a = ulid_from_str("hello");
        let b = ulid_from_str("world");
        assert_ne!(a, b);
    }

    #[test]
    fn parts_null_separator_matters() {
        // ["ab", "c"] vs ["a", "bc"] must differ.
        let a = ulid_from_parts(&["ab", "c"]);
        let b = ulid_from_parts(&["a", "bc"]);
        assert_ne!(a, b);
    }

    #[test]
    fn parts_consistent() {
        let a = ulid_from_parts(&["run123", "sample42", "0", "accuracy"]);
        let b = ulid_from_parts(&["run123", "sample42", "0", "accuracy"]);
        assert_eq!(a, b);
    }
}
