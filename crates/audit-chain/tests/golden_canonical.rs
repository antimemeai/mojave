#![allow(clippy::unwrap_used, clippy::expect_used)]

use audit_chain::canonical::encode;
use serde_json::json;

#[test]
fn golden_empty_object() {
    assert_eq!(encode(&json!({})).unwrap(), b"{}");
}

#[test]
fn golden_sorted_keys() {
    let out = encode(&json!({"z": 1, "a": 2, "m": 3})).unwrap();
    assert_eq!(out, br#"{"a":2,"m":3,"z":1}"#);
}

#[test]
fn golden_nested_sorted() {
    let out = encode(&json!({"b": {"z": 1, "a": 2}, "a": 0})).unwrap();
    assert_eq!(out, br#"{"a":0,"b":{"a":2,"z":1}}"#);
}

#[test]
fn golden_string_escaping() {
    let out = String::from_utf8(encode(&json!({"s": "a\tb\nc"})).unwrap()).unwrap();
    assert_eq!(out, r#"{"s":"a\tb\nc"}"#);
}

#[test]
fn golden_control_char_hex() {
    let out = String::from_utf8(encode(&json!({"s": "\x01\x1f"})).unwrap()).unwrap();
    // \x01 -> , \x1f -> 
    assert_eq!(out, "{\"s\":\"\\u0001\\u001f\"}");
}

#[test]
fn golden_unicode_passthrough() {
    let out = String::from_utf8(encode(&json!({"k": "漢字🔥"})).unwrap()).unwrap();
    assert_eq!(out, r#"{"k":"漢字🔥"}"#);
}

#[test]
fn golden_array_preserves_order() {
    assert_eq!(encode(&json!([3, 1, 2])).unwrap(), b"[3,1,2]");
}

#[test]
fn golden_integers() {
    assert_eq!(
        encode(&json!({"a": -1, "b": 0, "c": 18446744073709551615u64})).unwrap(),
        br#"{"a":-1,"b":0,"c":18446744073709551615}"#
    );
}

#[test]
fn golden_mixed_types() {
    let out = encode(&json!({
        "arr": [1, "two", null, true, false],
        "n": 42,
        "s": "hello"
    }))
    .unwrap();
    assert_eq!(
        out,
        br#"{"arr":[1,"two",null,true,false],"n":42,"s":"hello"}"#
    );
}

#[test]
fn golden_supplementary_plane_keys_sort_by_utf8() {
    // U+10002 (Linear B) and U+FF61 (Halfwidth Katakana)
    // UTF-8 order: U+FF61 (ef bd a1) < U+10002 (f0 90 80 82)
    // UTF-16 order: U+10002 (D800 DC02) < U+FF61
    // We sort by UTF-8 (Rust String::cmp), not UTF-16 (JCS).
    let v = json!({"\u{10002}": 1, "\u{FF61}": 2});
    let out = String::from_utf8(encode(&v).unwrap()).unwrap();
    let pos_ff61 = out.find('\u{FF61}').unwrap();
    let pos_10002 = out.find('\u{10002}').unwrap();
    assert!(
        pos_ff61 < pos_10002,
        "UTF-8 sort: U+FF61 ({pos_ff61}) should precede U+10002 ({pos_10002})"
    );
}
