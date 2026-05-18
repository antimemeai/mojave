use super::FormatAtoms;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ApplyError {
    #[error("region {start}..{end} is not valid UTF-8")]
    InvalidUtf8 { start: usize, end: usize },
}

#[must_use]
pub fn longest_string_region(body: &[u8]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        if b == b'"' || b == b'\'' {
            let quote = b;
            let mut j = i + 1;
            let mut in_escape = false;
            while j < body.len() {
                let c = body[j];
                if in_escape {
                    in_escape = false;
                } else if c == b'\\' {
                    in_escape = true;
                } else if c == quote {
                    break;
                }
                j += 1;
            }
            if j < body.len() && body[j] == quote {
                let start = i + 1;
                let end = j;
                if end > start && std::str::from_utf8(&body[start..end]).is_ok() {
                    let len = end - start;
                    let best_len = best.map_or(0, |(s, e)| e - s);
                    if len > best_len {
                        best = Some((start, end));
                    }
                }
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    best
}

/// Apply format atoms to the substring at `region` and splice back
/// into `body`, returning the new bytes.
pub fn apply_atoms(
    body: &[u8],
    atoms: &FormatAtoms,
    region: (usize, usize),
) -> Result<Vec<u8>, ApplyError> {
    let (start, end) = region;
    let original = std::str::from_utf8(&body[start..end])
        .map_err(|_| ApplyError::InvalidUtf8 { start, end })?;
    let perturbed = transform_region(original, atoms);
    let mut out = Vec::with_capacity(body.len() + perturbed.len());
    out.extend_from_slice(&body[..start]);
    out.extend_from_slice(perturbed.as_bytes());
    out.extend_from_slice(&body[end..]);
    Ok(out)
}

fn transform_region(original: &str, atoms: &FormatAtoms) -> String {
    let mut out = String::with_capacity(original.len());
    let bytes = original.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_uppercase() {
            let label_start = i;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b':' && bytes[j + 1] == b' ' {
                let label = &original[label_start..j];
                out.push_str(&atoms.casing.apply(label));
                out.push_str(atoms.separator.as_str());
                i = j + 2;
                continue;
            }
            out.push_str(&original[label_start..j]);
            i = j;
            continue;
        }
        let ch_end = next_char_boundary(original, i);
        out.push_str(&original[i..ch_end]);
        i = ch_end;
    }

    let punctuated = atoms.punctuation.apply(&out);
    atoms.padding.apply(&punctuated)
}

fn next_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return i;
    }
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Casing, FormatAtoms, Padding, Punctuation, Separator};

    #[test]
    fn longest_string_region_finds_quoted_substring() {
        let body = br#"{"messages":[{"role":"user","content":"What is 2+2?"}]}"#;
        let region = longest_string_region(body).expect("region found");
        let s = std::str::from_utf8(&body[region.0..region.1]).unwrap();
        assert_eq!(s, "What is 2+2?");
    }

    #[test]
    fn longest_string_region_picks_the_longest() {
        let body = br#"{"a":"short","b":"a much longer prompt content"}"#;
        let region = longest_string_region(body).expect("region found");
        let s = std::str::from_utf8(&body[region.0..region.1]).unwrap();
        assert_eq!(s, "a much longer prompt content");
    }

    #[test]
    fn longest_string_region_handles_escaped_quotes() {
        let body = br#"{"q":"he said \"hi\" politely"}"#;
        let region = longest_string_region(body).expect("region found");
        let s = std::str::from_utf8(&body[region.0..region.1]).unwrap();
        assert_eq!(s, r#"he said \"hi\" politely"#);
    }

    #[test]
    fn longest_string_region_returns_none_on_no_quotes() {
        let body = b"raw bytes no quotes here";
        assert_eq!(longest_string_region(body), None);
    }

    #[test]
    fn longest_string_region_returns_none_on_non_utf8() {
        let body = b"\"\xff\xfe\"";
        assert_eq!(longest_string_region(body), None);
    }

    #[test]
    fn apply_atoms_rewrites_label_separator_punctuation() {
        let body = br#"{"content":"Question: what is 2+2?"}"#;
        let atoms = FormatAtoms {
            separator: Separator::Newline,
            casing: Casing::Lower,
            punctuation: Punctuation::Period,
            padding: Padding::Original,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("question\nwhat is 2+2."), "perturbed body: {s}");
    }

    #[test]
    fn apply_atoms_leaves_wrapper_untouched() {
        let body = br#"{"content":"Question: hi?"}"#;
        let atoms = FormatAtoms {
            separator: Separator::Newline,
            casing: Casing::Upper,
            punctuation: Punctuation::None,
            padding: Padding::Original,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.starts_with(r#"{"content":""#));
        assert!(s.ends_with(r#""}"#));
    }

    #[test]
    fn apply_atoms_changes_bytes() {
        let body = br#"{"x":"Question: what is 2+2?"}"#;
        let atoms = FormatAtoms {
            separator: Separator::ArrowSpace,
            casing: Casing::Upper,
            punctuation: Punctuation::Period,
            padding: Padding::Original,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        assert_ne!(out, body.as_slice());
    }

    #[test]
    fn apply_atoms_no_label_falls_back_to_punctuation_only() {
        let body = br#"{"x":"hello world?"}"#;
        let atoms = FormatAtoms {
            separator: Separator::Newline,
            casing: Casing::Lower,
            punctuation: Punctuation::Period,
            padding: Padding::Original,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("hello world."), "got {s}");
    }

    #[test]
    fn padding_quotes_encloses() {
        let body = br#"{"x":"Question: hi?"}"#;
        let atoms = FormatAtoms {
            separator: Separator::ColonSpace,
            casing: Casing::Original,
            punctuation: Punctuation::Question,
            padding: Padding::QuotesEnclose,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains(r#"\"Question: hi?\""#), "got {s}");
    }

    #[test]
    fn padding_newlines_prepend() {
        let body = br#"{"x":"hello"}"#;
        let atoms = FormatAtoms {
            separator: Separator::ColonSpace,
            casing: Casing::Original,
            punctuation: Punctuation::None,
            padding: Padding::NewlinesPrepend,
        };
        let region = longest_string_region(body).unwrap();
        let out = apply_atoms(body, &atoms, region).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains(r#"\n\nhello"#), "got {s}");
    }

    #[test]
    fn padding_keeps_json_wrapper_parseable() {
        let body = br#"{"x":"hello"}"#;
        for padding in [
            Padding::QuotesEnclose,
            Padding::NewlinesPrepend,
            Padding::NewlinesAppend,
            Padding::NewlinesBoth,
        ] {
            let atoms = FormatAtoms {
                separator: Separator::ColonSpace,
                casing: Casing::Original,
                punctuation: Punctuation::None,
                padding,
            };
            let region = longest_string_region(body).unwrap();
            let out = apply_atoms(body, &atoms, region).unwrap();
            let parsed: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert!(parsed.get("x").is_some(), "padding={padding:?}");
        }
    }
}
