use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use background::Background;
pub use crop::Crop;
pub use rotate::Rotate;
pub use thumbnail::Thumbnail;

use crate::crypto::verify_hmac;
use crate::parameters::format::Format;

pub mod background;
pub mod rotate;
pub mod thumbnail;
pub mod origin;
pub mod crop;
pub mod format;

pub type ParametersResult<T> = Result<T, &'static str>;

#[derive(Deserialize, Debug)]
pub struct RawUrlParameters {
    w: Option<u16>,
    h: Option<u16>,
    q: Option<u8>,
    dpr: Option<f32>,
    crop: Option<String>,
    thumb: Option<String>,
    original: Option<bool>,
    rot: Option<String>,
    bg: Option<String>,
    f: Option<String>,
    token: Option<String>
}

impl RawUrlParameters {
    pub fn verify_token(&self, path: &str, url_parameters: &HashMap<String, String>) -> ParametersResult<()> {

        let env_key = match std::env::var("KEY") {
            Ok(key) => key,
            Err(_) => return Ok(())
        };

        if self.token.is_none() {
            return Err("Token is required");
        }

        let mut url_parameters: Vec<(&String, &String)> = url_parameters.iter().filter(|(k, _)| *k != "token").collect();
        url_parameters.sort_by(|a, b| a.0.cmp(b.0));

        let data: String = url_parameters.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<String>>().join("&");
        let data = format!("{}?{}", path, data);

        match verify_hmac(&data, &env_key, &self.token.clone().unwrap()) {
            true => Ok(()),
            false => Err("Invalid token")
        }

    }
}

#[derive(Debug, Serialize)]
pub struct UrlParameters<'a> {
    pub path: &'a Path,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub quality: Quality,
    pub crop: Option<Crop>,
    pub thumbnail: Thumbnail,
    pub original: bool,
    pub rotate: Rotate,
    pub background: Option<Background>,
    pub format: Format
}

impl<'a> UrlParameters<'a> {
    pub fn new(path: &'a str, value: RawUrlParameters) -> Self {
        
        let dpr = value.dpr.unwrap_or(1.0);
        let width = value.w.map(|width| (width as f32 * dpr).round() as u16);
        let height = value.h.map(|height| (height as f32 * dpr).round() as u16);
        
        Self {
            path: Path::new(path),
            width,
            height,
            quality: match value.q {
                Some(q) => Quality::Custom(q),
                None => Quality::Default
            },
            crop: Crop::from(&value.crop),
            thumbnail: Thumbnail::from(&value.thumb),
            original: value.original.unwrap_or(false),
            rotate: Rotate::from(&value.rot),
            background: Background::from(&value.bg),
            format: Format::from(&value.f)
        }
        
    }
}

#[derive(Debug, Serialize)]
pub enum Quality {
    Default,
    Custom(u8)
}