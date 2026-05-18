#![allow(clippy::unwrap_used, clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../eval-ingest/tests/fixtures")
        .join(name)
}

fn mojave() -> Command {
    Command::cargo_bin("mojave").expect("binary should exist")
}

#[test]
fn help_flag() {
    mojave()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Measurement engine"));
}

#[test]
fn ingest_inspect_json_outputs_valid_json() {
    let output = mojave()
        .args([
            "ingest",
            fixture_path("inspect_binary.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    assert!(parsed["records"].is_array(), "should have records array");
    assert!(
        parsed["source_meta"]["runner_name"].is_string(),
        "should have source_meta"
    );
}

#[test]
fn ingest_jsonl_outputs_valid_json() {
    let output = mojave()
        .args(["ingest", fixture_path("basic.jsonl").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    let records = parsed["records"]
        .as_array()
        .expect("records should be array");
    assert_eq!(records.len(), 5);
}

#[test]
fn analyze_outputs_valid_json_with_decisions() {
    let output = mojave()
        .args([
            "analyze",
            fixture_path("inspect_binary.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout should be valid JSON");
    assert!(parsed["decisions"].is_array(), "should have decisions");
    assert!(
        parsed["instruments_run"].is_array(),
        "should have instruments_run"
    );
    assert!(
        parsed["series_detected"].is_array(),
        "should have series_detected"
    );
    assert!(parsed["summaries"].is_object(), "should have summaries");
}

#[test]
fn analyze_decisions_have_hint_field() {
    let output = mojave()
        .args([
            "analyze",
            fixture_path("inspect_binary.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    if let Some(decisions) = parsed["decisions"].as_array() {
        for d in decisions {
            assert!(
                d["hint"].is_string(),
                "each decision should have a hint field"
            );
        }
    }
}

#[test]
fn missing_file_returns_exit_1() {
    mojave()
        .args(["analyze", "nonexistent_file_12345.json"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn invalid_flag_returns_exit_2() {
    mojave()
        .args(["analyze", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn monitor_with_watch_file() {
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    use std::io::Write;

    for i in 0..3 {
        let record = serde_json::json!({
            "trial_id": format!("01JAAA000000000000000000{:02}", i),
            "run_id": "01JAAA00000000000000000000",
            "task_id": "t1",
            "task_version": null,
            "agent_id": "a1",
            "agent_version": null,
            "judge_config": null,
            "seed": null,
            "timestamp": 1717200000 + i,
            "outcome": {"type": "Score", "value": 0.8},
            "metadata": {}
        });
        writeln!(tmp, "{}", serde_json::to_string(&record).unwrap()).unwrap();
    }

    let output = mojave()
        .args(["monitor", "--watch", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let last_line = String::from_utf8(output)
        .unwrap()
        .lines()
        .last()
        .unwrap_or("")
        .to_string();
    let summary: serde_json::Value =
        serde_json::from_str(&last_line).expect("last line should be valid JSON");
    assert!(
        summary["observations_seen"].is_number(),
        "summary should have observations_seen"
    );
}

#[test]
fn completions_bash() {
    mojave()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_mojave"));
}
