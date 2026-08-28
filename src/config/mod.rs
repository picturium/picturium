mod server;
mod security;
mod cors;
mod vips;
mod data;
mod cache;
pub mod encoder;
pub mod quality;
mod svg;
mod pdf;
mod image;
pub mod watermark;
pub mod output;
mod office;
mod vector;
pub mod video;

use crate::config::cache::CacheConfig;
use crate::config::cors::CorsConfig;
use crate::config::data::DataConfig;
use crate::config::image::ImageConfig;
use crate::config::office::OfficeConfig;
use crate::config::vector::VectorConfig;
use crate::config::output::OutputConfig;
use crate::config::pdf::PdfConfig;
use crate::config::security::SecurityConfig;
use crate::config::server::ServerConfig;
use crate::config::svg::SvgConfig;
use crate::config::video::VideoConfig;
use crate::config::vips::VipsConfig;
use crate::config::watermark::WatermarkConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const CONFIG_PATH_ENV: &str = "PICTURIUM_CONFIG";
pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub cors: CorsConfig,
    pub vips: VipsConfig,
    pub data: DataConfig,
    pub cache: CacheConfig,
    pub svg: SvgConfig,
    pub pdf: PdfConfig,
    pub image: ImageConfig,
    pub watermark: WatermarkConfig,
    pub output: OutputConfig,
    pub office: OfficeConfig,
    pub vector: VectorConfig,
    pub video: VideoConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = std::env::var(CONFIG_PATH_ENV).unwrap_or_else(|_| DEFAULT_CONFIG_PATH.into());

        let config: Self = ::config::Config::builder()
            .add_source(
                ::config::Config::try_from(&Self::default())
                    .context("Failed to build the default configuration")?,
            )
            .add_source(::config::File::new(&path, ::config::FileFormat::Toml).required(false))
            .add_source(
                ::config::Environment::with_prefix("PICTURIUM")
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(",")
                    .with_list_parse_key("cors.allowed_origins")
                    .with_list_parse_key("data.serve")
                    .with_list_parse_key("output.format_priority")
                    .with_list_parse_key("output.metadata"),
            )
            .build()
            .with_context(|| format!("Failed to load configuration from {path}"))?
            .try_deserialize()
            .with_context(|| format!("Failed to parse configuration from {path}"))?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        self.watermark.validate()?;
        self.output.validate()?;
        self.cache.validate()
    }
}

pub type SharedConfig = Arc<Config>;
