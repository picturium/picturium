use serde::Serialize;

#[derive(Default, Debug, PartialEq, Serialize)]
pub enum Origin {
    #[default]
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    LeftCenter,
    RightCenter,
    BottomLeft,
    BottomCenter,
    BottomRight
}

impl From<&str> for Origin {
    fn from(value: &str) -> Self {
        match value {
            "top-left" | "left-top" => Origin::TopLeft,
            "top-center" | "center-top" | "top" => Origin::TopCenter,
            "top-right" | "right-top" => Origin::TopRight,
            "left-center" | "center-left" | "left" => Origin::LeftCenter,
            "right-center" | "center-right" | "right" => Origin::RightCenter,
            "bottom-left" | "left-bottom" => Origin::BottomLeft,
            "bottom-center" | "center-bottom" | "bottom" => Origin::BottomCenter,
            "bottom-right" | "right-bottom" => Origin::BottomRight,
            _ => Origin::Center
        }
    }
}