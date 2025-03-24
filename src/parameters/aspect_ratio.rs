use log::{debug, warn};
use serde::Serialize;

const VIDEO_ASPECT_RATIO: f64 = 16.0 / 9.0;
const SQUARE_ASPECT_RATIO: f64 = 1.0;

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub enum AspectRatio {
    #[default]
    Auto,
    Custom(f64)
}

impl AspectRatio {

    pub fn from(value: &Option<String>) -> Self {
        
        let value = match value {
            Some(value) => value,
            None => return Self::default()
        };

        let ar = match value.as_str() {
            "auto" => Self::Auto,
            "video" => Self::Custom(VIDEO_ASPECT_RATIO),
            "square" => Self::Custom(SQUARE_ASPECT_RATIO),
            _ => Self::parse_parameter(value)
        };

        debug!("{ar:?}");

        ar

    }

    fn parse_parameter(value: &str) -> Self {

        let parts: Vec<&str> = value.split("/").collect();

        if parts.len() != 2 {
            warn!("Invalid aspect ratio format: {value}");
            return Self::default();
        }

        let width = match parts[0].parse::<f64>() {
            Ok(width) => width.max(0.0),
            Err(_) => {
                warn!("Failed to parse aspect ratio [width] from: {value}");
                return Self::default()
            },
        };

        let height = match parts[1].parse::<f64>() {
            Ok(height) => height.max(0.0),
            Err(_) => {
                warn!("Failed to parse aspect ratio [height] from: {value}");
                return Self::default()
            },
        };

        if width == 0.0 || height == 0.0 {
            warn!("Invalid aspect ratio value: {value}");
            return Self::default();
        }

        Self::Custom(width / height)

    }

}
