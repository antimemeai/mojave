use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AuditError;

#[derive(Debug)]
pub struct BlobStore {
    blob_dir: PathBuf,
}

impl BlobStore {
    pub fn new(blob_dir: PathBuf) -> Self {
        Self { blob_dir }
    }

    pub fn store(
        &self,
        data: &[u8],
        content_type: &str,
    ) -> Result<audit_events::BlobRef, AuditError> {
        std::fs::create_dir_all(&self.blob_dir)
            .map_err(|e| AuditError::BlobStore(format!("cannot create blob dir: {e}")))?;

        let hash: [u8; 32] = Sha256::digest(data).into();
        let hex = hex_encode(&hash);
        let blob_path = self.blob_dir.join(&hex);

        if !blob_path.exists() {
            std::fs::write(&blob_path, data)
                .map_err(|e| AuditError::BlobStore(format!("cannot write blob {hex}: {e}")))?;
        }

        Ok(audit_events::BlobRef {
            hash,
            location: audit_events::BlobLocation::File { path: blob_path },
            size_bytes: data.len() as u64,
            content_type: content_type.into(),
        })
    }

    pub fn blob_dir(&self) -> &Path {
        &self.blob_dir
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}
