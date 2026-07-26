use crate::enums::input::VipsInputFormat;
use crate::enums::output_format::OutputFormat;
use crate::params::parsed::Parameters;
use crate::process::source::Source;
use crate::services::format_resolver::{
    resolve_input_format, resolve_intermediate_format, resolve_output_format,
};
use crate::state::AppState;
use axum::http::HeaderMap;

#[derive(Debug)]
pub struct PipelineRequest<'a> {
    pub state: &'a AppState,
    pub source: &'a mut Source,
    pub parameters: &'a Parameters,
    pub input_format: VipsInputFormat,
    pub intermediate_format: Option<VipsInputFormat>,
    pub output_format: OutputFormat,
}

impl<'a> PipelineRequest<'a> {
    pub fn new(
        headers: &HeaderMap,
        state: &'a AppState,
        source: &'a mut Source,
        parameters: &'a Parameters,
    ) -> Self {
        let intermediate_format = resolve_intermediate_format(source);
        let input_format = resolve_input_format(source, intermediate_format);

        Self {
            state,
            source,
            parameters,
            input_format,
            intermediate_format,
            output_format: resolve_output_format(&headers, &state, &parameters),
        }
    }
}
