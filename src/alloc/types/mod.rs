use core::cmp;

pub mod vec;
pub mod string;

pub enum GrowthStrategy {
  /// Grow the capacity of the [Vec] by exactly the amount that is needed
  Exact,
  /// Grow the internal of the [Vec] by 2 * the previous capacity, or the exact capacity that is required, whichever is larger.
  Exponential,
}

impl GrowthStrategy {
  pub fn calculate_new_capacity(&self, capacity: usize, additional: usize) -> Option<usize> {
    let min_capacity = capacity.checked_add(additional)?;
    match self {
      GrowthStrategy::Exact => Some(min_capacity),
      GrowthStrategy::Exponential => Some(cmp::max(capacity.checked_mul(2)?, min_capacity)),
    }
  }
}
