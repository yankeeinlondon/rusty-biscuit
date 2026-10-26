//! Median and p95 of a set of timings.

use crate::baseline::Stats;

/// The median (mean of the middle two for an even count) and the
/// nearest-rank p95 of `samples_ms`, which must not be empty.
pub fn summarize(samples_ms: &[f64]) -> Stats {
    assert!(
        !samples_ms.is_empty(),
        "summarize needs at least one sample"
    );
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let median_ms = if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    };
    let rank = (sorted.len() as f64 * 0.95).ceil() as usize;
    Stats {
        median_ms,
        p95_ms: sorted[rank - 1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_count_median_is_the_middle_sample() {
        assert_eq!(
            summarize(&[3.0, 1.0, 2.0]),
            Stats {
                median_ms: 2.0,
                p95_ms: 3.0
            }
        );
    }

    #[test]
    fn even_count_median_averages_the_middle_pair() {
        assert_eq!(summarize(&[4.0, 1.0, 2.0, 3.0]).median_ms, 2.5);
    }

    #[test]
    fn p95_of_twenty_is_the_nineteenth_sample() {
        let samples: Vec<f64> = (1..=20).map(f64::from).collect();
        assert_eq!(summarize(&samples).p95_ms, 19.0);
    }

    #[test]
    fn single_sample_is_both_median_and_p95() {
        assert_eq!(
            summarize(&[7.0]),
            Stats {
                median_ms: 7.0,
                p95_ms: 7.0
            }
        );
    }
}
