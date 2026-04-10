// @reference: https://d3js.org/d3-scale/point

use itertools::Itertools;

use super::Scale;

/// Point scale maps discrete domain values to continuous range positions.
///
/// Points are evenly distributed across the range, with the first and last points
/// aligned to the range boundaries.
#[derive(Clone)]
pub struct ScalePoint<T> {
    domain: Vec<T>,
    range_start: f32,
    range_tick: f32,
}

impl<T> ScalePoint<T>
where
    T: PartialEq,
{
    /// Creates a new point scale with the given domain and range.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);
    /// assert_eq!(scale.tick(&1), Some(0.));
    /// assert_eq!(scale.tick(&2), Some(50.));
    /// assert_eq!(scale.tick(&3), Some(100.));
    /// ```
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let len = domain.len();
        let (range_start, range_tick) = if len == 0 {
            (0., 0.)
        } else {
            let (min, max) = range
                .iter()
                .minmax()
                .into_option()
                .map_or((0., 0.), |(min, max)| (*min, *max));

            let range_diff = max - min;

            if len == 1 {
                (min, range_diff)
            } else {
                (min, range_diff / (len - 1) as f32)
            }
        };

        Self {
            domain,
            range_start,
            range_tick,
        }
    }
}

impl<T> Scale<T> for ScalePoint<T>
where
    T: PartialEq,
{
    fn tick(&self, value: &T) -> Option<f32> {
        let index = self.domain.iter().position(|v| v == value)?;

        if self.domain.len() == 1 {
            Some(self.range_start + self.range_tick / 2.)
        } else {
            Some(self.range_start + index as f32 * self.range_tick)
        }
    }

    fn least_index(&self, tick: f32) -> usize {
        if self.domain.is_empty() {
            return 0;
        }

        if self.range_tick == 0. {
            return 0;
        }

        let normalized_tick = tick - self.range_start;
        let index = (normalized_tick / self.range_tick).round() as usize;
        index.min(self.domain.len() - 1)
    }
}
