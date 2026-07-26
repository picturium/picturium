use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage, VipsInterpretation};

/// Rec.709 luma weights from the CSS / SVG `feColorMatrix type="saturate"` spec.
/// Using these exact constants makes the result match the CSS `saturate()` filter.
const LUMA: [f64; 3] = [0.213, 0.715, 0.072];

/// Applies a CSS-equivalent `saturate()` filter.
///
/// `factor == 1.0` leaves the image unchanged, `0.0` produces greyscale, and values
/// above `1.0` oversaturate. Like CSS engines, this recombines the colour bands per
/// pixel via a single matrix multiply.
pub fn apply(image: VipsImage, factor: f64) -> Result<VipsImage> {
    let bands = image.get_bands();

    let matrix = match (image.get_interpretation(), bands) {
        // Greyscale has nothing to saturate.
        (VipsInterpretation::BlackWhite | VipsInterpretation::GREY16, _) => return Ok(image),
        (_, 1 | 2) => return Ok(image),

        // CMYK has no CSS equivalent: saturate the C/M/Y inks and leave K, which
        // carries lightness, untouched. Best effort, not a CSS-faithful result.
        (VipsInterpretation::CMYK, 4) => colour_matrix_with_passthrough(factor),

        // RGB family. sRGB matches CSS exactly; a wider gamut diverges by the
        // caller's own colour-space choice. The fourth band, when present, is alpha.
        (_, 3) => colour_matrix(factor),
        (_, 4) => colour_matrix_with_passthrough(factor),

        // Unexpected band count: skip rather than risk a matrix/band mismatch.
        _ => return Ok(image),
    };

    let matrix = VipsImage::new_matrix(bands, bands, &matrix)
        .map_err(|e| anyhow::anyhow!("Failed to build saturate matrix: {:?}", e))?;

    image
        .recomb(matrix)
        .map_err(|e| anyhow::anyhow!("Failed to apply saturate: {:?}", e))
}

/// 3x3 saturation matrix: `M[row][col] = luma[col] * (1 - factor)`, plus `factor` on
/// the diagonal. Each row sums to 1, so neutral greys are preserved.
fn colour_matrix(factor: f64) -> Vec<f64> {
    let mut matrix = vec![0.0; 9];

    for row in 0..3 {
        for col in 0..3 {
            let diagonal = if row == col { factor } else { 0.0 };
            matrix[row * 3 + col] = LUMA[col] * (1.0 - factor) + diagonal;
        }
    }

    matrix
}

/// The 3x3 [`colour_matrix`] placed in the top-left of a 4x4 identity. The fourth
/// band (alpha for RGBA, K for CMYK) passes through unchanged.
fn colour_matrix_with_passthrough(factor: f64) -> Vec<f64> {
    let colour = colour_matrix(factor);
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
        (a - b).abs() < 1e-9
    }

    #[test]
    fn unchanged_at_factor_one() {
        let matrix = colour_matrix(1.0);
        assert!(approx(matrix[0], 1.0) && approx(matrix[4], 1.0) && approx(matrix[8], 1.0));
        assert!(approx(matrix[1], 0.0) && approx(matrix[2], 0.0) && approx(matrix[3], 0.0));
        assert!(approx(matrix[5], 0.0) && approx(matrix[6], 0.0) && approx(matrix[7], 0.0));
    }

    #[test]
    fn greyscale_at_factor_zero() {
        let matrix = colour_matrix(0.0);

        for row in 0..3 {
            assert!(approx(matrix[row * 3], LUMA[0]));
            assert!(approx(matrix[row * 3 + 1], LUMA[1]));
            assert!(approx(matrix[row * 3 + 2], LUMA[2]));
        }
    }

    #[test]
    fn rows_sum_to_one() {
        for factor in [0.0, 0.5, 1.0, 2.0] {
            let matrix = colour_matrix(factor);

            for row in 0..3 {
                let sum: f64 = matrix[row * 3..row * 3 + 3].iter().sum();
                assert!(approx(sum, 1.0), "row {row} sum {sum} for factor {factor}");
            }
        }
    }

    #[test]
    fn fourth_band_passes_through() {
        let matrix = colour_matrix_with_passthrough(2.0);
        assert!(approx(matrix[15], 1.0));

        for i in 0..3 {
            assert!(approx(matrix[3 * 4 + i], 0.0));
            assert!(approx(matrix[i * 4 + 3], 0.0));
        }
    }
}
