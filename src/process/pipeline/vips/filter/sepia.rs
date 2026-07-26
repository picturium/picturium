use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage, VipsInterpretation};

/// Applies a CSS-equivalent `sepia()` filter.
///
/// The recombination matches the W3C `feColorMatrix type="matrix"` sepia matrix, so the
/// result matches the CSS `sepia()` filter. Like saturate and hue, this recombines the
/// colour bands per pixel via a single matrix multiply. The `amount` argument arrives
/// already-inverted from `FilterQueue::apply_sepia`: `1.0` leaves the image unchanged and
/// `0.0` applies full sepia toning.
pub fn apply(image: VipsImage, amount: f64) -> Result<VipsImage> {
    let bands = image.get_bands();

    let matrix = match (image.get_interpretation(), bands) {
        // Greyscale has no hue to tone.
        (VipsInterpretation::BlackWhite | VipsInterpretation::GREY16, _) => return Ok(image),
        (_, 1 | 2) => return Ok(image),

        // CMYK has no CSS equivalent: tone the C/M/Y inks and leave K, which carries
        // lightness, untouched. Best effort, not a CSS-faithful result.
        (VipsInterpretation::CMYK, 4) => colour_matrix_with_passthrough(amount),

        // RGB family. sRGB matches CSS exactly; a wider gamut diverges by the caller's own
        // colour-space choice. The fourth band, when present, is alpha.
        (_, 3) => colour_matrix(amount),
        (_, 4) => colour_matrix_with_passthrough(amount),

        // Unexpected band count: skip rather than risk a matrix/band mismatch.
        _ => return Ok(image),
    };

    let matrix = VipsImage::new_matrix(bands, bands, &matrix)
        .map_err(|e| anyhow::anyhow!("Failed to build sepia matrix: {:?}", e))?;

    image
        .recomb(matrix)
        .map_err(|e| anyhow::anyhow!("Failed to apply sepia: {:?}", e))
}

/// 3x3 W3C sepia `feColorMatrix` for the given `amount`. The coefficients are the spec's
/// `0.393/0.769/0.189 …` base toning matrix linearly interpolated toward the identity as
/// `amount` increases, so `amount == 0.0` is full sepia and `amount == 1.0` is a no-op.
/// Unlike the hue and saturate matrices, the rows do not sum to 1 — that is what produces
/// the warm, lifted toning.
fn colour_matrix(amount: f64) -> Vec<f64> {
    vec![
        0.393 + (amount * 0.607),
        0.769 - (amount * 0.769),
        0.189 - (amount * 0.189),
        0.349 - (amount * 0.349),
        0.686 + (amount * 0.314),
        0.168 - (amount * 0.168),
        0.272 - (amount * 0.272),
        0.534 - (amount * 0.534),
        0.131 + (amount * 0.869),
    ]
}

/// The 3x3 [`colour_matrix`] placed in the top-left of a 4x4 identity. The fourth band
/// (alpha for RGBA, K for CMYK) passes through unchanged.
fn colour_matrix_with_passthrough(amount: f64) -> Vec<f64> {
    let colour = colour_matrix(amount);
    let mut matrix = vec![0.0; 16];

    for row in 0..3 {
        for col in 0..3 {
            matrix[row * 4 + col] = colour[row * 3 + col];
        }
    }

    matrix[15] = 1.0;
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn full_sepia_at_amount_zero() {
        // amount == 0.0 is the W3C full-toning matrix, taken straight from the spec.
        let matrix = colour_matrix(0.0);
        let expected = [
            0.393, 0.769, 0.189, 0.349, 0.686, 0.168, 0.272, 0.534, 0.131,
        ];

        for (i, want) in expected.iter().enumerate() {
            assert!(
                approx(matrix[i], *want),
                "cell {i}: {} vs {}",
                matrix[i],
                want
            );
        }
    }

    #[test]
    fn identity_at_amount_one() {
        // amount == 1.0 (no sepia) collapses to the identity matrix.
        let matrix = colour_matrix(1.0);
        assert!(approx(matrix[0], 1.0) && approx(matrix[4], 1.0) && approx(matrix[8], 1.0));
        assert!(approx(matrix[1], 0.0) && approx(matrix[2], 0.0) && approx(matrix[3], 0.0));
        assert!(approx(matrix[5], 0.0) && approx(matrix[6], 0.0) && approx(matrix[7], 0.0));
    }

    #[test]
    fn linear_interpolation_at_half() {
        // amount == 0.5 should sit halfway between the full-toning and identity matrices.
        let matrix = colour_matrix(0.5);
        let expected = [
            0.393 + 0.5 * 0.607,
            0.769 - 0.5 * 0.769,
            0.189 - 0.5 * 0.189,
            0.349 - 0.5 * 0.349,
            0.686 + 0.5 * 0.314,
            0.168 - 0.5 * 0.168,
            0.272 - 0.5 * 0.272,
            0.534 - 0.5 * 0.534,
            0.131 + 0.5 * 0.869,
        ];

        for (i, want) in expected.iter().enumerate() {
            assert!(
                approx(matrix[i], *want),
                "cell {i}: {} vs {}",
                matrix[i],
                want
            );
        }
    }

    #[test]
    fn fourth_band_passes_through() {
        let matrix = colour_matrix_with_passthrough(0.0);
        assert!(approx(matrix[15], 1.0));

        for i in 0..3 {
            assert!(approx(matrix[3 * 4 + i], 0.0));
            assert!(approx(matrix[i * 4 + 3], 0.0));
        }
    }
}
