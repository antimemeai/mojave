/// A parsed Gherkin feature file.
#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub source: String,
    pub scenarios: Vec<Scenario>,
}

/// A single scenario (plain or outline).
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub line: usize,
    pub steps: Vec<Step>,
    pub kind: ScenarioKind,
}

/// Whether a scenario is a plain scenario or a scenario outline with
/// an examples table.
#[derive(Debug, Clone)]
pub enum ScenarioKind {
    Plain,
    Outline { examples: Examples },
}

/// A single step (Given/When/Then/And/But).
#[derive(Debug, Clone)]
pub struct Step {
    pub text: String,
    pub line: usize,
    pub kind: StepKind,
}

/// The keyword that introduced a step — preserved for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Given,
    When,
    Then,
    And,
    But,
}

/// An Examples table attached to a Scenario Outline.
#[derive(Debug, Clone)]
pub struct Examples {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Examples {
    /// Substitute `<placeholder>` tokens in `text` with values from
    /// the given example row.
    pub fn substitute(&self, text: &str, row_index: usize) -> String {
        let row = &self.rows[row_index];
        let mut result = text.to_string();
        for (i, header) in self.headers.iter().enumerate() {
            let placeholder = format!("<{header}>");
            if let Some(value) = row.get(i) {
                result = result.replace(&placeholder, value);
            }
        }
        result
    }
}
