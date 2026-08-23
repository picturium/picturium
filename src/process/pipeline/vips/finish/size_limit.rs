use anyhow::Result;
use tracing::{debug, warn};

pub(crate) fn shrink_to_limit(
    quality: u8,
    first: Vec<u8>,
    limit: usize,
    threshold: f64,
    attempts: u8,
    min_quality: u8,
    mut encode: impl FnMut(u8) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let floor = limit - (limit as f64 * threshold.clamp(0.0, 1.0)) as usize;

    let mut low = min_quality;
    let mut high = quality.saturating_sub(1);

    let mut next = (quality as usize * limit / first.len().max(1)) as u8;

    let mut best: Option<Vec<u8>> = None;
    let mut smallest = first;

    for _ in 0..attempts {
        if low > high {
            break;
        }

        let quality = next.clamp(low, high);
        let buffer = encode(quality)?;
        debug!(
            "Size limit {limit} B: quality {quality}% produced {} B",
            buffer.len()
        );

        if buffer.len() <= limit {
            let in_band = buffer.len() >= floor;
            best = Some(buffer);

            if in_band {
                break;
            }

            low = quality + 1;
        } else {
            if buffer.len() < smallest.len() {
                smallest = buffer;
            }

            high = quality.saturating_sub(1);
        }

        next = low + high.saturating_sub(low) / 2;
    }

    Ok(best.unwrap_or_else(|| {
        warn!(
            "Could not fit output under limit=size:{limit} in {attempts} attempts, serving {} B",
            smallest.len()
        );
        smallest
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_QUALITY_FIXTURE: u8 = 10;

    fn encoder(bytes_per_quality: usize) -> impl FnMut(u8) -> Result<Vec<u8>> {
        move |quality| Ok(vec![0u8; quality as usize * bytes_per_quality])
    }

    #[test]
    fn returns_a_buffer_under_the_limit() {
        let result = shrink_to_limit(90, vec![0u8; 90_000], 40_000, 0.1, 3, MIN_QUALITY_FIXTURE, encoder(1000)).unwrap();
        assert!(result.len() <= 40_000, "got {} B", result.len());
    }

    #[test]
    fn lands_inside_the_acceptance_band() {
        let result = shrink_to_limit(90, vec![0u8; 90_000], 40_000, 0.1, 3, MIN_QUALITY_FIXTURE, encoder(1000)).unwrap();
        assert!(result.len() >= 36_000, "got {} B", result.len());
    }

    #[test]
    fn a_wide_band_is_satisfied_by_the_first_retry() {
        let mut calls = 0;
        let result = shrink_to_limit(90, vec![0u8; 90_000], 40_000, 0.5, 3, MIN_QUALITY_FIXTURE, |quality| {
            calls += 1;
            Ok(vec![0u8; quality as usize * 1000])
        })
        .unwrap();

        assert_eq!(calls, 1);
        assert!(result.len() <= 40_000);
    }

    #[test]
    fn serves_the_smallest_attempt_when_the_limit_is_unreachable() {
        let result = shrink_to_limit(90, vec![0u8; 90_000], 5_000, 0.1, 3, MIN_QUALITY_FIXTURE, encoder(1000)).unwrap();
        assert_eq!(result.len(), MIN_QUALITY_FIXTURE as usize * 1000);
    }

    #[test]
    fn never_encodes_below_the_quality_floor() {
        let mut lowest = u8::MAX;
        shrink_to_limit(90, vec![0u8; 90_000], 1, 0.1, 5, MIN_QUALITY_FIXTURE, |quality| {
            lowest = lowest.min(quality);
            Ok(vec![0u8; quality as usize * 1000])
        })
        .unwrap();

        assert_eq!(lowest, MIN_QUALITY_FIXTURE);
    }

    fn convex_encoder(quality: u8) -> Result<Vec<u8>> {
        Ok(vec![0u8; 10 * (quality as usize).pow(2)])
    }

    #[test]
    fn climbs_back_up_after_an_undershooting_first_guess() {
        let mut probes = Vec::new();
        let result = shrink_to_limit(90, convex_encoder(90).unwrap(), 40_000, 0.1, 5, MIN_QUALITY_FIXTURE, |quality| {
            probes.push(quality);
            convex_encoder(quality)
        })
        .unwrap();

        assert!(probes[0] < 50, "first probe was {:?}", probes[0]);
        assert!(result.len() <= 40_000, "got {} B", result.len());
        assert!(
            result.len() >= 36_000,
            "got {} B after probing {probes:?}",
            result.len()
        );
    }

    #[test]
    fn zero_attempts_returns_the_original_buffer() {
        let mut calls = 0;
        let result = shrink_to_limit(90, vec![0u8; 90_000], 40_000, 0.1, 0, MIN_QUALITY_FIXTURE, |quality| {
            calls += 1;
            Ok(vec![0u8; quality as usize * 1000])
        })
        .unwrap();

        assert_eq!(calls, 0);
        assert_eq!(result.len(), 90_000);
    }

    #[test]
    fn propagates_encoder_errors() {
        let result = shrink_to_limit(90, vec![0u8; 90_000], 40_000, 0.1, 3, MIN_QUALITY_FIXTURE, |_| {
            Err(anyhow::anyhow!("encoder exploded"))
        });

        assert!(result.is_err());
    }
}
