use crate::config::SharedConfig;
use crate::enums::autorot::Autorot;
use crate::enums::download::Download;
use crate::enums::dpi::Dpi;
use crate::enums::force::Force;
use crate::enums::image_extend::ImageExtend;
use crate::enums::image_fit::ImageFit;
use crate::enums::image_gravity::ImageGravity;
use crate::enums::image_resample::ImageResample;
use crate::enums::original::Original;
use crate::enums::output_format::OutputFormat;
use crate::enums::output_quality::OutputQuality;
use crate::enums::upsize::Upsize;
use crate::params::aspect_ratio::AspectRatio;
use crate::params::background::Background;
use crate::params::crop::Crop;
use crate::params::filter::Filter;
use crate::params::limits::Limits;
use crate::params::metadata::Metadata;
use crate::params::padding::Padding;
use crate::params::RequestParams;
use crate::params::rotate::Rotate;
use crate::params::thumbnail::Thumbnail;
use crate::params::watermark::Watermark;

const DEFAULT_DPR: f32 = 1.0;
const DEFAULT_SCALE: f32 = 1.0;

#[derive(Debug)]
pub struct Parameters {
    pub force: Force,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub aspect_ratio: AspectRatio,
    pub gravity: ImageGravity,
    pub dpr: f32,
    pub scale: f32,
    pub upsize: Upsize,
    pub extend: ImageExtend,
    pub resample: ImageResample,
    pub fit: ImageFit,
    pub padding: Option<Padding>,
    pub auto_rotate: Autorot,
    pub rotate: Rotate,
    pub background: Option<Background>,
    pub crop: Option<Crop>,
    pub filter: Filter,
    pub download: Download,
    pub original: Original,
    pub quality: OutputQuality,
    pub format: OutputFormat,
    pub dpi: Dpi,
    pub style: Option<String>,
    pub metadata: Metadata,
    pub fallback: Option<String>,
    pub limits: Limits,
    pub pages: Option<Vec<u32>>,
    pub thumbnail: Thumbnail,
    pub watermark: Watermark,
}

impl Parameters {
    pub fn new(config: &SharedConfig, params: RequestParams) -> Self {
        let dpr = params.dpr.unwrap_or(DEFAULT_DPR);
        let scale = params.scale.unwrap_or(DEFAULT_SCALE);

        let padding = params.padding.map(|p| p.apply_dpr(dpr));
        let watermark = params.watermark.unwrap_or_default().apply_dpr(dpr);

        // `thumb=p:` is deprecated; new `page`|`pages` parameter takes priority
        let mut thumbnail = params.thumbnail.unwrap_or_default();
        let pages = params.pages.map(|pages| pages.0).or(thumbnail.pages.take());

        Self {
            force: params.force.unwrap_or_default(),
            width: params.width,
            height: params.height,
            aspect_ratio: params.aspect_ratio.unwrap_or_default(),
            gravity: params.gravity.unwrap_or(config.image.gravity),
            dpr,
            scale,
            upsize: params.upsize.unwrap_or_else(|| config.image.upsize.into()),
            extend: params.extend.unwrap_or(config.image.extend),
            resample: params.resample.unwrap_or(config.image.resample),
            fit: params.fit.unwrap_or(config.image.fit),
            padding,
            auto_rotate: params.auto_rotate.unwrap_or_else(|| config.image.auto_rotate.into()),
            rotate: params.rotate.unwrap_or_default(),
            background: params.background,
            crop: params.crop,
            filter: params.filter.unwrap_or_default(),
            download: params.download.unwrap_or_default(),
            original: params.original.unwrap_or_default(),
            quality: params.quality.unwrap_or(config.output.quality),
            format: params.format.unwrap_or_default(),
            dpi: params.dpi.unwrap_or_default(),
            style: params.style,
            metadata: params.metadata.unwrap_or_else(|| config.output.metadata.clone()),
            fallback: params.fallback,
            limits: {
                let mut limits = params.limits.unwrap_or_default();
                limits.size = limits.size.or(Some(config.output.max_size).filter(|size| *size > 0));

                let dimension = limits.dimension.get_or_insert_default();
                let config_limit = |value: u32| u16::try_from(value).ok().filter(|value| *value > 0);
                dimension.width = dimension.width.or_else(|| config_limit(config.output.max_width));
                dimension.height = dimension.height.or_else(|| config_limit(config.output.max_height));

                limits
            },
            pages,
            thumbnail,
            watermark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::pages::Pages;
    use std::sync::Arc;

    fn resolve(params: RequestParams) -> Parameters {
        Parameters::new(&Arc::new(crate::config::Config::default()), params)
    }

    #[test]
    fn the_top_level_pages_parameter_wins_over_the_deprecated_thumb_page() {
        let parameters = resolve(RequestParams {
            pages: Some(Pages(vec![5])),
            thumbnail: Some("p:2".parse().unwrap()),
            ..Default::default()
        });

        assert_eq!(parameters.pages, Some(vec![5]));
        assert_eq!(parameters.thumbnail.pages, None);
    }

    #[test]
    fn the_deprecated_thumb_page_still_selects_pages_on_its_own() {
        let parameters = resolve(RequestParams {
            thumbnail: Some("p:2|timing:500".parse().unwrap()),
            ..Default::default()
        });

        assert_eq!(parameters.pages, Some(vec![2]));
        assert_eq!(parameters.thumbnail.pages, None);
        assert_eq!(parameters.thumbnail.timing, Some(500));
    }

    #[test]
    fn no_page_selection_leaves_pages_unset() {
        assert_eq!(resolve(RequestParams::default()).pages, None);
    }
}
