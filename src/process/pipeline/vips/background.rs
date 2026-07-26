use crate::params::background::Background;

pub(crate) fn resolve_background(background: Option<Background>) -> [f64; 4] {
    let background = background.unwrap_or_default();
    let (red, green, blue, alpha) = background.to_rgb_with_bit_depth(8);

    [red, green, blue, alpha]
}

pub(crate) fn resolve_opaque_matte(background: Option<Background>) -> [f64; 3] {
    let [red, green, blue, alpha] = resolve_background(background);
    let opacity = alpha / 255.0;

    [red * opacity, green * opacity, blue * opacity]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::color::Color;

    #[test]
    fn omitted_background_is_transparent() {
        assert_eq!(resolve_background(None), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn opaque_background_is_resolved_to_eight_bit_rgba() {
        assert_eq!(
            resolve_background(Some(Color {
                red: 1.0,
                green: 0.5,
                blue: 0.0,
                alpha: 1.0,
            })),
            [255.0, 127.5, 0.0, 255.0],
        );
    }

    #[test]
    fn transparent_color_preserves_its_rgb_channels() {
        assert_eq!(
            resolve_background(Some(Color {
                red: 1.0,
                green: 0.5,
                blue: 0.25,
                alpha: 0.0,
            })),
            [255.0, 127.5, 63.75, 0.0],
        );
    }

    #[test]
    fn semi_transparent_jpeg_matte_is_composited_over_black() {
        assert_eq!(
            resolve_opaque_matte(Some(Color {
                red: 1.0,
                green: 0.5,
                blue: 0.0,
                alpha: 0.5,
            })),
            [127.5, 63.75, 0.0],
        );
    }
}
