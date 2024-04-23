use log::error;
use serde::Serialize;

const COLORS: [&str; 3] = ["transparent", "black", "white"];

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct Background(pub u8, pub u8, pub u8, pub u8);

impl Background {
    pub fn is_transparent(&self) -> bool {
        self.3 == 0
    }
    
    pub fn from(value: &Option<String>) -> Option<Background> {

        // Format: bg=#rrggbb[aa] or bg=r,g,b[,a] or bg=transparent
        let value = match value {
            Some(value) => value,
            None => return None
        };

        if COLORS.contains(&value.as_str()) {
            return match value.as_str() {
                "transparent" => Some(Background(0, 0, 0, 0)),
                "black" => Some(Background(0, 0, 0, 255)),
                "white" => Some(Background(255, 255, 255, 255)),
                _ => None
            };
        }

        match value.contains(',') {
            false => {
                if value.len() != 6 && value.len() != 8 {
                    error!("Invalid background format: {value}");
                    return None;
                }
                
                let r = match u8::from_str_radix(&value[0..2], 16) {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                let g = match u8::from_str_radix(&value[2..4], 16) {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                let b = match u8::from_str_radix(&value[4..6], 16) {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                // alpha is optional
                let a = if value.len() == 8 {
                    match u8::from_str_radix(&value[6..8], 16) {
                        Ok(value) => value,
                        Err(_) => {
                            error!("Invalid background format: {value}");
                            return None
                        }
                    }
                } else {
                    255
                };

                Some(Background(r, g, b, a))
            },
            true => {
                let parts: Vec<&str> = value.split(',').collect();

                if parts.len() < 3 || parts.len() > 4 {
                    error!("Invalid background format: {value}");
                    return None;
                }

                let r = match parts[0].parse::<u8>() {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                let g = match parts[1].parse::<u8>() {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                let b = match parts[2].parse::<u8>() {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Invalid background format: {value}");
                        return None
                    }
                };
                
                let a = if parts.len() == 4 {
                    match parts[3].parse::<u8>() {
                        Ok(value) => value,
                        Err(_) => {
                            error!("Invalid background format: {value}");
                            return None
                        }
                    }
                } else {
                    255
                };

                Some(Background(r, g, b, a))
            }
        }
        
    }
}

impl From<&Background> for Vec<f64> {
    fn from(value: &Background) -> Self {
        vec![value.0 as f64, value.1 as f64, value.2 as f64, value.3 as f64]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_background_try_from() {
        assert_eq!(Background::from(&None), None);
        assert_eq!(Background::from(&Some("".to_string())), None);
        assert_eq!(Background::from(&Some("invalid".to_string())), None);
        assert_eq!(Background::from(&Some("123".to_string())), None);
        assert_eq!(Background::from(&Some("123,123".to_string())), None);
        assert_eq!(Background::from(&Some("123,123,123".to_string())), Some(Background(123, 123, 123, 255)));
        assert_eq!(Background::from(&Some("123,123,123,123".to_string())), Some(Background(123, 123, 123, 123)));
        assert_eq!(Background::from(&Some("#1234".to_string())), None);
        assert_eq!(Background::from(&Some("#123456".to_string())), Some(Background(18, 52, 86, 255)));
        assert_eq!(Background::from(&Some("#12345678".to_string())), Some(Background(18, 52, 86, 120)));
    }
}