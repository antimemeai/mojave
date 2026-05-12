use cucumber::{given, then, when, World};
use eval_core::{JudgeConfig, Outcome, TrialRecord};
use std::collections::BTreeMap;
use ulid::Ulid;

#[derive(Debug, Default, World)]
pub struct TrialRecordWorld {
    records: Vec<TrialRecord>,
    json: Option<String>,
    deserialized: Option<TrialRecord>,
    judge_config: Option<JudgeConfig>,
    outcome_error: Option<String>,
    judge_config_error: Option<String>,
}

const FIXED_TRIAL_ID: u128 = 1;
const FIXED_RUN_ID: u128 = 2;

fn make_record(outcome: Outcome, judge_config: Option<JudgeConfig>) -> TrialRecord {
    TrialRecord {
        trial_id: Ulid::from(FIXED_TRIAL_ID),
        run_id: Ulid::from(FIXED_RUN_ID),
        task_id: "task-042".to_string(),
        task_version: None,
        agent_id: "agent-001".to_string(),
        agent_version: None,
        judge_config,
        seed: Some(42),
        timestamp: 1715400000,
        outcome,
        metadata: BTreeMap::new(),
    }
}

#[given(expr = "a TrialRecord with binary outcome {word}")]
fn given_binary_record(world: &mut TrialRecordWorld, val: String) {
    let outcome = Outcome::Binary(val == "true");
    world.records.push(make_record(outcome, None));
}

#[given(expr = "agent_id {string} and task_id {string}")]
fn given_ids(world: &mut TrialRecordWorld, agent: String, task: String) {
    if let Some(r) = world.records.last_mut() {
        r.agent_id = agent;
        r.task_id = task;
    }
}

#[given(expr = "judge_config with model {string} and family {string}")]
fn given_judge_config(world: &mut TrialRecordWorld, model: String, family: String) {
    if let Some(r) = world.records.last_mut() {
        r.judge_config = Some(JudgeConfig {
            model,
            family,
            prompt_template_hash: "hash".to_string(),
            temperature: 0.0,
            seed: None,
        });
    }
}

#[given(expr = "a TrialRecord with score outcome {float}")]
fn given_score_record(world: &mut TrialRecordWorld, val: f64) {
    world.records.push(make_record(Outcome::Score(val), None));
}

#[given(expr = "a TrialRecord with graded outcome {int}")]
fn given_graded_record(world: &mut TrialRecordWorld, val: u8) {
    world.records.push(make_record(Outcome::Graded(val), None));
}

#[given("a TrialRecord with no judge_config")]
fn given_no_judge(world: &mut TrialRecordWorld) {
    world.records.push(make_record(Outcome::Binary(true), None));
}

#[given(expr = "a TrialRecord with multi-criterion outcome {string}")]
fn given_multi_criterion(world: &mut TrialRecordWorld, spec: String) {
    let mut criteria = BTreeMap::new();
    for pair in spec.split(',') {
        let mut parts = pair.split('=');
        let key = parts.next().unwrap().trim().to_string();
        let value: f64 = parts.next().unwrap().trim().parse().unwrap();
        criteria.insert(key, value);
    }
    world
        .records
        .push(make_record(Outcome::MultiCriterion(criteria), None));
}

#[given(expr = "a TrialRecord with metadata key {string} value {string}")]
fn given_metadata(world: &mut TrialRecordWorld, key: String, value: String) {
    let mut record = make_record(Outcome::Binary(true), None);
    record
        .metadata
        .insert(key, serde_json::Value::String(value));
    world.records.push(record);
}

#[given("a TrialRecord with no seed")]
fn given_no_seed(world: &mut TrialRecordWorld) {
    let mut record = make_record(Outcome::Binary(true), None);
    record.seed = None;
    world.records.push(record);
}

#[given(expr = "a JudgeConfig with model {string} and family {string}")]
fn given_standalone_judge_config(world: &mut TrialRecordWorld, model: String, family: String) {
    world.judge_config = Some(JudgeConfig {
        model,
        family,
        prompt_template_hash: "hash".to_string(),
        temperature: 0.0,
        seed: None,
    });
}

#[when("I serialize to JSON")]
fn serialize_json(world: &mut TrialRecordWorld) {
    let record = world.records.last().unwrap();
    world.json = Some(serde_json::to_string(record).unwrap());
}

#[when("deserialize back")]
fn deserialize_json(world: &mut TrialRecordWorld) {
    let json = world.json.as_ref().expect("no JSON to deserialize");
    world.deserialized = Some(serde_json::from_str(json).unwrap());
}

#[when("I serialize to JSON and deserialize back")]
fn json_roundtrip(world: &mut TrialRecordWorld) {
    let record = world.records.last().unwrap();
    let json = serde_json::to_string(record).unwrap();
    world.deserialized = Some(serde_json::from_str(&json).unwrap());
}

#[when("I construct an Outcome::Score with NaN")]
fn construct_nan_score(world: &mut TrialRecordWorld) {
    match Outcome::score(f64::NAN) {
        Ok(_) => {}
        Err(e) => world.outcome_error = Some(e.to_string()),
    }
}

#[when("I construct an Outcome::Score with Infinity")]
fn construct_inf_score(world: &mut TrialRecordWorld) {
    match Outcome::score(f64::INFINITY) {
        Ok(_) => {}
        Err(e) => world.outcome_error = Some(e.to_string()),
    }
}

#[when("I construct a JudgeConfig with NaN temperature")]
fn construct_nan_temperature(world: &mut TrialRecordWorld) {
    match JudgeConfig::new(
        "model".into(),
        "family".into(),
        "hash".into(),
        f32::NAN,
        None,
    ) {
        Ok(_) => {}
        Err(e) => world.judge_config_error = Some(e.to_string()),
    }
}

#[then("the round-tripped record equals the original")]
fn assert_roundtrip(world: &mut TrialRecordWorld) {
    let original = world.records.last().unwrap();
    let deserialized = world.deserialized.as_ref().unwrap();
    assert_eq!(original, deserialized);
}

#[then("the two outcomes are not equal")]
fn assert_outcomes_differ(world: &mut TrialRecordWorld) {
    assert!(world.records.len() >= 2);
    assert_ne!(world.records[0].outcome, world.records[1].outcome);
}

#[then(expr = "the family field is {string}")]
fn assert_family(world: &mut TrialRecordWorld, expected: String) {
    let jc = world.judge_config.as_ref().unwrap();
    assert_eq!(jc.family, expected);
}

#[then("the judge_config field is null")]
fn assert_null_judge(world: &mut TrialRecordWorld) {
    let json = world.json.as_ref().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(v["judge_config"].is_null());
}

#[then("all three criteria are preserved with exact values")]
fn assert_multi_criterion(world: &mut TrialRecordWorld) {
    let deserialized = world.deserialized.as_ref().unwrap();
    if let Outcome::MultiCriterion(ref m) = deserialized.outcome {
        assert_eq!(m.len(), 3);
        assert_eq!(m["accuracy"], 0.92);
        assert_eq!(m["helpfulness"], 0.78);
        assert_eq!(m["safety"], 1.0);
    } else {
        panic!("Expected MultiCriterion outcome");
    }
}

#[then(expr = "the metadata key {string} has value {string}")]
fn assert_metadata(world: &mut TrialRecordWorld, key: String, expected: String) {
    let deserialized = world.deserialized.as_ref().unwrap();
    let val = deserialized
        .metadata
        .get(&key)
        .expect("metadata key missing");
    assert_eq!(val.as_str().unwrap(), expected);
}

#[then("the seed field is null")]
fn assert_null_seed(world: &mut TrialRecordWorld) {
    let json = world.json.as_ref().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(v["seed"].is_null());
}

#[then("I get a non-finite score error")]
fn assert_outcome_error(world: &mut TrialRecordWorld) {
    let err = world
        .outcome_error
        .as_ref()
        .expect("expected outcome error");
    assert!(err.contains("finite"), "error: {err}");
}

#[then("I get a non-finite temperature error")]
fn assert_judge_config_error(world: &mut TrialRecordWorld) {
    let err = world
        .judge_config_error
        .as_ref()
        .expect("expected judge config error");
    assert!(err.contains("finite"), "error: {err}");
}

#[then(expr = "the outcome JSON has a {string} field with value {string}")]
fn assert_outcome_tagged(world: &mut TrialRecordWorld, field: String, expected: String) {
    let json = world.json.as_ref().unwrap();
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    let outcome = &v["outcome"];
    assert_eq!(
        outcome[&field].as_str().unwrap(),
        expected,
        "outcome.{field} mismatch in {outcome}"
    );
}

fn main() {
    let runner = TrialRecordWorld::run("../../tck/eval-core");
    futures::executor::block_on(runner);
}
