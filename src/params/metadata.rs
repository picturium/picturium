use std::str::FromStr;
use serde::{Deserialize, Deserializer};
use crate::enums::output_metadata::OutputMetadata;

pub type Metadata = Vec<OutputMetadata>;

pub fn deserialize_metadata<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Metadata>, D::Error> {
    let value = Option::<String>::deserialize(d)?;

    let Some(raw) = value else {
        return Ok(None);
    };

    let metadata = raw
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| OutputMetadata::from_str(s.trim()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| serde::de::Error::custom(format!("invalid metadata value: {e}")))?;

    Ok(Some(metadata))
}
