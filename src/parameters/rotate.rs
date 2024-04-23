use serde::Serialize;

#[derive(Default, Copy, Clone, Debug, Serialize, PartialEq)]
pub enum Rotate {
    #[default]
    No = 0,
    Left = 90,
    UpsideDown = 180,
    Right = 270,
}

impl Rotate {

    pub fn from(value: &Option<String>) -> Self {

        let value = match value {
            Some(value) => value,
            None => return Self::default()
        };

        match value.as_str() {
            "90" | "left" | "anticlockwise" => Rotate::Right,
            "180" | "bottom-up" | "upside-down" => Rotate::UpsideDown,
            "270" | "right" | "clockwise" => Rotate::Left,
            _ => Rotate::No
        }

    }

}