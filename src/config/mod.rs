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
mod watermark;
pub mod output;
mod office;

use crate::config::cache::CacheConfig;
use crate::config::cors::CorsConfig;
use crate::config::data::DataConfig;
use crate::config::image::ImageConfig;
use crate::config::office::OfficeConfig;
use crate::config::output::OutputConfig;
use crate::config::pdf::PdfConfig;
use crate::config::security::SecurityConfig;
use crate::config::server::ServerConfig;
use crate::config::svg::SvgConfig;
use crate::config::vips::VipsConfig;
use crate::config::watermark::WatermarkConfig;
use anyhow::{Context, Result};
use std::str::FromStr;
use std::sync::Arc;

pub(self) fn parse_env<T: FromStr>(key: &str, default: &str) -> Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(key)
        .unwrap_or(default.into())
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse {key}"))
}

/// Similar to [`parse_env`], but falls back to a typed default instead of a string
pub(self) fn parse_env_or<T: FromStr>(key: &str, default: T) -> Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match std::env::var(key) {
        Ok(value) => value
            .trim()
            .parse()
            .with_context(|| format!("Failed to parse {key}")),
        Err(_) => Ok(default),
    }
}

pub trait ConfigFromEnv {
    fn from_env() -> Result<Self> where Self: Sized;
}

#[derive(Debug, Clone)]
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
}

impl ConfigFromEnv for Config {
    fn from_env() -> Result<Self> {
        Ok(Config {
            server: ServerConfig::from_env()?,
            security: SecurityConfig::from_env()?,
            cors: CorsConfig::from_env()?,
            vips: VipsConfig::from_env()?,
            data: DataConfig::from_env()?,
            cache: CacheConfig::from_env()?,
            svg: SvgConfig::from_env()?,
            pdf: PdfConfig::from_env()?,
            image: ImageConfig::from_env()?,
            watermark: WatermarkConfig::from_env()?,
            output: OutputConfig::from_env()?,
            office: OfficeConfig::from_env()?,
        })
    }
}

pub type SharedConfig = Arc<Config>;
