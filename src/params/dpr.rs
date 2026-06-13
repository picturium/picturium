use serde::{Deserialize, Deserializer};

pub fn deserialize_dpr<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f32>, D::Error> {
    let value = Option::<f32>::deserialize(d)?;

    if let Some(dpr) = value && dpr < 1.0 {
        return Err(serde::de::Error::custom("dpr must be greater than or equal to 1.0"));
    }

    Ok(value)
}
