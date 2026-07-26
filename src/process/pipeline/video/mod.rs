use crate::enums::input::VideoInputFormat;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::{Result, anyhow};

/// Run the video pre-pipeline.
///
/// Extracts a representative frame from the video source and writes it to a
/// temporary file. The returned [`TempFile`] owns the path and will delete the
/// file automatically when dropped.
pub fn process(request: &PipelineRequest, format: VideoInputFormat) -> Result<String> {
    unimplemented!()
    // let source_path = request.source.path
    //     .ok_or_else(|| anyhow!("Video source path is not set"))?;
    //
    // // TODO: Implement video frame extraction (e.g. via ffmpeg) for each format.
    // //       For now every variant falls through to the unimplemented stub.
    // let _format = format;
    // let _source_path = source_path;
    //
    // Err(anyhow!("Video pipeline not yet implemented for {:?}", format))
}
