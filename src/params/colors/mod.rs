pub mod rgb;
pub mod hsl;
pub mod hwb;
pub mod oklab;
pub mod oklch;
pub mod hex;
pub mod named;

use std::fmt;

#[derive(Debug)]
pub struct ColorParseError(pub String);

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ColorParseError {}

pub trait Color {
    fn to_rgb(&self) -> (f64, f64, f64, f64);
}