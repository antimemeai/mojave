#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_recover::gc;
use std::fs;

#[test]
fn gc_no_blob_dir_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let result = gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_scanned, 0);
    assert_eq!(result.blobs_deleted, 0);
}

#[test]
fn gc_removes_orphan_blobs() {
    let dir = tempfile::tempdir().unwrap();
    let blob_dir = dir.path().join("blobs");
    fs::create_dir_all(&blob_dir).unwrap();

    fs::write(blob_dir.join("deadbeef".repeat(4)), b"orphan data").unwrap();
    fs::write(blob_dir.join("cafebabe".repeat(4)), b"also orphan").unwrap();

    let result = gc::gc_blobs(dir.path()).unwrap();
    assert_eq!(result.blobs_scanned, 2);
    assert_eq!(result.blobs_deleted, 2);
    assert_eq!(fs::read_dir(&blob_dir).unwrap().count(), 0);
}
