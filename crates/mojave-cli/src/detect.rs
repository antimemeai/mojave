use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Inspect,
    Jsonl,
}

pub fn detect_format(path: &Path) -> Result<InputFormat, DetectError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "jsonl" | "ndjson" => return Ok(InputFormat::Jsonl),
        "json" => {
            return sniff_json_file(path);
        }
        _ => {}
    }

    sniff_json_file(path)
}

pub fn parse_format_flag(flag: &str) -> Result<Option<InputFormat>, DetectError> {
    match flag.to_lowercase().as_str() {
        "auto" => Ok(None),
        "inspect" => Ok(Some(InputFormat::Inspect)),
        "jsonl" => Ok(Some(InputFormat::Jsonl)),
        other => Err(DetectError::UnknownFormat(other.to_string())),
    }
}

fn sniff_json_file(path: &Path) -> Result<InputFormat, DetectError> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| DetectError::IoError(e.to_string()))?;
    let trimmed = contents.trim_start();

    if trimmed.starts_with('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if value.get("eval").is_some() || value.get("results").is_some() {
                return Ok(InputFormat::Inspect);
            }
        }
        return Ok(InputFormat::Jsonl);
    }

    if trimmed
        .lines()
        .all(|l| l.trim_start().starts_with('{') || l.trim().is_empty())
    {
        return Ok(InputFormat::Jsonl);
    }

    Err(DetectError::Unrecognized)
}

#[derive(Debug)]
pub enum DetectError {
    UnknownFormat(String),
    IoError(String),
    Unrecognized,
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::UnknownFormat(s) => write!(f, "unknown format: {s}"),
            DetectError::IoError(s) => write!(f, "I/O error during detection: {s}"),
            DetectError::Unrecognized => write!(f, "could not detect input format"),
        }
    }
}

impl std::error::Error for DetectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn jsonl_extension_detected() {
        let tmp = tempfile::Builder::new()
            .suffix(".jsonl")
            .tempfile()
            .unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn ndjson_extension_detected() {
        let tmp = tempfile::Builder::new()
            .suffix(".ndjson")
            .tempfile()
            .unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn inspect_json_detected_by_eval_key() {
        let mut tmp = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        write!(tmp, r#"{{"eval":{{"task":"t1"}},"results":[]}}"#).unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Inspect);
    }

    #[test]
    fn plain_json_object_treated_as_jsonl() {
        let mut tmp = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        write!(tmp, r#"{{"task_id":"t","score":0.5}}"#).unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), InputFormat::Jsonl);
    }

    #[test]
    fn parse_format_flag_auto() {
        assert_eq!(parse_format_flag("auto").unwrap(), None);
    }

    #[test]
    fn parse_format_flag_explicit() {
        assert_eq!(
            parse_format_flag("inspect").unwrap(),
            Some(InputFormat::Inspect)
        );
        assert_eq!(
            parse_format_flag("jsonl").unwrap(),
            Some(InputFormat::Jsonl)
        );
    }

    #[test]
    fn parse_format_flag_unknown() {
        assert!(parse_format_flag("xml").is_err());
    }
}
