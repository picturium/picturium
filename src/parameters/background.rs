use log::error;
use serde::Serialize;

#[derive(Default, Debug, PartialEq, Serialize)]
pub struct Background(u8, u8, u8, u8);

impl Background {
    pub fn from(value: &Option<String>) -> Option<Background> {

        // Format: bg=#rrggbb[aa] or bg=r,g,b[,a]
        let value = match value {
            Some(value) => value,
            None => return None
        };

        match value.starts_with('#') {
            true => {
                let value = value.trim_start_matches('#');
                
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
            false => {
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