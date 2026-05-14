//! `Problem` — the declarative, content-addressable description of
//! an SA campaign's input space. Vector of factors, each with a name,
//! distribution, and kind (continuous / discrete / categorical /
//! boolean).
//!
//! # The construction discipline
//!
//! Mirrors `workspace_core::AuditEntryBuilder`'s single-writer
//! pattern: `Problem` is `#[non_exhaustive]`; only `ProblemBuilder::build`
//! produces `Problem` values. External callers cannot construct via
//! struct literal, cannot use `Default`-then-mutate, cannot reach
//! around the builder's parameter validation. Every `Problem` value
//! reachable across crate boundaries has been validated at build
//! time.
//!
//! Per `decisions/2026-04-28-saltelli-problem-shape.md` § "What this
//! gates."
//!
//! # Content-addressing
//!
//! `Problem::content_hash() -> [u8; 32]` returns SHA-256 over the
//! canonical-JSON serialization of the `Problem`. Stable across
//! calls; content-equivalent `Problem`s hash equally; semantically
//! distinct `Problem`s hash distinctly. The hash lives inside
//! saltelli's `context` payloads on the audit envelope (per
//! `decisions/2026-04-28-saltelli-ledger-composition.md`) — *not*
//! as a parallel provenance attestation, but as a content-identifier
//! for "which Problem produced this result?"
//!
//! Blake3 deferred to a follow-on PR per
//! `decisions/2026-04-28-saltelli-rng-determinism.md` § "Why SHA-256
//! and not Blake3" — Problem JSON is small (factor descriptions, not
//! sample matrices); SHA-256 is already in the workspace dep graph.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::distribution::Distribution;

/// The role a factor plays in the experiment. Closed enum,
/// `#[non_exhaustive]`. Continuous is the default for typical SA
/// applications.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind")]
pub enum FactorKind {
    /// Continuous real-valued factor — the typical SA case.
    #[default]
    Continuous,
    /// Integer-valued factor (still varied via `quantile` over a
    /// discrete distribution like `DiscreteUniform`).
    Discrete,
    /// Categorical factor with `n` distinct levels. Quantile maps
    /// `[0, 1]` to `{0, 1, …, n-1}` via the underlying
    /// `DiscreteUniform { 0, n-1 }` distribution.
    Categorical { n: usize },
    /// Boolean factor — equivalent to `Bernoulli` distribution.
    Boolean,
}

/// A single factor in the experiment. Name + distribution + kind.
/// `#[non_exhaustive]` blocks struct-literal construction outside
/// this crate; consumers go through `ProblemBuilder`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Factor {
    pub name: String,
    pub distribution: Distribution,
    pub kind: FactorKind,
}

/// A named group of factors treated as a single unit in SA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub factor_indices: Vec<usize>,
}

/// The declarative input-space description for an SA campaign.
///
/// `factors` + optional `groups`. The `correlation` / `output`
/// fields named in `rust_salib_crate_research.md` § 3.1 land via
/// follow-on PRs (each has its own design questions). `Problem` is
/// `#[non_exhaustive]` — adding those fields is non-breaking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Problem {
    pub factors: Vec<Factor>,
    /// Factor groups for grouped SA. `None` = ungrouped (each factor independent).
    pub groups: Option<Vec<Group>>,
}

impl Problem {
    /// Number of factors.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.factors.len()
    }

    /// Read-only view of factors.
    #[must_use]
    pub fn factors(&self) -> &[Factor] {
        &self.factors
    }

    /// SHA-256 over the canonical-JSON serialization. Stable across
    /// calls; content-equivalent Problems hash equally.
    ///
    /// # Panics
    ///
    /// Never. `serde_json::to_vec` on a `Problem` cannot fail —
    /// every field is plain data.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn content_hash(&self) -> [u8; 32] {
        // `serde_json::to_vec` cannot fail on a Problem — every field
        // is plain data with stable serde representations (no
        // HashMap whose iteration order varies; no float NaN that
        // doesn't round-trip; no I/O). The `.expect` is a documented
        // panic, not a recoverable error path.
        let bytes = serde_json::to_vec(self)
            .expect("serializing Problem to JSON cannot fail (all plain data)");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }
}

/// Builder for `Problem`. The only public path to a `Problem` value.
#[derive(Debug, Default, Clone)]
pub struct ProblemBuilder {
    factors: Vec<Factor>,
    groups: Vec<Group>,
}

/// Errors arising from `ProblemBuilder::build`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    /// At least one factor is required.
    #[error("Problem must have at least one factor")]
    Empty,
    /// Factor names must be unique.
    #[error("duplicate factor name: {name}")]
    DuplicateName { name: String },
    /// Distribution parameters failed validation.
    #[error("invalid distribution for factor {name}: {reason}")]
    InvalidDistribution { name: String, reason: String },
    /// Categorical factor's `n` is 0.
    #[error("Categorical factor {name} must have n >= 1")]
    EmptyCategorical { name: String },
    /// A group has an empty `factor_indices` list.
    #[error("group {group} has empty factor_indices")]
    EmptyGroup { group: String },
    /// A group references a factor index beyond `factors.len()`.
    #[error("group {group}: factor index {index} out of range (dim={dim})")]
    GroupIndexOutOfRange {
        group: String,
        index: usize,
        dim: usize,
    },
    /// A factor appears in more than one group.
    #[error("factor {index} appears in multiple groups")]
    FactorInMultipleGroups { index: usize },
}

impl ProblemBuilder {
    /// Fresh empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a continuous factor with the given name and distribution.
    #[must_use]
    pub fn factor(mut self, name: &str, distribution: Distribution) -> Self {
        self.factors.push(Factor {
            name: name.to_string(),
            distribution,
            kind: FactorKind::Continuous,
        });
        self
    }

    /// Add a factor group. Indices must refer to factors already added.
    #[must_use]
    pub fn group(mut self, name: &str, factor_indices: &[usize]) -> Self {
        self.groups.push(Group {
            name: name.to_string(),
            factor_indices: factor_indices.to_vec(),
        });
        self
    }

    /// Add a factor with explicit `FactorKind`.
    #[must_use]
    pub fn factor_with_kind(
        mut self,
        name: &str,
        distribution: Distribution,
        kind: FactorKind,
    ) -> Self {
        self.factors.push(Factor {
            name: name.to_string(),
            distribution,
            kind,
        });
        self
    }

    /// Validate and finalize. Returns `Problem` on success; `BuildError`
    /// on validation failure (empty, duplicate names, invalid
    /// distribution params, empty Categorical).
    pub fn build(self) -> Result<Problem, BuildError> {
        if self.factors.is_empty() {
            return Err(BuildError::Empty);
        }

        // Duplicate-name check.
        let mut seen: Vec<&str> = Vec::with_capacity(self.factors.len());
        for f in &self.factors {
            if seen.contains(&f.name.as_str()) {
                return Err(BuildError::DuplicateName {
                    name: f.name.clone(),
                });
            }
            seen.push(f.name.as_str());
        }

        // Distribution + kind validation.
        for f in &self.factors {
            validate_distribution(&f.distribution).map_err(|reason| {
                BuildError::InvalidDistribution {
                    name: f.name.clone(),
                    reason,
                }
            })?;
            if let FactorKind::Categorical { n: 0 } = f.kind {
                return Err(BuildError::EmptyCategorical {
                    name: f.name.clone(),
                });
            }
        }

        // Group validation.
        let dim = self.factors.len();
        let mut factor_group_owner: Vec<Option<usize>> = vec![None; dim];
        for (gi, g) in self.groups.iter().enumerate() {
            if g.factor_indices.is_empty() {
                return Err(BuildError::EmptyGroup {
                    group: g.name.clone(),
                });
            }
            for &idx in &g.factor_indices {
                if idx >= dim {
                    return Err(BuildError::GroupIndexOutOfRange {
                        group: g.name.clone(),
                        index: idx,
                        dim,
                    });
                }
                if factor_group_owner[idx].is_some() {
                    return Err(BuildError::FactorInMultipleGroups { index: idx });
                }
                factor_group_owner[idx] = Some(gi);
            }
        }

        let groups = if self.groups.is_empty() {
            None
        } else {
            Some(self.groups)
        };

        Ok(Problem {
            factors: self.factors,
            groups,
        })
    }
}

/// Internal: per-variant parameter validation. Returns `Err(reason)`
/// on bad params; called by `ProblemBuilder::build`.
///
/// **`NaN`-safety note.** The checks below are written as `!(a < b)` /
/// `!(x > 0.0)` deliberately. The simplified forms (`a >= b`, `x <= 0.0`)
/// would let `NaN` slip through — `NaN >= b` is `false` in `IEEE-754`,
/// so `if a >= b` would *not* error on `NaN` parameters. The negated
/// form errors on `NaN` as well (`!(NaN < b)` is `!false` = `true`).
/// Rejecting `NaN` params at build time is required: a `Distribution`
/// carrying a `NaN` parameter would produce `NaN` samples in
/// `quantile`, polluting every downstream estimator.
#[allow(clippy::neg_cmp_op_on_partial_ord, clippy::nonminimal_bool)]
fn validate_distribution(d: &Distribution) -> Result<(), String> {
    match *d {
        Distribution::Uniform { lo, hi } => {
            if !(lo < hi) {
                return Err(format!("Uniform: lo ({lo}) must be < hi ({hi})"));
            }
        }
        Distribution::Normal { sigma, .. } => {
            if !(sigma > 0.0) {
                return Err(format!("Normal: sigma ({sigma}) must be > 0"));
            }
        }
        Distribution::LogNormal { sigma_log, .. } => {
            if !(sigma_log > 0.0) {
                return Err(format!("LogNormal: sigma_log ({sigma_log}) must be > 0"));
            }
        }
        Distribution::Triangular { lo, mode, hi } => {
            if !(lo < hi) {
                return Err(format!("Triangular: lo ({lo}) must be < hi ({hi})"));
            }
            if !(lo <= mode && mode <= hi) {
                return Err(format!(
                    "Triangular: mode ({mode}) must be in [lo ({lo}), hi ({hi})]"
                ));
            }
        }
        Distribution::Beta {
            alpha,
            beta,
            lo,
            hi,
        } => {
            if !(alpha > 0.0) {
                return Err(format!("Beta: alpha ({alpha}) must be > 0"));
            }
            if !(beta > 0.0) {
                return Err(format!("Beta: beta ({beta}) must be > 0"));
            }
            if !(lo < hi) {
                return Err(format!("Beta: lo ({lo}) must be < hi ({hi})"));
            }
        }
        Distribution::Gamma { shape, scale } => {
            if !(shape > 0.0) {
                return Err(format!("Gamma: shape ({shape}) must be > 0"));
            }
            if !(scale > 0.0) {
                return Err(format!("Gamma: scale ({scale}) must be > 0"));
            }
        }
        Distribution::Weibull { shape, scale } => {
            if !(shape > 0.0) {
                return Err(format!("Weibull: shape ({shape}) must be > 0"));
            }
            if !(scale > 0.0) {
                return Err(format!("Weibull: scale ({scale}) must be > 0"));
            }
        }
        Distribution::Exponential { lambda } => {
            if !(lambda > 0.0) {
                return Err(format!("Exponential: lambda ({lambda}) must be > 0"));
            }
        }
        Distribution::Bernoulli { p } => {
            if !(0.0..=1.0).contains(&p) {
                return Err(format!("Bernoulli: p ({p}) must be in [0, 1]"));
            }
        }
        Distribution::DiscreteUniform { lo, hi } => {
            if !(lo <= hi) {
                return Err(format!("DiscreteUniform: lo ({lo}) must be <= hi ({hi})"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn uniform(lo: f64, hi: f64) -> Distribution {
        Distribution::Uniform { lo, hi }
    }

    // ── ProblemBuilder happy paths ──────────────────────────────────

    #[test]
    fn build_single_factor() {
        let p = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        assert_eq!(p.dim(), 1);
        assert_eq!(p.factors()[0].name, "x");
        assert_eq!(p.factors()[0].kind, FactorKind::Continuous);
    }

    #[test]
    fn build_three_factors() {
        let p = ProblemBuilder::new()
            .factor("a", uniform(0.0, 1.0))
            .factor(
                "b",
                Distribution::Normal {
                    mu: 0.0,
                    sigma: 1.0,
                },
            )
            .factor("c", Distribution::Exponential { lambda: 1.0 })
            .build()
            .expect("builds");
        assert_eq!(p.dim(), 3);
        let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn build_with_kind_preserves_kind() {
        let p = ProblemBuilder::new()
            .factor_with_kind("d", uniform(0.0, 10.0), FactorKind::Discrete)
            .factor_with_kind(
                "c",
                Distribution::DiscreteUniform { lo: 0, hi: 4 },
                FactorKind::Categorical { n: 5 },
            )
            .factor_with_kind("b", Distribution::Bernoulli { p: 0.5 }, FactorKind::Boolean)
            .build()
            .expect("builds");
        assert_eq!(p.factors()[0].kind, FactorKind::Discrete);
        assert_eq!(p.factors()[1].kind, FactorKind::Categorical { n: 5 });
        assert_eq!(p.factors()[2].kind, FactorKind::Boolean);
    }

    #[test]
    fn factor_kind_default_is_continuous() {
        assert_eq!(FactorKind::default(), FactorKind::Continuous);
    }

    // ── ProblemBuilder error paths ──────────────────────────────────

    #[test]
    fn empty_builder_fails() {
        let err = ProblemBuilder::new().build().unwrap_err();
        assert_eq!(err, BuildError::Empty);
    }

    #[test]
    fn duplicate_name_fails() {
        let err = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .factor("x", uniform(2.0, 3.0))
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            BuildError::DuplicateName {
                name: "x".to_string()
            }
        );
    }

    #[test]
    fn invalid_uniform_lo_geq_hi_fails() {
        let err = ProblemBuilder::new()
            .factor("x", Distribution::Uniform { lo: 1.0, hi: 1.0 })
            .build()
            .unwrap_err();
        match err {
            BuildError::InvalidDistribution { name, .. } => assert_eq!(name, "x"),
            _ => panic!("wrong error variant: {err:?}"),
        }
    }

    #[test]
    fn invalid_normal_sigma_zero_fails() {
        let err = ProblemBuilder::new()
            .factor(
                "x",
                Distribution::Normal {
                    mu: 0.0,
                    sigma: 0.0,
                },
            )
            .build()
            .unwrap_err();
        match err {
            BuildError::InvalidDistribution { name, .. } => assert_eq!(name, "x"),
            _ => panic!("wrong error variant: {err:?}"),
        }
    }

    #[test]
    fn invalid_beta_alpha_zero_fails() {
        let err = ProblemBuilder::new()
            .factor(
                "x",
                Distribution::Beta {
                    alpha: 0.0,
                    beta: 1.0,
                    lo: 0.0,
                    hi: 1.0,
                },
            )
            .build()
            .unwrap_err();
        assert!(matches!(err, BuildError::InvalidDistribution { .. }));
    }

    #[test]
    fn invalid_triangular_mode_outside_range_fails() {
        let err = ProblemBuilder::new()
            .factor(
                "x",
                Distribution::Triangular {
                    lo: 0.0,
                    mode: 2.0,
                    hi: 1.0,
                },
            )
            .build()
            .unwrap_err();
        assert!(matches!(err, BuildError::InvalidDistribution { .. }));
    }

    #[test]
    fn invalid_bernoulli_p_above_one_fails() {
        let err = ProblemBuilder::new()
            .factor("x", Distribution::Bernoulli { p: 1.5 })
            .build()
            .unwrap_err();
        assert!(matches!(err, BuildError::InvalidDistribution { .. }));
    }

    #[test]
    fn invalid_exponential_lambda_zero_fails() {
        let err = ProblemBuilder::new()
            .factor("x", Distribution::Exponential { lambda: 0.0 })
            .build()
            .unwrap_err();
        assert!(matches!(err, BuildError::InvalidDistribution { .. }));
    }

    #[test]
    fn empty_categorical_fails() {
        let err = ProblemBuilder::new()
            .factor_with_kind(
                "x",
                Distribution::DiscreteUniform { lo: 0, hi: 0 },
                FactorKind::Categorical { n: 0 },
            )
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            BuildError::EmptyCategorical {
                name: "x".to_string()
            }
        );
    }

    // ── Problem methods ─────────────────────────────────────────────

    #[test]
    fn dim_matches_factor_count() {
        let p = ProblemBuilder::new()
            .factor("a", uniform(0.0, 1.0))
            .factor("b", uniform(0.0, 1.0))
            .factor("c", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        assert_eq!(p.dim(), 3);
    }

    #[test]
    fn factors_returns_in_insertion_order() {
        let p = ProblemBuilder::new()
            .factor("alpha", uniform(0.0, 1.0))
            .factor(
                "beta",
                Distribution::Normal {
                    mu: 0.0,
                    sigma: 1.0,
                },
            )
            .factor("gamma", Distribution::Exponential { lambda: 1.0 })
            .build()
            .expect("builds");
        let names: Vec<&str> = p.factors().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    // ── Content-addressing ──────────────────────────────────────────

    #[test]
    fn content_hash_is_stable_across_calls() {
        let p = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        let h1 = p.content_hash();
        let h2 = p.content_hash();
        let h3 = p.content_hash();
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn content_hash_equal_for_equal_problems() {
        let make = || {
            ProblemBuilder::new()
                .factor("x", uniform(0.0, 1.0))
                .factor(
                    "y",
                    Distribution::Normal {
                        mu: 0.0,
                        sigma: 2.0,
                    },
                )
                .build()
                .expect("builds")
        };
        assert_eq!(make().content_hash(), make().content_hash());
    }

    #[test]
    fn content_hash_distinct_for_different_distributions() {
        let p1 = ProblemBuilder::new()
            .factor("x", Distribution::Uniform { lo: 0.0, hi: 1.0 })
            .build()
            .expect("builds");
        let p2 = ProblemBuilder::new()
            .factor("x", Distribution::Uniform { lo: 0.0, hi: 2.0 })
            .build()
            .expect("builds");
        assert_ne!(p1.content_hash(), p2.content_hash());
    }

    #[test]
    fn content_hash_distinct_for_different_factor_names() {
        let p1 = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        let p2 = ProblemBuilder::new()
            .factor("y", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        assert_ne!(p1.content_hash(), p2.content_hash());
    }

    #[test]
    fn content_hash_distinct_for_factor_order_swap() {
        let p1 = ProblemBuilder::new()
            .factor("a", uniform(0.0, 1.0))
            .factor("b", uniform(2.0, 3.0))
            .build()
            .expect("builds");
        let p2 = ProblemBuilder::new()
            .factor("b", uniform(2.0, 3.0))
            .factor("a", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        // Factor order matters for indexing, so the hashes differ.
        assert_ne!(p1.content_hash(), p2.content_hash());
    }

    #[test]
    fn content_hash_distinct_for_different_kinds() {
        let p1 = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        let p2 = ProblemBuilder::new()
            .factor_with_kind("x", uniform(0.0, 1.0), FactorKind::Discrete)
            .build()
            .expect("builds");
        assert_ne!(p1.content_hash(), p2.content_hash());
    }

    #[test]
    fn content_hash_returns_thirty_two_bytes() {
        let p = ProblemBuilder::new()
            .factor("x", uniform(0.0, 1.0))
            .build()
            .expect("builds");
        let h = p.content_hash();
        assert_eq!(h.len(), 32);
    }

    // ── serde round-trip ────────────────────────────────────────────

    #[test]
    fn problem_serde_round_trip() {
        let p = ProblemBuilder::new()
            .factor("a", uniform(0.0, 1.0))
            .factor(
                "b",
                Distribution::Beta {
                    alpha: 2.0,
                    beta: 5.0,
                    lo: 0.0,
                    hi: 1.0,
                },
            )
            .factor_with_kind("c", Distribution::Bernoulli { p: 0.3 }, FactorKind::Boolean)
            .build()
            .expect("builds");
        let json = serde_json::to_string(&p).expect("serialize");
        let back: Problem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, p);
        assert_eq!(back.content_hash(), p.content_hash());
    }

    #[test]
    fn factor_kind_serde_round_trip() {
        let cases = vec![
            FactorKind::Continuous,
            FactorKind::Discrete,
            FactorKind::Categorical { n: 4 },
            FactorKind::Boolean,
        ];
        for k in cases {
            let json = serde_json::to_string(&k).expect("serialize");
            let back: FactorKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, k);
        }
    }

    // ── BuildError sanity ───────────────────────────────────────────

    #[test]
    fn build_error_implements_display_and_debug() {
        let err = BuildError::Empty;
        let _ = format!("{err}");
        let _ = format!("{err:?}");
        let err = BuildError::DuplicateName { name: "x".into() };
        let _ = format!("{err}");
    }

    // ── Group tests ────────────────────────────────────────────────

    #[test]
    fn grouped_problem_builds() {
        let p = ProblemBuilder::new()
            .factor("x1", uniform(0.0, 1.0))
            .factor("x2", uniform(0.0, 1.0))
            .factor("x3", uniform(0.0, 1.0))
            .group("shape", &[0, 1])
            .group("scale", &[2])
            .build()
            .unwrap();
        assert_eq!(p.groups.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn no_groups_gives_none() {
        let p = ProblemBuilder::new()
            .factor("x1", uniform(0.0, 1.0))
            .build()
            .unwrap();
        assert!(p.groups.is_none());
    }

    #[test]
    fn group_index_out_of_range_fails() {
        let result = ProblemBuilder::new()
            .factor("x1", uniform(0.0, 1.0))
            .group("bad", &[5])
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn factor_in_multiple_groups_fails() {
        let result = ProblemBuilder::new()
            .factor("x1", uniform(0.0, 1.0))
            .factor("x2", uniform(0.0, 1.0))
            .group("a", &[0])
            .group("b", &[0, 1])
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn empty_group_fails() {
        let result = ProblemBuilder::new()
            .factor("x1", uniform(0.0, 1.0))
            .group("empty", &[])
            .build();
        assert!(result.is_err());
    }
}
