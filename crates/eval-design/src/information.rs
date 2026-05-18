use crate::item_pool::ItemMetadata;

/// 2PL IRT probability: P(θ) = 1 / (1 + exp(-a(θ - b)))
#[must_use]
pub fn prob_correct(theta: f64, difficulty: f64, discrimination: f64) -> f64 {
    let z = discrimination * (theta - difficulty);
    1.0 / (1.0 + (-z).exp())
}

/// Fisher information for a 2PL item at ability θ:
/// I(θ) = a² * P(θ) * (1 - P(θ))
///
/// Maximized when θ = b (difficulty matches ability), where P = 0.5
/// and I = a²/4.
#[must_use]
pub fn fisher_information(theta: f64, item: &ItemMetadata) -> f64 {
    let p = prob_correct(theta, item.difficulty, item.discrimination);
    item.discrimination * item.discrimination * p * (1.0 - p)
}

/// Maximum Fisher information for an item (occurs at θ = b): a²/4
#[must_use]
pub fn max_fisher_information(item: &ItemMetadata) -> f64 {
    item.discrimination * item.discrimination * 0.25
}

/// Total Fisher information from a set of items at a given θ.
/// Under local independence, individual item informations sum.
#[must_use]
pub fn total_information(theta: f64, items: &[&ItemMetadata]) -> f64 {
    items.iter().map(|i| fisher_information(theta, i)).sum()
}

/// Minimum Fisher information across a grid of θ values.
/// Used for minimax selection: the worst-case information
/// this item set provides across all plausible ability levels.
#[must_use]
pub fn min_information_over_range(
    items: &[&ItemMetadata],
    theta_min: f64,
    theta_max: f64,
    n_grid: usize,
) -> f64 {
    if n_grid == 0 || items.is_empty() {
        return 0.0;
    }
    let step = if n_grid > 1 {
        (theta_max - theta_min) / (n_grid - 1) as f64
    } else {
        0.0
    };
    let mut min_info = f64::INFINITY;
    for i in 0..n_grid {
        let theta = theta_min + step * i as f64;
        let info = total_information(theta, items);
        if info < min_info {
            min_info = info;
        }
    }
    min_info
}

/// Standard error of the ability estimate given total information:
/// SE(θ) = 1 / sqrt(I(θ))
#[must_use]
pub fn standard_error(information: f64) -> f64 {
    if information <= 0.0 {
        return f64::INFINITY;
    }
    1.0 / information.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_pool::{ItemId, ItemMetadata};

    fn item(difficulty: f64, discrimination: f64) -> ItemMetadata {
        ItemMetadata::new(
            ItemId::new(format!("d{difficulty}_a{discrimination}")),
            difficulty,
            discrimination,
            "test".into(),
        )
    }

    #[test]
    fn prob_correct_at_difficulty_is_half() {
        let p = prob_correct(1.0, 1.0, 1.5);
        assert!((p - 0.5).abs() < 1e-10);
    }

    #[test]
    fn prob_correct_above_difficulty_exceeds_half() {
        let p = prob_correct(2.0, 1.0, 1.0);
        assert!(p > 0.5);
    }

    #[test]
    fn prob_correct_below_difficulty_below_half() {
        let p = prob_correct(0.0, 1.0, 1.0);
        assert!(p < 0.5);
    }

    #[test]
    fn fisher_information_maximized_at_difficulty() {
        let it = item(1.0, 2.0);
        let at_b = fisher_information(1.0, &it);
        let away = fisher_information(3.0, &it);
        assert!(at_b > away);
    }

    #[test]
    fn fisher_information_at_difficulty_equals_a_squared_over_four() {
        let it = item(0.0, 2.0);
        let info = fisher_information(0.0, &it);
        let expected = 2.0 * 2.0 * 0.25;
        assert!((info - expected).abs() < 1e-10);
    }

    #[test]
    fn max_fisher_matches_at_b() {
        let it = item(1.5, 1.8);
        let max_i = max_fisher_information(&it);
        let at_b = fisher_information(1.5, &it);
        assert!((max_i - at_b).abs() < 1e-10);
    }

    #[test]
    fn total_information_sums() {
        let i1 = item(0.0, 1.0);
        let i2 = item(0.0, 2.0);
        let items: Vec<&ItemMetadata> = vec![&i1, &i2];
        let total = total_information(0.0, &items);
        let sum = fisher_information(0.0, &i1) + fisher_information(0.0, &i2);
        assert!((total - sum).abs() < 1e-10);
    }

    #[test]
    fn min_information_finds_worst_theta() {
        let i1 = item(0.0, 2.0);
        let items: Vec<&ItemMetadata> = vec![&i1];
        let min_i = min_information_over_range(&items, -3.0, 3.0, 61);
        let at_extreme = fisher_information(3.0, &i1);
        assert!((min_i - at_extreme).abs() < 0.01);
    }

    #[test]
    fn standard_error_decreases_with_information() {
        let se_low = standard_error(1.0);
        let se_high = standard_error(4.0);
        assert!(se_high < se_low);
        assert!((se_low - 1.0).abs() < 1e-10);
        assert!((se_high - 0.5).abs() < 1e-10);
    }

    #[test]
    fn standard_error_infinity_for_zero_info() {
        assert!(standard_error(0.0).is_infinite());
    }
}
