use serde::Serialize;
use crate::parameters::origin::Origin;

#[derive(Debug, PartialEq, Serialize)]
pub struct Crop {
    pub aspect_ratio: AspectRatio,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub gravity: Origin,
    pub offset: (i16, i16),
}

impl Crop {
    pub fn from(value: &Option<String>) -> Option<Crop> {

        // Format: crop=aspect_ratio,width?,height?,origin?,offset_x?,offset_y?
        let value = match value {
            Some(value) => value,
            None => return None
        };

        let parts: Vec<&str> = value.split(',').collect();

        let aspect_ratio = match parts[0] {
            "video" => AspectRatio::Video,
            "square" => AspectRatio::Square,
            "free" => AspectRatio::Free,
            _ => {
                let ratio_parts: Vec<&str> = parts[0].split(':').collect();

                if ratio_parts.len() < 2 {
                    return None;
                }

                let width = match ratio_parts[0].parse::<u8>() {
                    Ok(value) => value,
                    Err(_) => return None
                };

                let height = match ratio_parts[1].parse::<u8>() {
                    Ok(value) => value,
                    Err(_) => return None
                };

                AspectRatio::Custom(width, height)
            }
        };

        let width = if parts.len() > 1 {
            parts[1].parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let width = if width == 0 {
            None
        } else {
            Some(width)
        };

        let height = if parts.len() > 2 {
            parts[2].parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let height = if aspect_ratio == AspectRatio::Free {
            if height == 0 || width.is_none() {
                return None;
            } else {
                Some(height)
            }
        } else if width.is_some() && height == 0 {
            None
        } else if height != 0 {
            Some(height)
        } else {
            None
        };

        let gravity = if parts.len() > 3 {
            Origin::from(parts[3])
        } else {
            Origin::default()
        };

        let offset_x = if parts.len() > 4 {
            parts[4].parse::<i16>().unwrap_or(0)
        } else {
            0
        };

        let offset_y = if parts.len() > 5 {
            parts[5].parse::<i16>().unwrap_or(0)
        } else {
            0
        };

        Some(Crop {
            aspect_ratio,
            width,
            height,
            gravity,
            offset: (offset_x, offset_y)
        })

    }
}

#[derive(Default, Debug, PartialEq, Serialize)]
pub enum AspectRatio {
    #[default]
    Video,
    Square,
    Custom(u8, u8),
    Free
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_from() {
        let crop = Crop::from(&Some("video,100,200,top,10,20".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Video);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (10, 20));

        let crop = Crop::from(&Some("square,100,200,top,10,20".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (10, 20));

        let crop = Crop::from(&Some("free,100,200,top,10,20".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Free);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (10, 20));

        let crop = Crop::from(&Some("16:9,100,200,top,10,20".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Custom(16, 9));
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (10, 20));

        let crop = Crop::from(&Some("video".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Video);
        assert_eq!(crop.width, None);
        assert_eq!(crop.height, None);
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,0".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, None);
        assert_eq!(crop.height, None);
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,100".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, None);
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,100,0".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, None);
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,100,200".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,100,200,top".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (0, 0));

        let crop = Crop::from(&Some("square,100,200,top,10".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::TopCenter);
        assert_eq!(crop.offset, (10, 0));

        let crop = Crop::from(&Some("square,100,200,xyz".to_string())).unwrap();
        assert_eq!(crop.aspect_ratio, AspectRatio::Square);
        assert_eq!(crop.width, Some(100));
        assert_eq!(crop.height, Some(200));
        assert_eq!(crop.gravity, Origin::Center);
        assert_eq!(crop.offset, (0, 0));
    }
}
