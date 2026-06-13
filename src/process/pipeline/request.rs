use crate::enums::input::VipsInputFormat;
use crate::enums::output_format::OutputFormat;
use crate::params::parsed::Parameters;
use crate::process::source::Source;
use crate::services::format_resolver::{resolve_intermediate_format, resolve_output_format};
use crate::state::AppState;
use axum::http::HeaderMap;

pub struct PipelineRequest<'a> {
    pub state: &'a AppState,
    pub source: &'a Source,
    pub parameters: &'a Parameters,
    pub intermediate_format: Option<VipsInputFormat>,
    pub output_format: OutputFormat,
}

impl<'a> PipelineRequest<'a> {
    pub fn new(headers: &HeaderMap, state: &'a AppState, source: &'a Source, parameters: &'a Parameters) -> Self {
        Self {
            state,
            source,
            parameters,
            intermediate_format: resolve_intermediate_format(source),
            output_format: resolve_output_format(&headers, &state, &parameters),
        }
    }

    // fn get_source_details(path: &PathBuf, parameters: &Parameters) -> Result<(u16, u16)> {
    //     let path = path.to_str().ok_or(anyhow!("Invalid path"))?;
    //
    //     let image = VipsImage::new_from_file(path)?;
    //     let (width, height) = (image.get_width() as u16, image.get_height() as u16);
    //
    //     Ok(match parameters.auto_rotate == true {
    //         // EXIF orientations 6 and 8 rotate the image 90°/270°, swapping width and height
    //         // EXIF orientations 5 and 7 are mirrored variants of 6 and 8, also swapping width and height
    //         true if matches!(image.get_orientation(), 5 | 6 | 7 | 8) => (height, width),
    //         _ => (width, height),
    //     })
    // }
}