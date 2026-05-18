use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CanonicalEncodingError {
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("float rejected at path {path}")]
    FloatRejected { path: String },
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalEncodingError> {
    let json_value = serde_json::to_value(value)?;
    let mut buf = String::new();
    write_value(&json_value, &mut buf, &mut String::new())?;
    Ok(buf.into_bytes())
}

fn write_value(
    value: &serde_json::Value,
    buf: &mut String,
    path: &mut String,
) -> Result<(), CanonicalEncodingError> {
    match value {
        serde_json::Value::Null => {
            buf.push_str("null");
        }
        serde_json::Value::Bool(b) => {
            buf.push_str(if *b { "true" } else { "false" });
        }
        serde_json::Value::Number(n) => {
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                return Err(CanonicalEncodingError::FloatRejected { path: path.clone() });
            }
            let _ = write!(buf, "{n}");
        }
        serde_json::Value::String(s) => {
            write_string(s, buf);
        }
        serde_json::Value::Array(arr) => {
            buf.push('[');
            let base_len = path.len();
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                path.truncate(base_len);
                let _ = write!(path, "/{i}");
                write_value(item, buf, path)?;
            }
            path.truncate(base_len);
            buf.push(']');
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            buf.push('{');
            let base_len = path.len();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                write_string(key, buf);
                buf.push(':');
                path.truncate(base_len);
                let _ = write!(path, "/{key}");
                if let Some(val) = map.get(*key) {
                    write_value(val, buf, path)?;
                }
            }
            path.truncate(base_len);
            buf.push('}');
        }
    }
    Ok(())
}

fn write_string(s: &str, buf: &mut String) {
    buf.push('"');
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\x08' => buf.push_str("\\b"),
            '\x0C' => buf.push_str("\\f"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if c < '\x20' => {
                let _ = write!(buf, "\\u{:04x}", c as u32);
            }
            c => buf.push(c),
        }
    }
    buf.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_object() {
        let v = json!({});
        let out = encode(&v).unwrap();
        assert_eq!(out, b"{}");
    }

    #[test]
    fn empty_array() {
        let v = json!([]);
        let out = encode(&v).unwrap();
        assert_eq!(out, b"[]");
    }

    #[test]
    fn keys_sorted_by_codepoint() {
        let v = json!({"b": 1, "a": 2, "c": 3});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":2,"b":1,"c":3}"#);
    }

    #[test]
    fn nested_keys_sorted() {
        let v = json!({"outer": {"z": 1, "a": 2}});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"outer":{"a":2,"z":1}}"#);
    }

    #[test]
    fn integer_preserved() {
        let v = json!({"n": -42});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("-42"));
    }

    #[test]
    fn u64_max_preserved() {
        let v = json!({"n": u64::MAX});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("18446744073709551615"));
    }

    #[test]
    fn i64_min_preserved() {
        let v = json!({"n": i64::MIN});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("-9223372036854775808"));
    }

    #[test]
    fn float_rejected() {
        let v = json!({"n": 1.5});
        let err = encode(&v).unwrap_err();
        match err {
            CanonicalEncodingError::FloatRejected { path } => assert!(path.contains("n")),
            other => panic!("expected FloatRejected, got {other:?}"),
        }
    }

    #[test]
    fn deeply_nested_float_rejected_with_path() {
        let v = json!({"a": {"b": [1, 2.5]}});
        let err = encode(&v).unwrap_err();
        match err {
            CanonicalEncodingError::FloatRejected { path } => {
                assert!(path.contains("a") && path.contains("b") && path.contains("1"));
            }
            other => panic!("expected FloatRejected, got {other:?}"),
        }
    }

    #[test]
    fn string_escaping_backslash_and_quote() {
        let v = json!({"s": "a\\b\"c"});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("a\\\\b\\\"c"));
    }

    #[test]
    fn control_chars_escaped() {
        let v = json!({"s": "\x00\n"});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("\\u0000"));
        assert!(out.contains("\\n"));
    }

    #[test]
    fn array_order_preserved() {
        let v = json!([3, 1, 2]);
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, "[3,1,2]");
    }

    #[test]
    fn no_whitespace_in_output() {
        let v = json!({"a": [1, {"b": 2}], "c": "hello"});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(!out.contains(' '));
        assert!(!out.contains('\t'));
        assert!(!out.contains('\n'));
    }

    #[test]
    fn null_value() {
        let v = json!(null);
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, "null");
    }

    #[test]
    fn bool_values() {
        assert_eq!(
            String::from_utf8(encode(&json!(true)).unwrap()).unwrap(),
            "true"
        );
        assert_eq!(
            String::from_utf8(encode(&json!(false)).unwrap()).unwrap(),
            "false"
        );
    }

    #[test]
    fn deterministic_output() {
        let v = json!({"z": 1, "a": 2, "m": [3, 4]});
        let out1 = encode(&v).unwrap();
        let out2 = encode(&v).unwrap();
        assert_eq!(out1, out2);
    }

    #[test]
    fn distinct_values_produce_distinct_bytes() {
        let v1 = json!({"seq": 0});
        let v2 = json!({"seq": 1});
        assert_ne!(encode(&v1).unwrap(), encode(&v2).unwrap());
    }

    #[test]
    fn unicode_round_trip() {
        let v = json!({"emoji": "🔥", "cjk": "漢字"});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert!(out.contains("🔥"));
        assert!(out.contains("漢字"));
    }

    #[test]
    fn zero_integer() {
        let v = json!({"n": 0});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"n":0}"#);
    }

    #[test]
    fn negative_one() {
        let v = json!({"n": -1});
        let out = String::from_utf8(encode(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"n":-1}"#);
    }
}
