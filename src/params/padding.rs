use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Padding for each side: (top, right, bottom, left).
/// Supports CSS-like shorthand syntax with `,` as separator:
/// - `10`             → all sides 10
/// - `10,20`          → top/bottom 10, left/right 20
/// - `10,20,30`       → top 10, left/right 20, bottom 30
/// - `10,20,30,40`    → top 10, right 20, bottom 30, left 40
#[derive(Debug, Clone, Copy, PartialEq, Default, Eq)]
pub struct Padding {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Debug)]
pub struct PaddingParseError(String);

impl fmt::Display for PaddingParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid padding value: '{}'", self.0)
    }
}

impl std::error::Error for PaddingParseError {}

impl FromStr for Padding {
    type Err = PaddingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        let nums: Result<Vec<u32>, _> = parts.iter().map(|p| p.trim().parse()).collect();
        let nums = nums.map_err(|_| PaddingParseError(format!("Invalid padding value: '{s}'")))?;

        let (top, right, bottom, left) = match nums.as_slice() {
            // single value  →  all sides equal
            [all] => (*all, *all, *all, *all),
            // horizontal,vertical  →  sides derived symmetrically
            [v, h] => (*v, *h, *v, *h),
            // top,horizontal,bottom  →  left mirrors right
            [top, h, bottom] => (*top, *h, *bottom, *h),
            // top,right,bottom,left
            [top, right, bottom, left] => (*top, *right, *bottom, *left),
            _ => return Err(PaddingParseError(s.to_string())),
        };

        Ok(Self { top, right, bottom, left })
    }
}

impl<'de> Deserialize<'de> for Padding {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Padding;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "padding value like 10,20,30,40")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                let all = u32::try_from(v).map_err(de::Error::custom)?;
                Ok(Padding { top: all, right: all, bottom: all, left: all })
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                let all = u32::try_from(v).map_err(de::Error::custom)?;
                Ok(Padding { top: all, right: all, bottom: all, left: all })
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_any(V)
    }
}

impl Serialize for Padding {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{},{},{},{}", self.top, self.right, self.bottom, self.left))
    }
}

impl Padding {
    pub fn apply_dpr(&self, dpr: f32) -> Self {
        Self {
            top: (self.top as f32 * dpr) as u32,
            right: (self.right as f32 * dpr) as u32,
            bottom: (self.bottom as f32 * dpr) as u32,
            left: (self.left as f32 * dpr) as u32,
        }
    }
}
