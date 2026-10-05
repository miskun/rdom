//! Whole-cell shares of a free space (DIVERGENCES §1, "alignment free
//! space is shared in whole cells"): factors that are CSS `<number>`s
//! held in `f32`, and the rolling floor that hands out shares summing to
//! the whole cells of the space — two equal shares of 31 cells are 15 +
//! 16. Flex layout's §9.7 shares and grid layout's `fr` tracks and
//! spanning-item distributions take their cells here.

/// Relative tolerance for arithmetic on flex factors.
///
/// Factors are CSS `<number>`s parsed into `f32` (24-bit mantissa), so
/// a decimal factor carries a relative error of up to 2⁻²⁴ and a sum or
/// product of them a few times that: `0.1 + 0.2 + 0.7` is
/// 0.99999999255 once widened to `f64`, and `80 × (0.2 + 0.7)` is
/// 71.9999999. Comparisons against one and the floors that turn shares
/// into cells treat values within this tolerance as equal.
///
/// Why a tolerance and not an `f32` sum: summing in `f32` only moves
/// the rounding (it happens to give exactly 1.0 for `0.1 + 0.2 + 0.7`,
/// but `10 × 0.1` gives 1.0000001), and the floors still see products
/// a hair below an integer. The tolerance cannot misfire on a real
/// fraction: four `f32` epsilons (≈ 4.8e-7) of the largest main size
/// (`u16::MAX` cells) is 0.03 of a cell, and a factor sum within it of
/// one is one at the precision an `f32` factor holds.
pub(super) const FACTOR_TOLERANCE: f64 = 4.0 * f32::EPSILON as f64;

/// "The sum of the flex factors is less than one" (CSS Flexbox §9.7
/// step 4.b, CSS Grid 2 §11.7.1 step 2), with the factors' `f32`
/// rounding forgiven ([`FACTOR_TOLERANCE`]).
pub(super) fn sums_below_one(sum: f64) -> bool {
    sum < 1.0 - FACTOR_TOLERANCE
}

/// Floor a rolling share target to whole cells, forgiving the factors'
/// `f32` rounding ([`FACTOR_TOLERANCE`]) so `71.9999999` is 72.
pub(super) fn floor_cells(x: f64) -> u32 {
    (x + x.abs() * FACTOR_TOLERANCE + 1e-9)
        .floor()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Shares of `amount` by weight, in whole cells: the share of the
/// weight that brings the running total to `w` is the floor of
/// `amount × w / total` less the cells already handed out, so the
/// shares sum to the floor of `amount` and each is within a cell of its
/// exact value.
pub(super) struct Rolling {
    amount: f64,
    total: f64,
    weight: f64,
    handed: u32,
}

impl Rolling {
    /// Shares of `amount` over weights summing to `total`.
    pub(super) fn new(amount: f64, total: f64) -> Self {
        Self {
            amount,
            total,
            weight: 0.0,
            handed: 0,
        }
    }

    /// The next share, of `weight`. Zero when the total weight is not
    /// positive.
    pub(super) fn share(&mut self, weight: f64) -> u32 {
        if self.total <= 0.0 {
            return 0;
        }
        self.weight += weight;
        let to = floor_cells(self.amount * self.weight / self.total);
        let share = to.saturating_sub(self.handed);
        self.handed = to.max(self.handed);
        share
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DIVERGENCES §1: two equal shares of 31 cells are 15 + 16, three
    /// of 11 are 3 + 4 + 4 — the remainder cells go to the later shares,
    /// the floor of each running total.
    #[test]
    fn rolling_shares_sum_to_the_whole_cells() {
        let mut r = Rolling::new(31.0, 2.0);
        assert_eq!([r.share(1.0), r.share(1.0)], [15, 16]);
        let mut r = Rolling::new(11.0, 3.0);
        assert_eq!([r.share(1.0), r.share(1.0), r.share(1.0)], [3, 4, 4]);
        let mut r = Rolling::new(80.0, 1.0);
        assert_eq!([r.share(0.2), r.share(0.7)], [16, 56]);
        assert_eq!(Rolling::new(5.0, 0.0).share(1.0), 0);
    }
}
