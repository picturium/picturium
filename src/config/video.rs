use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VideoConfig {
    pub default_time: String,
    pub extraction_timeout: u64,
    pub animation_timeout: u64,
    pub animation_frames: u32,
    pub animation_fps: f64,
    pub max_animation_frames: u32,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            default_time: "1".into(),
            extraction_timeout: 15,
            animation_timeout: 60,
            animation_frames: 48,
            animation_fps: 12.0,
            max_animation_frames: 300,
        }
    }
}
