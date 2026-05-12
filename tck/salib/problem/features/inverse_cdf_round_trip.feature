# Inverse-CDF (`quantile`) properties for the closed `Distribution`
# enum. The single-direction-only contract: `quantile(u)` for
# `u ∈ [0, 1]`. Out-of-range u saturates to support boundaries.
#
# Properties pinned per scenario:
#
# 1. Boundary semantics. `quantile(0)` returns the lower support and
#    `quantile(1)` the upper support, for distributions with finite
#    support. For unbounded support (Normal, LogNormal, Gamma,
#    Weibull, Exponential), the behavior is documented per-test.
#
# 2. Monotonicity. `quantile(u)` is non-decreasing in `u` across
#    `[0, 1]` for every distribution. (The discrete distributions
#    have flat steps; the continuous ones strictly increase except
#    where probability mass is zero.)
#
# 3. Known-quantile-points. Closed-form evaluations agree with the
#    impl: `Normal::quantile(0.5) == mu`, `Uniform(lo, hi)::quantile(0.5)
#    == (lo+hi)/2`, etc.
#
# 4. Special-case identities. `Beta(1, 1) ≡ Uniform`, `Weibull(1, λ⁻¹)
#    ≡ Exponential(λ)`, `Gamma(1, λ⁻¹) ≡ Exponential(λ)`. Distinct
#    Distribution variants that produce equivalent distributions
#    must give bit-equal-or-near-equal quantiles.
#
# Provenance: `rust_salib_crate_research.md` § 3.1 + per-distribution
# closed-form references in
# `decisions/2026-04-28-saltelli-problem-shape.md`.
#
# Mechanized: `crates/saltelli-core/tests/distribution_tck.rs`.

Feature: Distribution — inverse-CDF properties

  # ── Boundary semantics ──────────────────────────────────────────

  Scenario: Uniform quantile maps unit interval linearly
    Given a Uniform distribution on [10, 30]
    Then quantile(0.0) is 10.0
    And quantile(0.5) is 20.0
    And quantile(1.0) is 30.0

  Scenario: Triangular quantile hits its support boundaries
    Given a Triangular distribution on [-1, 0, 1]
    Then quantile(0.0) is -1.0
    And quantile(1.0) is 1.0

  Scenario: Beta quantile on a non-unit interval hits boundaries
    Given a Beta(2, 5) distribution on [0.5, 1.5]
    Then quantile(0.0) is approximately 0.5 within 1e-9
    And quantile(1.0) is approximately 1.5 within 1e-9

  # ── Known-quantile-points ──────────────────────────────────────

  Scenario: Normal median is mu
    Given a Normal distribution with mu 7.0 and sigma 2.0
    Then quantile(0.5) is approximately 7.0 within 1e-9

  Scenario: Symmetric Beta median is the midpoint
    Given a Beta(3, 3) distribution on [0, 1]
    Then quantile(0.5) is approximately 0.5 within 1e-9

  Scenario: LogNormal median is exp(mu_log)
    Given a LogNormal distribution with mu_log 1.0 and sigma_log 0.5
    Then quantile(0.5) is approximately 2.718281828 within 1e-6

  # ── Monotonicity ────────────────────────────────────────────────

  Scenario: Uniform quantile is monotone non-decreasing
    Given a Uniform distribution on [0, 1]
    Then quantile is monotone non-decreasing across [0, 1]

  Scenario: Normal quantile is monotone non-decreasing
    Given a Normal distribution with mu 0.0 and sigma 1.0
    Then quantile is monotone non-decreasing across [0, 1]

  Scenario: Beta quantile is monotone non-decreasing
    Given a Beta(2, 5) distribution on [0, 1]
    Then quantile is monotone non-decreasing across [0, 1]

  Scenario: Bernoulli quantile is monotone non-decreasing
    Given a Bernoulli distribution with p 0.4
    Then quantile is monotone non-decreasing across [0, 1]

  # ── Special-case identities ─────────────────────────────────────

  Scenario: Beta(1, 1) is Uniform
    Given a Beta(1, 1) distribution on [0, 1] and a Uniform on [0, 1]
    Then their quantiles agree at 5 sample points within 1e-9

  Scenario: Weibull(1, scale) is Exponential(1/scale)
    Given a Weibull(1, 4) distribution and an Exponential(0.25)
    Then their quantiles agree at 5 sample points within 1e-12

  Scenario: Gamma(1, scale) is Exponential(1/scale)
    Given a Gamma(1, 2) distribution and an Exponential(0.5)
    Then their quantiles agree at 5 sample points within 1e-7

  # ── Out-of-range u saturates ────────────────────────────────────

  Scenario: out-of-range u below zero saturates to lower support
    Given a Uniform distribution on [10, 30]
    Then quantile(-0.5) is 10.0

  Scenario: out-of-range u above one saturates to upper support
    Given a Uniform distribution on [10, 30]
    Then quantile(1.5) is 30.0
