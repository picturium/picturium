use serde::de::Visitor;
use serde::{Deserialize, Deserializer, de};
use std::fmt;
use std::str::FromStr;

const MIN_DELAY: u16 = 5;
const MAX_DELAY: u16 = 5000;

#[derive(Debug, Clone)]
pub struct Animate {
    pub enabled: bool,
    pub frames: Option<i32>,
    pub timing: Option<u16>,
    pub loop_count: Option<u16>,
    pub stride: Option<u16>,
}

impl Default for Animate {
    fn default() -> Self {
        Self {
            enabled: true,
            frames: None,
            timing: None,
            loop_count: None,
            stride: None,
        }
    }
}

impl Animate {
    pub fn is_requested(&self) -> bool {
        self.enabled
            && (self.frames.is_some()
                || self.timing.is_some()
                || self.loop_count.is_some()
                || self.stride.is_some())
    }

    pub fn requested_frames(&self) -> i32 {
        match self.enabled {
            false => 1,
            true => self.frames.filter(|frames| *frames > 0).unwrap_or(-1),
        }
    }
}

#[derive(Debug)]
pub struct AnimateParseError(String);

impl fmt::Display for AnimateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AnimateParseError {}

impl FromStr for Animate {
    type Err = AnimateParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut animate = Animate::default();

        if matches!(s.trim(), "off" | "false" | "no" | "0") {
            animate.enabled = false;
            return Ok(animate);
        }

        let parts: Vec<&str> = s.split('|').filter(|s| !s.trim().is_empty()).collect();
        let mut fps = None;

        for part in parts {
            let (key, value) = part.split_once(':').ok_or_else(|| {
                AnimateParseError(format!("Missing ':' in animate segment '{part}'"))
            })?;

            match key {
                "frames" => {
                    animate.frames = Some(value.parse::<i32>().map_err(|_| {
                        AnimateParseError(format!("Invalid frames value: '{value}'"))
                    })?)
                }
                "timing" => animate.timing = Some(parse_delay(value)?),
                "fps" => {
                    let rate = value
                        .parse::<f32>()
                        .ok()
                        .filter(|rate| *rate > 0.0)
                        .ok_or_else(|| {
                            AnimateParseError(format!("Invalid fps value: '{value}'"))
                        })?;

                    fps = Some(delay_in_range((1000.0 / rate).round() as i64, value)?);
                }
                "loop" => {
                    animate.loop_count = Some(value.parse::<u16>().map_err(|_| {
                        AnimateParseError(format!("Invalid loop value: '{value}'"))
                    })?)
                }
                "stride" => {
                    let stride = value.parse::<u16>().map_err(|_| {
                        AnimateParseError(format!("Invalid stride value: '{value}'"))
                    })?;

                    if stride < 1 {
                        return Err(AnimateParseError("Stride must be at least 1".into()));
                    }

                    animate.stride = Some(stride);
                }
                _ => return Err(AnimateParseError(format!("Unknown animate key: '{key}'"))),
            }
        }

        match (animate.timing, fps) {
            (Some(_), Some(_)) => Err(AnimateParseError(
                "Only one of 'timing' and 'fps' can be given".into(),
            )),
            (None, Some(fps)) => {
                animate.timing = Some(fps);
                Ok(animate)
            }
            _ => Ok(animate),
        }
    }
}

fn parse_delay(value: &str) -> Result<u16, AnimateParseError> {
    let delay = value
        .parse::<i64>()
        .map_err(|_| AnimateParseError(format!("Invalid timing value: '{value}'")))?;

    delay_in_range(delay, value)
}

fn delay_in_range(delay: i64, value: &str) -> Result<u16, AnimateParseError> {
    match u16::try_from(delay) {
        Ok(delay) if (MIN_DELAY..=MAX_DELAY).contains(&delay) => Ok(delay),
        _ => Err(AnimateParseError(format!(
            "Frame delay must be between {MIN_DELAY} and {MAX_DELAY} ms, got '{value}' ({delay} ms)"
        ))),
    }
}

impl<'de> Deserialize<'de> for Animate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Animate;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "animate parameters in format anim=frames:10|timing:500|loop:0|stride:2, or anim=off")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animate(value: &str) -> Animate {
        value.parse().unwrap()
    }

    #[test]
    fn animation_is_on_by_default() {
        let animate = Animate::default();

        assert!(animate.enabled);
        assert_eq!(animate.requested_frames(), -1);
    }

    #[test]
    fn parses_every_key() {
        let animate = animate("frames:10|timing:500|loop:3|stride:2");

        assert!(animate.enabled);
        assert_eq!(animate.frames, Some(10));
        assert_eq!(animate.timing, Some(500));
        assert_eq!(animate.loop_count, Some(3));
        assert_eq!(animate.stride, Some(2));
    }

    #[test]
    fn the_off_switch_flattens_to_a_single_frame() {
        for value in ["off", "false", "no", "0"] {
            let animate = animate(value);

            assert!(!animate.enabled, "{value} should switch animation off");
            assert_eq!(animate.requested_frames(), 1);
        }
    }

    #[test]
    fn a_frame_count_below_one_means_every_frame() {
        assert_eq!(animate("frames:-1").requested_frames(), -1);
        assert_eq!(animate("frames:0").requested_frames(), -1);
        assert_eq!(animate("frames:12").requested_frames(), 12);
    }

    #[test]
    fn fps_becomes_a_frame_delay() {
        assert_eq!(animate("fps:10").timing, Some(100));
        assert_eq!(animate("fps:24").timing, Some(42));
    }

    #[test]
    fn timing_and_fps_are_mutually_exclusive() {
        assert!("timing:100|fps:10".parse::<Animate>().is_err());
        assert!("fps:10|timing:100".parse::<Animate>().is_err());
    }

    #[test]
    fn rejects_a_delay_outside_the_supported_range() {
        assert!("timing:1".parse::<Animate>().is_err());
        assert!("timing:9999".parse::<Animate>().is_err());
        assert!("fps:300".parse::<Animate>().is_err());
        assert!("fps:0.1".parse::<Animate>().is_err());
        assert!("fps:0".parse::<Animate>().is_err());
    }

    #[test]
    fn rejects_a_stride_below_one() {
        assert!("stride:0".parse::<Animate>().is_err());
        assert_eq!(animate("stride:1").stride, Some(1));
    }

    #[test]
    fn an_animation_is_only_requested_when_a_key_is_given() {
        assert!(!Animate::default().is_requested());
        assert!(!animate("off").is_requested());
        assert!(animate("frames:10").is_requested());
        assert!(animate("timing:100").is_requested());
        assert!(animate("fps:10").is_requested());
        assert!(animate("loop:0").is_requested());
        assert!(animate("stride:2").is_requested());
    }

    #[test]
    fn rejects_unknown_keys_and_missing_separators() {
        assert!("bogus:1".parse::<Animate>().is_err());
        assert!("10".parse::<Animate>().is_err());
    }
}
