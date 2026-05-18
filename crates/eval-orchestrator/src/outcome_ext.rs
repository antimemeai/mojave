use eval_core::Outcome;

pub fn outcome_to_f64(outcome: &Outcome) -> f64 {
    match outcome {
        Outcome::Binary(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Outcome::Score(f) => *f,
        Outcome::Graded(g) => *g as f64 / 255.0,
        Outcome::MultiCriterion(m) => {
            if m.is_empty() {
                0.0
            } else {
                m.values().sum::<f64>() / m.len() as f64
            }
        }
    }
}

pub fn outcome_to_ordinal(outcome: &Outcome) -> u32 {
    match outcome {
        Outcome::Binary(b) => u32::from(*b),
        Outcome::Score(f) => (*f * 1000.0) as u32,
        Outcome::Graded(g) => *g as u32,
        Outcome::MultiCriterion(m) => {
            if m.is_empty() {
                0
            } else {
                (m.values().sum::<f64>() / m.len() as f64 * 1000.0) as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn binary_true_to_f64() {
        assert!((outcome_to_f64(&Outcome::Binary(true)) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn binary_false_to_f64() {
        assert!((outcome_to_f64(&Outcome::Binary(false)) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_to_f64() {
        assert!((outcome_to_f64(&Outcome::Score(0.75)) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn graded_to_f64() {
        let expected = 128.0 / 255.0;
        assert!((outcome_to_f64(&Outcome::Graded(128)) - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn multi_criterion_to_f64() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), 0.8);
        m.insert("b".to_string(), 0.4);
        assert!((outcome_to_f64(&Outcome::MultiCriterion(m)) - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_multi_criterion_to_f64() {
        let m = BTreeMap::new();
        assert!((outcome_to_f64(&Outcome::MultiCriterion(m)) - 0.0).abs() < f64::EPSILON);
    }
}
