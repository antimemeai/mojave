use eval_orchestrator::types::{Decision, MeasurementIssue};

pub fn decision_hint(decision: &Decision) -> String {
    match decision {
        Decision::StopEarly {
            evidence,
            estimate,
            ci,
            ..
        } => {
            let half_width = (ci.1 - ci.0) / 2.0;
            format!(
                "Effect stable at {estimate:.2} \u{00b1} {half_width:.2}. Evidence ({evidence:.1}) exceeds threshold \u{2014} safe to stop."
            )
        }
        Decision::ContinueRunning { current_n, .. } => {
            format!("{current_n} observations, insufficient evidence.")
        }
        Decision::Regression {
            observation_value,
            control_limits,
            ..
        } => {
            format!(
                "Observation {observation_value:.3} outside control limits [{:.3}, {:.3}].",
                control_limits.0, control_limits.1
            )
        }
        Decision::MeasurementWarning { issue, .. } => match issue {
            MeasurementIssue::LowAgreement { kappa, threshold } => {
                format!(
                    "Inter-rater agreement (\u{03ba}={kappa:.2}) below threshold ({threshold:.2})."
                )
            }
            MeasurementIssue::InsufficientRaters { have, need } => {
                format!(
                    "Only {have} rater(s) found, need \u{2265}{need} for inter-rater reliability."
                )
            }
            MeasurementIssue::InsufficientSamples { have, need } => {
                format!("Only {have} sample(s), need \u{2265}{need}.")
            }
            MeasurementIssue::HighVariance { cv, threshold } => {
                format!("Coefficient of variation ({cv:.2}) exceeds threshold ({threshold:.2}).")
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_orchestrator::types::SeriesKey;

    fn test_series() -> SeriesKey {
        SeriesKey {
            task_id: "t".into(),
            agent_id: "a".into(),
            scorer: None,
        }
    }

    #[test]
    fn hint_stop_early() {
        let d = Decision::StopEarly {
            series: test_series(),
            evidence: 47.2,
            estimate: 0.82,
            ci: (0.79, 0.85),
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.82"), "hint should contain estimate");
        assert!(
            hint.contains("safe to stop"),
            "hint should say safe to stop"
        );
    }

    #[test]
    fn hint_continue_running() {
        let d = Decision::ContinueRunning {
            series: test_series(),
            current_n: 38,
            estimated_n_needed: 0,
            power_at_current_n: 0.0,
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("38"), "hint should contain current_n");
        assert!(
            hint.contains("insufficient"),
            "hint should say insufficient"
        );
    }

    #[test]
    fn hint_regression() {
        let d = Decision::Regression {
            series: test_series(),
            signal: spc_charts::ChartSignal::InControl,
            observation_value: 0.43,
            control_limits: (0.71, 0.89),
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.430"), "hint should contain observation");
        assert!(hint.contains("0.710"), "hint should contain lower limit");
    }

    #[test]
    fn hint_low_agreement() {
        let d = Decision::MeasurementWarning {
            series: test_series(),
            issue: MeasurementIssue::LowAgreement {
                kappa: 0.31,
                threshold: 0.67,
            },
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("0.31"), "hint should contain kappa");
        assert!(hint.contains("0.67"), "hint should contain threshold");
    }

    #[test]
    fn hint_insufficient_raters() {
        let d = Decision::MeasurementWarning {
            series: test_series(),
            issue: MeasurementIssue::InsufficientRaters { have: 1, need: 2 },
        };
        let hint = decision_hint(&d);
        assert!(hint.contains("1 rater"), "hint should contain count");
    }
}
