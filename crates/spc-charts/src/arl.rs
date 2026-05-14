#![allow(clippy::cast_precision_loss)]

use crate::types::SpcError;

/// Compute ARL for a two-sided CUSUM chart via Markov chain
/// discretization (Brook & Evans 1972).
///
/// Combines the upper (C⁺) and lower (C⁻) one-sided CUSUMs:
/// `ARL = 1 / (1/ARL_upper + 1/ARL_lower)`.
///
/// `k`: reference value (allowance) in σ units.
/// `h`: decision interval in σ units.
/// `shift`: mean shift in σ units (0 for ARL₀).
/// `n_states`: discretization resolution (default 200).
pub fn cusum_arl(k: f64, h: f64, shift: f64, n_states: usize) -> Result<f64, SpcError> {
    // Upper CUSUM detects positive shifts: effective shift = +δ
    let arl_upper = cusum_arl_one_sided(k, h, shift, n_states)?;
    // Lower CUSUM detects negative shifts: effective shift = -δ
    let arl_lower = cusum_arl_one_sided(k, h, -shift, n_states)?;
    // Two-sided ARL via independence of C⁺ and C⁻
    Ok(1.0 / (1.0 / arl_upper + 1.0 / arl_lower))
}

/// Compute ARL for a one-sided (upper) CUSUM chart via Markov chain
/// discretization (Brook & Evans 1972).
///
/// `k`: reference value (allowance) in σ units.
/// `h`: decision interval in σ units.
/// `shift`: mean shift in σ units (0 for ARL₀).
/// `n_states`: discretization resolution.
fn cusum_arl_one_sided(k: f64, h: f64, shift: f64, n_states: usize) -> Result<f64, SpcError> {
    if k <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "k",
            value: k,
        });
    }
    if h <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "h",
            value: h,
        });
    }
    let n = n_states;
    let delta = h / n as f64;

    // State midpoints: s_i = (i + 0.5) * delta for i in 0..n
    // Transition: from state s_i, new value = max(0, s_i + Z - k)
    // where Z ~ N(shift, 1). Need P(new value falls in state j).
    //
    // Build (I - Q) matrix where Q[i][j] = P(transition from i to j
    // without signaling). Signal = new value > h.

    let mut mat = nalgebra::DMatrix::<f64>::zeros(n, n);
    let rhs = nalgebra::DVector::<f64>::from_element(n, 1.0);

    for i in 0..n {
        let s_i = (i as f64 + 0.5) * delta;
        for j in 0..n {
            let lo = j as f64 * delta;
            let hi = (j + 1) as f64 * delta;
            // P(lo ≤ max(0, s_i + Z - k) < hi)
            // For j == 0: includes the absorbing-at-zero region:
            // P(s_i + Z - k ≤ 0) + P(0 < s_i + Z - k < delta)
            let z_lo = lo - s_i + k - shift;
            let z_hi = hi - s_i + k - shift;
            let p = if j == 0 {
                phi(z_hi)
            } else {
                phi(z_hi) - phi(z_lo)
            };
            mat[(i, j)] = -p;
        }
        mat[(i, i)] += 1.0;
    }

    // Solve (I - Q) * arl_vec = 1
    let decomp = mat.lu();
    let arl_vec = decomp.solve(&rhs).ok_or(SpcError::SingularArlMatrix(h))?;

    // ARL starting from state 0 (CUSUM starts at 0).
    Ok(arl_vec[0])
}

/// Compute ARL for an EWMA chart via Markov chain discretization
/// (Lucas & Saccucci 1990).
///
/// `lambda`: smoothing constant.
/// `l_sigma`: control limit width in σ.
/// `shift`: mean shift in σ units (0 for ARL₀).
/// `n_states`: discretization resolution (default 200).
pub fn ewma_arl(lambda: f64, l_sigma: f64, shift: f64, n_states: usize) -> Result<f64, SpcError> {
    if lambda <= 0.0 || lambda > 1.0 {
        return Err(SpcError::InvalidLambda(lambda));
    }
    if l_sigma <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "l_sigma",
            value: l_sigma,
        });
    }
    let n = n_states;

    // Asymptotic EWMA control limit width.
    let sigma_z = (lambda / (2.0 - lambda)).sqrt();
    let ucl = l_sigma * sigma_z;
    let lcl = -ucl;
    let range = ucl - lcl;
    let delta = range / n as f64;

    let mut mat = nalgebra::DMatrix::<f64>::zeros(n, n);
    let rhs = nalgebra::DVector::<f64>::from_element(n, 1.0);

    for i in 0..n {
        let z_i = lcl + (i as f64 + 0.5) * delta;
        for j in 0..n {
            let z_lo = lcl + j as f64 * delta;
            let z_hi = z_lo + delta;
            // EWMA update: Z_new = lambda*X + (1-lambda)*z_i
            // X needed for Z_new in [z_lo, z_hi]:
            // x_lo = (z_lo - (1-lambda)*z_i) / lambda
            // x_hi = (z_hi - (1-lambda)*z_i) / lambda
            let x_lo = (z_lo - (1.0 - lambda) * z_i) / lambda - shift;
            let x_hi = (z_hi - (1.0 - lambda) * z_i) / lambda - shift;
            let p = phi(x_hi) - phi(x_lo);
            mat[(i, j)] = -p;
        }
        mat[(i, i)] += 1.0;
    }

    let decomp = mat.lu();
    let arl_vec = decomp
        .solve(&rhs)
        .ok_or(SpcError::SingularArlMatrix(l_sigma))?;

    // ARL starting from the center state (EWMA starts at μ₀).
    let center_state = n / 2;
    Ok(arl_vec[center_state])
}

/// Standard normal CDF via the error function.
fn phi(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation (Abramowitz & Stegun 7.1.26, max error 1.5e-7).
fn erf(x: f64) -> f64 {
    let a1 = 0.254_829_592;
    let a2 = -0.284_496_736;
    let a3 = 1.421_413_741;
    let a4 = -1.453_152_027;
    let a5 = 1.061_405_429;
    let p = 0.327_591_1;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}
