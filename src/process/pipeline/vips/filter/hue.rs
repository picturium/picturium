use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage, VipsInterpretation};

/// Applies a CSS-equivalent `hue-rotate()` filter.
///
/// `degrees == 0` (and any multiple of `360`) leaves the image unchanged. The rotation
/// matches the W3C `feColorMatrix type="hueRotate"` matrix, so the result matches the CSS
/// `hue-rotate()` filter. Like saturate, this recombines the colour bands per pixel via a
/// single matrix multiply; it is a fixed colour-axis rotation, not an HSL hue shift.
pub fn apply(image: VipsImage, degrees: f64) -> Result<VipsImage> {
    let bands = image.get_bands();

    let matrix = match (image.get_interpretation(), bands) {
        // Greyscale has no hue to rotate.
        (VipsInterpretation::BlackWhite | VipsInterpretation::GREY16, _) => return Ok(image),
        (_, 1 | 2) => return Ok(image),

        // CMYK has no CSS equivalent: rotate the C/M/Y inks and leave K, which carries
        // lightness, untouched. Best effort, not a CSS-faithful result.
        (VipsInterpretation::CMYK, 4) => colour_matrix_with_passthrough(degrees),

        // RGB family. sRGB matches CSS exactly; a wider gamut diverges by the caller's own
        // colour-space choice. The fourth band, when present, is alpha.
        (_, 3) => colour_matrix(degrees),
        (_, 4) => colour_matrix_with_passthrough(degrees),

        // Unexpected band count: skip rather than risk a matrix/band mismatch.
        _ => return Ok(image),
    };

    let matrix = VipsImage::new_matrix(bands, bands, &matrix)
        .map_err(|e| anyhow::anyhow!("Failed to build hue matrix: {:?}", e))?;

    image
        .recomb(matrix)
        .map_err(|e| anyhow::anyhow!("Failed to apply hue: {:?}", e))
}

/// 3x3 W3C `hueRotate` matrix for the given angle. The `0.213/0.715/0.072` terms are the
/// Rec.709 luma weights (matching saturate's `LUMA`); the remaining coefficients are the
/// spec-derived combinations, so they are written inline rather than factored out. Each row
/// sums to 1, so neutral greys are preserved.
fn colour_matrix(degrees: f64) -> Vec<f64> {
    let (sin, cos) = degrees.to_radians().sin_cos();

    vec![
        0.213 + 0.787 * cos - 0.213 * sin,
        0.715 - 0.715 * cos - 0.715 * sin,
        0.072 - 0.072 * cos + 0.928 * sin,
        0.213 - 0.213 * cos + 0.143 * sin,
        0.715 + 0.285 * cos + 0.140 * sin,
        0.072 - 0.072 * cos - 0.283 * sin,
        0.213 - 0.213 * cos - 0.787 * sin,
        0.715 - 0.715 * cos + 0.715 * sin,
        0.072 + 0.928 * cos + 0.072 * sin,
    ]
}

/// The 3x3 [`colour_matrix`] placed in the top-left of a 4x4 identity. The fourth band
/// (alpha for RGBA, K for CMYK) passes through unchanged.
fn colour_matrix_with_passthrough(degrees: f64) -> Vec<f64> {
    let colour = colour_matrix(degrees);
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
    fn identity_at_zero_degrees() {
        let matrix = colour_matrix(0.0);
        assert!(approx(matrix[0], 1.0) && approx(matrix[4], 1.0) && approx(matrix[8], 1.0));
        assert!(approx(matrix[1], 0.0) && approx(matrix[2], 0.0) && approx(matrix[3], 0.0));
        assert!(approx(matrix[5], 0.0) && approx(matrix[6], 0.0) && approx(matrix[7], 0.0));
    }

    #[test]
    fn rows_sum_to_one() {
        for degrees in [0.0, 30.0, 90.0, 180.0, 270.0, 360.0] {
            let matrix = colour_matrix(degrees);

            for row in 0..3 {
                let sum: f64 = matrix[row * 3..row * 3 + 3].iter().sum();
                assert!(
                    approx(sum, 1.0),
                    "row {row} sum {sum} for {degrees} degrees"
                );
            }
        }
    }

    #[test]
    fn full_rotation_is_identity() {
        let rotated = colour_matrix(360.0);
        let identity = colour_matrix(0.0);

        for i in 0..9 {
            assert!(
                approx(rotated[i], identity[i]),
                "cell {i}: {} vs {}",
                rotated[i],
                identity[i]
            );
        }
    }

    #[test]
    fn known_angle_sanity() {
        // 180 degrees: cos = -1, sin = 0.
        let matrix = colour_matrix(180.0);
        assert!(approx(matrix[0], 0.213 - 0.787)); // row0col0: 0.213 + 0.787*(-1) = -0.574
        assert!(approx(matrix[1], 0.715 + 0.715)); // row0col1: 0.715 - 0.715*(-1) = 1.43
        assert!(approx(matrix[4], 0.715 - 0.285)); // row1col1: 0.715 + 0.285*(-1) = 0.43
    }

    #[test]
    fn fourth_band_passes_through() {
        let matrix = colour_matrix_with_passthrough(90.0);
        assert!(approx(matrix[15], 1.0));

        for i in 0..3 {
            assert!(approx(matrix[3 * 4 + i], 0.0));
            assert!(approx(matrix[i * 4 + 3], 0.0));
        }
    }
}
