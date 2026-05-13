#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("fixtures")
}

#[test]
fn obf_matches_gsdesign() {
    let path = fixtures_dir().join("gsdesign_obf.json");
    if !path.exists() {
        eprintln!("SKIP: OBF fixtures not found at {}", path.display());
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();

    for fixture in &fixtures {
        let k = fixture["K"].as_u64().unwrap() as usize;
        let r_bounds: Vec<f64> = fixture["upper_bounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();

        let our_bounds = seq_anytime_valid::boundary::obf::boundaries(k, 0.05).unwrap();

        for (i, (&r_val, &our_val)) in r_bounds.iter().zip(our_bounds.iter()).enumerate() {
            let diff = (r_val - our_val).abs();
            assert!(
                diff < 0.1,
                "OBF K={k} look {}: gsDesign={r_val:.4}, ours={our_val:.4}, diff={diff:.4}",
                i + 1
            );
        }
    }
}
