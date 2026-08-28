use crate::config::encoder::GifEncoderConfig;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use picturium_libvips::{GifSaveOptions, VipsBufferSaving, VipsImage, VipsKeep};

pub(crate) fn finish_image(
    request: &PipelineRequest,
    image: &VipsImage,
    keep: VipsKeep,
    quality: u8,
) -> anyhow::Result<Vec<u8>> {
    let output = &request.state.config.output;
    let config = &output.encoder.gif;

    let background = resolve_background(request.parameters.background);
    let settings = settings_for_quality(config, quality);

    image
        .save_gif(Some(GifSaveOptions {
            effort: config.effort,
            bitdepth: settings.bitdepth,
            dither: settings.dither,
            interframe_maxerror: settings.interframe_maxerror,
            interpalette_maxerror: settings.interpalette_maxerror,
            keep,
            background: &background,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Failed to save GIF image: {:?}", e))
}

struct GifSettings {
    bitdepth: i32,
    dither: f64,
    interframe_maxerror: f64,
    interpalette_maxerror: f64,
}

fn settings_for_quality(config: &GifEncoderConfig, quality: u8) -> GifSettings {
    GifSettings {
        bitdepth: interpolate(
            quality,
            config.min_bitdepth as f64,
            config.bitdepth as f64,
        )
        .round()
        .clamp(1.0, 8.0) as i32,
        dither: interpolate(quality, config.min_dither, config.dither),
        interframe_maxerror: interpolate(quality, config.interframe_maxerror, 0.0),
        interpalette_maxerror: interpolate(quality, config.interpalette_maxerror, 0.0),
    }
}

fn interpolate(quality: u8, at_min: f64, at_max: f64) -> f64 {
    at_min + (at_max - at_min) * (quality.clamp(1, 100) - 1) as f64 / 99.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(quality: u8) -> GifSettings {
        settings_for_quality(&GifEncoderConfig::default(), quality)
    }

    #[test]
    fn the_top_of_the_range_keeps_every_colour_and_frame_intact() {
        let settings = settings(100);

        assert_eq!(settings.bitdepth, 8);
        assert_eq!(settings.dither, 1.0);
        assert_eq!(settings.interframe_maxerror, 0.0);
        assert_eq!(settings.interpalette_maxerror, 0.0);
    }

    #[test]
    fn the_bottom_of_the_range_matches_the_configured_floor() {
        let settings = settings(1);

        assert_eq!(settings.bitdepth, 5);
        assert_eq!(settings.dither, 0.6);
        assert_eq!(settings.interframe_maxerror, 8.0);
        assert_eq!(settings.interpalette_maxerror, 20.0);
    }

    #[test]
    fn the_palette_shrinks_as_quality_drops() {
        assert_eq!(settings(90).bitdepth, 8);
        assert_eq!(settings(70).bitdepth, 7);
        assert_eq!(settings(50).bitdepth, 6);
        assert_eq!(settings(10).bitdepth, 5);
    }

    #[test]
    fn interframe_error_grows_as_quality_drops() {
        assert!(settings(90).interframe_maxerror < settings(50).interframe_maxerror);
        assert!(settings(50).interframe_maxerror < settings(10).interframe_maxerror);
    }

    #[test]
    fn a_flat_configuration_is_quality_independent() {
        let config = GifEncoderConfig {
            min_bitdepth: 8,
            min_dither: 1.0,
            interframe_maxerror: 0.0,
            interpalette_maxerror: 0.0,
            ..GifEncoderConfig::default()
        };

        for quality in [1, 50, 100] {
            let settings = settings_for_quality(&config, quality);
            assert_eq!(settings.bitdepth, 8);
            assert_eq!(settings.dither, 1.0);
        }
    }
}
