use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Deserializer};

pub fn deserialize_style<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let value = Option::<String>::deserialize(d)?;

    let Some(style) = value else {
        return Ok(None);
    };

    if style.is_empty() {
        return Err(serde::de::Error::custom("style must not be empty"));
    }

    let decoded = BASE64.decode(style.as_bytes())
        .map_err(|e| serde::de::Error::custom(format!("style is not valid base64: {e}")))?;

    let decoded_str = String::from_utf8(decoded)
        .map_err(|e| serde::de::Error::custom(format!("style is not valid UTF-8: {e}")))?;

    let cleaned = decoded_str.split_whitespace().collect::<Vec<_>>().join(" ");

    Ok(Some(cleaned))
}
