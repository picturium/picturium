use serde::{Deserialize, Deserializer};

pub fn parse_scale(value: f32) -> Result<f32, &'static str> {
    if value <= 0.0 {
        return Err("scale must be greater than 0.0");
    }
    Ok(value)
}

pub fn deserialize_scale<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f32>, D::Error> {
    let value = Option::<f32>::deserialize(d)?;

    if let Some(scale) = value {
        parse_scale(scale).map_err(serde::de::Error::custom)?;
    }

    Ok(value)
}
