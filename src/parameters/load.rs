use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Load {
    pub dpi: Option<f64>
}

impl Default for Load {
    fn default() -> Self {
        Load {
            dpi: None
        }
    }
}

impl Load {
    pub fn from(value: &Option<String>) -> Self {
        
        if value.is_none() {
            return Self::default();
        }
        
        let parameters = value.as_ref().unwrap().split(",");
        let mut load = Self::default();
        
        for parameter in parameters {
            let parts: Vec<&str> = parameter.split(":").collect();
            
            if parts.len() < 2 {
                continue;
            }
            
            match parts[0] {
                "dpi" => {
                    load.dpi = match parts[1].parse::<f64>() {
                        Ok(dpi) => Some(dpi),
                        Err(_) => None,
                    }
                },
                _ => {}
            };
        }
        
        load
        
    }
}