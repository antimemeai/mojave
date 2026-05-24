use std::collections::HashSet;
use std::path::Path;

use audit_chain::seal::SealedAuditEntry;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct GcResult {
    pub blobs_scanned: usize,
    pub blobs_referenced: usize,
    pub blobs_deleted: usize,
}

pub fn collect_referenced_blob_hashes(chain_dir: &Path) -> Result<HashSet<String>, GcError> {
    let mut hashes = HashSet::new();

    for entry in std::fs::read_dir(chain_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)?;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(sealed) = serde_json::from_str::<SealedAuditEntry>(trimmed) {
                if let Some(blob_ref) = &sealed.base.blob_ref {
                    let hex: String =
                        blob_ref
                            .hash
                            .iter()
                            .fold(String::with_capacity(64), |mut s, b| {
                                use std::fmt::Write;
                                let _ = write!(s, "{b:02x}");
                                s
                            });
                    hashes.insert(hex);
                }
            }
        }
    }

    Ok(hashes)
}

pub fn gc_blobs(audit_dir: &Path) -> Result<GcResult, GcError> {
    let blob_dir = audit_dir.join("blobs");
    if !blob_dir.exists() {
        return Ok(GcResult {
            blobs_scanned: 0,
            blobs_referenced: 0,
            blobs_deleted: 0,
        });
    }

    let referenced = collect_referenced_blob_hashes(audit_dir)?;
    let mut scanned = 0usize;
    let mut deleted = 0usize;

    for entry in std::fs::read_dir(&blob_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        scanned += 1;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !referenced.contains(name) {
            std::fs::remove_file(&path)?;
            deleted += 1;
        }
    }

    Ok(GcResult {
        blobs_scanned: scanned,
        blobs_referenced: referenced.len(),
        blobs_deleted: deleted,
    })
}
