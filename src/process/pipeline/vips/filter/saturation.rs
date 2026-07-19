use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage, VipsInterpretation};

/// Rec.709 luma weights as defined by the CSS / SVG `feColorMatrix type="saturate"`
/// specification. These are the exact constants browsers use, so applying the matrix
/// on sRGB pixels byte-matches the CSS `saturate()` filter.
const LUMA: [f64; 3] = [0.213, 0.715, 0.072];

/// Applies a CSS-equivalent `saturate()` filter with factor `value`.
///
/// `value == 1.0` is the identity, `0.0` produces greyscale, and values above `1.0`
/// oversaturate. The operation is a per-pixel linear recombination of the colour
/// bands, matching how CSS engines implement the filter.
pub fn apply(image: VipsImage, value: f64) -> Result<VipsImage> {
    let bands = image.get_bands();
    let interpretation = image.get_interpretation();

    let matrix = match (interpretation, bands) {
        // Greyscale: saturation is a no-op, nothing to recombine.
        (VipsInterpretation::BlackWhite, _) | (VipsInterpretation::GREY16, _) => return Ok(image),
        (_, 1) | (_, 2) => return Ok(image),

        // CMYK is a subtractive space with no CSS equivalent. We saturate the C/M/Y
        // inks toward neutral and leave K (which carries lightness) untouched. This is
        // a best-effort choice, not a CSS-faithful result.
        (VipsInterpretation::CMYK, 4) => cmyk_matrix(value),

        // RGB family (sRGB matches CSS exactly; wide-gamut output diverges by the
        // caller's own colour-space choice). 3 bands plain, 4 bands carry alpha.
        (_, 3) => rgb_matrix(value),
        (_, 4) => rgba_matrix(value),

        // Anything else (unexpected band count) is left untouched rather than risk a
        // matrix/​band mismatch in `recomb`.
        _ => return Ok(image),
    };

    let n = bands;
    let matrix_image = VipsImage::new_matrix(n, n, &matrix)
        .map_err(|e| anyhow::anyhow!("Failed to build saturation matrix: {:?}", e))?;

    image.recomb(&matrix_image)
        .map_err(|e| anyhow::anyhow!("Failed to apply saturation: {:?}", e))
}

/// `M[i][j] = luma[j] * (1 - s) + (i == j ? s : 0)` over the three RGB bands.
fn rgb_matrix(s: f64) -> Vec<f64> {
    let mut m = vec![0.0; 9];
    for row in 0..3 {
        for col in 0..3 {
            m[row * 3 + col] = LUMA[col] * (1.0 - s) + if row == col { s } else { 0.0 };
        }
    }
    m
}

/// RGB saturation matrix embedded in a 4x4 with an identity alpha row/column.
fn rgba_matrix(s: f64) -> Vec<f64> {
    let rgb = rgb_matrix(s);
    let mut m = vec![0.0; 16];
    for row in 0..3 {
        for col in 0..3 {
            m[row * 4 + col] = rgb[row * 3 + col];
        }
    }
    m[15] = 1.0; // alpha passthrough
    m
}

/// CMYK saturation matrix: C/M/Y get the saturation recombination, K is passed
/// through unchanged. No CSS equivalent exists for CMYK input.
fn cmyk_matrix(s: f64) -> Vec<f64> {
    let cmy = rgb_matrix(s);
    let mut m = vec![0.0; 16];
    for row in 0..3 {
        for col in 0..3 {
            m[row * 4 + col] = cmy[row * 3 + col];
        }
    }
    m[15] = 1.0; // K passthrough
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn identity_at_one() {
        let m = rgb_matrix(1.0);
        // s == 1 must be the identity matrix.
        assert!(approx(m[0], 1.0) && approx(m[4], 1.0) && approx(m[8], 1.0));
        assert!(approx(m[1], 0.0) && approx(m[2], 0.0) && approx(m[3], 0.0));
        assert!(approx(m[5], 0.0) && approx(m[6], 0.0) && approx(m[7], 0.0));
    }

    #[test]
    fn greyscale_at_zero() {
        let m = rgb_matrix(0.0);
        // s == 0 collapses every row to the luma weights.
        for row in 0..3 {
            assert!(approx(m[row * 3], LUMA[0]));
            assert!(approx(m[row * 3 + 1], LUMA[1]));
            assert!(approx(m[row * 3 + 2], LUMA[2]));
        }
    }

    #[test]
    fn rows_sum_to_one() {
        // Each row sums to 1 for any s, so neutral greys are preserved.
        for s in [0.0, 0.5, 1.0, 2.0] {
            let m = rgb_matrix(s);
            for row in 0..3 {
                let sum: f64 = m[row * 3..row * 3 + 3].iter().sum();
                assert!(approx(sum, 1.0), "row {row} sum {sum} for s={s}");
            }
        }
    }

    #[test]
    fn rgba_alpha_passthrough() {
        let m = rgba_matrix(2.0);
        assert!(approx(m[15], 1.0));
        // Alpha row and column are otherwise zero.
        for i in 0..3 {
            assert!(approx(m[3 * 4 + i], 0.0)); // alpha row
            assert!(approx(m[i * 4 + 3], 0.0)); // alpha column
        }
    }

    #[test]
    fn cmyk_k_passthrough() {
        let m = cmyk_matrix(0.0);
        assert!(approx(m[15], 1.0));
        for i in 0..3 {
            assert!(approx(m[3 * 4 + i], 0.0)); // K row untouched
            assert!(approx(m[i * 4 + 3], 0.0)); // K column untouched
        }
    }
}
