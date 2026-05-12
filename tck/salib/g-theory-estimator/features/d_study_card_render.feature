# G-theory D-study — operator-readable card surface
#
# Per bead `thunderdome-zqes`. The D-study amendment emits
# diagnostics named:
#
#   d_study_g_coefficient_n_i_<i>_n_r_<r>
#   d_study_phi_coefficient_n_i_<i>_n_r_<r>
#
# These flow into the card's per-row `diagnostics` map. Today they
# render as a generic semicolon-joined list — `name=value;
# name=value; ...` — which buries the reliability-counterfactual
# story under noise. An operator scanning the card has to grep
# through diagnostic names and reconstruct the (n_i, n_r) → (G, Phi)
# table by eye.
#
# Fix: surface a typed D-study sub-table on the card so the
# counterfactual is visible at a glance: one row per design cell,
# columns (n_i, n_r, G, Phi), grouped per-row beneath the row's
# verdicts. Both markdown and PDF renderers carry it. Companion
# of `thunderdome-r2bp` (D-study computation; closed).
#
# Mechanization:
#   `crates/thunderdome-sensitivity/src/card.rs` `#[cfg(test)]` mod.

Feature: D-study reliability counterfactuals render as a typed sub-table

  Scenario: parsing diagnostic names into typed entries
    Given a diagnostics map with both g_coefficient and phi_coefficient
      keys for design cells (n_i=2, n_r=2) and (n_i=4, n_r=2)
    When extract_d_study_entries runs
    Then it returns two entries sorted by (n_i, n_r)
    And the (2,2) entry has g_coefficient=0.55 and phi_coefficient=0.50
    And the (4,2) entry has g_coefficient=0.71 and phi_coefficient=0.66

  Scenario: non-D-study diagnostics are ignored by the parser
    # Plain `g_coefficient` (the observed-design coefficient, not a
    # D-study projection) sits next to the D-study keys in the same
    # diagnostics map. The parser must not pivot it into the table —
    # that diagnostic has its own surface in the row's existing
    # rendered diagnostics list.
    Given a diagnostics map containing `g_coefficient` and `r2_linear`
      but no `d_study_*` keys
    When extract_d_study_entries runs
    Then it returns an empty vector

  Scenario: half-entries surface what the estimator actually said
    # A future amendment could emit G without Phi (or vice versa)
    # for a particular design cell. The parser surfaces what the
    # estimator emitted instead of dropping the entry whole — the
    # render layer's `fmt_coef` substitutes "—" for the missing
    # side. Better to show the operator the partial truth than to
    # silently elide a real coefficient.
    Given a diagnostics map with `d_study_g_coefficient_n_i_3_n_r_2`
      but no matching phi_coefficient key
    When extract_d_study_entries runs
    Then it returns one entry with g_coefficient=Some and phi_coefficient=None
    And render_markdown emits the row as "| 3 | 2 | 0.6500 | — |"

  Scenario: malformed D-study names are silently skipped
    # D-study is additive context, not a load-bearing invariant. A
    # name like `d_study_g_coefficient_n_i_two_n_r_2` (non-integer)
    # or `d_study_g_coefficient` (no dimensions) should be skipped
    # without panicking — the renderer surfaces what it can.
    Given a diagnostics map with malformed `d_study_*` names
    When extract_d_study_entries runs
    Then it returns an empty vector

  # MARK: - Render surfaces

  Scenario: markdown card emits a D-study table when entries are present
    Given a card row whose diagnostics include D-study keys for
      (n_i=2, n_r=2) and (n_i=4, n_r=2)
    When render_markdown produces the card
    Then the output contains a "## D-study reliability counterfactuals" heading
    And the output contains a "| n_i | n_r | G | Phi |" table header
    And the (2, 2, G, Phi) row appears as "| 2 | 2 | 0.5500 | 0.5000 |"
    And the (4, 2, G, Phi) row appears as "| 4 | 2 | 0.7100 | 0.6600 |"

  Scenario: markdown card omits the section when no row carries d_study
    Given a card whose rows have no D-study diagnostics
    When render_markdown produces the card
    Then the output does NOT contain "D-study"

  Scenario: PDF render carries the D-study block per row
    # PDF mirrors the markdown surface. The D-study block sits below
    # the row's verdicts line (per-row, not per-output) — the
    # counterfactuals belong to a specific estimator-emission. PDF
    # render is plain typography (Helvetica/Courier; the LaTeX
    # upgrade is bead `thunderdome-z52f`).
    Given a card with a g_theory_pir row carrying two D-study cells
    When render_pdf produces the card
    Then the byte stream is non-empty and starts with the PDF magic
    And the PDF document contains the D-study cells
