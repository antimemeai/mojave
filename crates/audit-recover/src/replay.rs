use audit_chain::seal::{ChainHead, SealedAuditEntry};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReplayError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error at line {line}: {source}")]
    JsonParse {
        line: usize,
        source: serde_json::Error,
    },
}

#[derive(Debug)]
pub struct ReplayResult {
    pub chain_head: Option<ChainHead>,
    pub entry_count: usize,
    pub truncated_lines: usize,
}

pub fn replay_chain_file(path: &std::path::Path) -> Result<ReplayResult, ReplayError> {
    let contents = std::fs::read_to_string(path)?;
    replay_chain_str(&contents)
}

pub fn replay_chain_str(contents: &str) -> Result<ReplayResult, ReplayError> {
    let mut head = None::<ChainHead>;
    let mut count = 0usize;
    let mut truncated = 0usize;

    let lines: Vec<&str> = contents.lines().collect();
    let total_lines = lines.len();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SealedAuditEntry>(trimmed) {
            Ok(entry) => {
                head = Some(ChainHead::resume(entry.entry_hash(), entry.seq() + 1));
                count += 1;
            }
            Err(e) => {
                if i == total_lines - 1 {
                    truncated += 1;
                    eprintln!(
                        "audit-recover: truncated last line {}, skipping (crash recovery)",
                        i + 1
                    );
                } else {
                    return Err(ReplayError::JsonParse {
                        line: i + 1,
                        source: e,
                    });
                }
            }
        }
    }

    Ok(ReplayResult {
        chain_head: head,
        entry_count: count,
        truncated_lines: truncated,
    })
}
