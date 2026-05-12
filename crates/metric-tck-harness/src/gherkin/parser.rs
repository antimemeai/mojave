use super::model::{Examples, Feature, Scenario, ScenarioKind, Step, StepKind};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a Gherkin feature file from its text content.
///
/// `source` is a human-readable name for the file (used in error
/// messages and diagnostic output).
pub fn parse_feature(content: &str, source: &str) -> Result<Feature, ParseError> {
    let lines: Vec<(usize, &str)> = content
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .collect();

    let mut feature_name = String::new();
    let mut scenarios: Vec<Scenario> = Vec::new();
    let mut idx = 0;

    while idx < lines.len() {
        let (line_num, line) = lines[idx];
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            idx += 1;
            continue;
        }

        if let Some(name) = trimmed.strip_prefix("Feature:") {
            feature_name = name.trim().to_string();
            idx += 1;
            // Skip feature description lines until we hit a scenario or EOF
            while idx < lines.len() {
                let t = lines[idx].1.trim();
                if t.starts_with("Scenario") || t.starts_with("@") || t.starts_with("Feature:") {
                    break;
                }
                idx += 1;
            }
            continue;
        }

        if trimmed.starts_with("Scenario Outline:") || trimmed.starts_with("Scenario Template:") {
            let name = trimmed
                .trim_start_matches("Scenario Outline:")
                .trim_start_matches("Scenario Template:")
                .trim()
                .to_string();
            let (scenario, next_idx) = parse_scenario(name, line_num, &lines, idx + 1, true)?;
            scenarios.push(scenario);
            idx = next_idx;
            continue;
        }

        if let Some(name) = trimmed.strip_prefix("Scenario:") {
            let name = name.trim().to_string();
            let (scenario, next_idx) = parse_scenario(name, line_num, &lines, idx + 1, false)?;
            scenarios.push(scenario);
            idx = next_idx;
            continue;
        }

        idx += 1;
    }

    if feature_name.is_empty() {
        return Err(ParseError {
            message: format!("{source}: no Feature: line found"),
        });
    }

    Ok(Feature {
        name: feature_name,
        source: source.to_string(),
        scenarios,
    })
}

fn parse_scenario(
    name: String,
    line: usize,
    lines: &[(usize, &str)],
    start: usize,
    is_outline: bool,
) -> Result<(Scenario, usize), ParseError> {
    let mut steps: Vec<Step> = Vec::new();
    let mut idx = start;
    let mut examples: Option<Examples> = None;

    while idx < lines.len() {
        let (line_num, raw) = lines[idx];
        let trimmed = raw.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            idx += 1;
            continue;
        }

        if trimmed.starts_with("Scenario") || trimmed.starts_with("Feature:") {
            break;
        }

        if trimmed.starts_with("Examples:") || trimmed.starts_with("Scenarios:") {
            let (ex, next_idx) = parse_examples(lines, idx + 1)?;
            examples = Some(ex);
            idx = next_idx;
            continue;
        }

        if let Some((kind, text)) = parse_step_line(trimmed) {
            steps.push(Step {
                text: text.to_string(),
                line: line_num,
                kind,
            });
            idx += 1;
            continue;
        }

        // Skip unrecognized lines (tags, blank continuations, etc.)
        idx += 1;
    }

    let kind = if is_outline {
        ScenarioKind::Outline {
            examples: examples.unwrap_or(Examples {
                headers: Vec::new(),
                rows: Vec::new(),
            }),
        }
    } else {
        ScenarioKind::Plain
    };

    Ok((
        Scenario {
            name,
            line,
            steps,
            kind,
        },
        idx,
    ))
}

fn parse_step_line(trimmed: &str) -> Option<(StepKind, &str)> {
    for (prefix, kind) in [
        ("Given ", StepKind::Given),
        ("When ", StepKind::When),
        ("Then ", StepKind::Then),
        ("And ", StepKind::And),
        ("But ", StepKind::But),
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some((kind, rest));
        }
    }
    None
}

fn parse_examples(lines: &[(usize, &str)], start: usize) -> Result<(Examples, usize), ParseError> {
    let mut idx = start;
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    while idx < lines.len() {
        let trimmed = lines[idx].1.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            idx += 1;
            continue;
        }

        if !trimmed.starts_with('|') {
            break;
        }

        let cells: Vec<String> = trimmed
            .split('|')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        if headers.is_empty() {
            headers = cells;
        } else {
            rows.push(cells);
        }
        idx += 1;
    }

    Ok((Examples { headers, rows }, idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_feature() {
        let src = "\
Feature: minimal

  Scenario: trivial
    Given a step
    When another step
    Then a result
";
        let f = parse_feature(src, "test.feature").unwrap();
        assert_eq!(f.name, "minimal");
        assert_eq!(f.scenarios.len(), 1);
        assert_eq!(f.scenarios[0].steps.len(), 3);
    }

    #[test]
    fn parses_scenario_outline() {
        let src = "\
Feature: outlined

  Scenario Outline: per row
    Given the value is <val>

    Examples:
      | val   |
      | alpha |
      | beta  |
";
        let f = parse_feature(src, "").unwrap();
        assert_eq!(f.scenarios.len(), 1);
        match &f.scenarios[0].kind {
            ScenarioKind::Outline { examples } => {
                assert_eq!(examples.headers, vec!["val"]);
                assert_eq!(examples.rows.len(), 2);
                assert_eq!(
                    examples.substitute("the value is <val>", 0),
                    "the value is alpha"
                );
            }
            ScenarioKind::Plain => panic!("expected outline"),
        }
    }

    #[test]
    fn skips_comments_and_tags() {
        let src = "\
# A comment
@tag
Feature: tagged

  # Another comment
  Scenario: simple
    Given a step
";
        let f = parse_feature(src, "").unwrap();
        assert_eq!(f.name, "tagged");
        assert_eq!(f.scenarios.len(), 1);
    }

    #[test]
    fn no_feature_line_is_error() {
        let result = parse_feature("Scenario: orphan\n  Given a step\n", "bad.feature");
        assert!(result.is_err());
    }
}
