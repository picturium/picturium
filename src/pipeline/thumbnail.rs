// use std::path::Path;
// use crate::parameters::UrlParameters;
// use crate::pipeline::PipelineResult;
// use crate::services::formats::{is_thumbnail_format, OutputFormat};

// pub(crate) async fn run(working_file: &mut Path, url_parameters: &UrlParameters<'_>, output_format: &OutputFormat) -> PipelineResult<()> {
//
//     if !is_thumbnail_format(url_parameters.path) {
//         return Ok(());
//     }
//
//     // TODO > Generate thumbnail and change working_file
//
//     Ok(())
//
// }