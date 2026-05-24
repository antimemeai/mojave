#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_emit::blob_store::BlobStore;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn store_creates_content_addressed_file() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));
    let data = b"hello blob world";
    let blob_ref = store.store(data, "text/plain").unwrap();

    assert_eq!(blob_ref.size_bytes, data.len() as u64);
    assert_eq!(blob_ref.content_type, "text/plain");

    let expected_hash: [u8; 32] = Sha256::digest(data).into();
    assert_eq!(blob_ref.hash, expected_hash);

    match &blob_ref.location {
        audit_events::BlobLocation::File { path } => {
            assert!(path.exists());
            assert_eq!(std::fs::read(path).unwrap(), data);
        }
    }
}

#[test]
fn store_deduplicates_same_content() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));
    let data = b"dedup test";

    let ref1 = store.store(data, "text/plain").unwrap();
    let ref2 = store.store(data, "text/plain").unwrap();

    assert_eq!(ref1.hash, ref2.hash);
    assert_eq!(
        std::fs::read_dir(dir.path().join("blobs")).unwrap().count(),
        1
    );
}

#[test]
fn store_different_content_creates_different_files() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path().join("blobs"));

    store.store(b"content A", "text/plain").unwrap();
    store.store(b"content B", "text/plain").unwrap();

    assert_eq!(
        std::fs::read_dir(dir.path().join("blobs")).unwrap().count(),
        2
    );
}
