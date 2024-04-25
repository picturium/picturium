use serde::Serialize;

#[derive(Default, Copy, Clone, Debug, Serialize, PartialEq)]
pub enum Format {
    #[default]
    Auto,
    Jpg,
    Png,
    Webp,
    Avif,
    Pdf
}

impl Format {

    pub fn from(value: &Option<String>) -> Self {

        let value = match value {
            Some(value) => value,
            None => return Self::default()
        };

        match value.as_str() {
            "jpg" | "jpeg" => Format::Jpg,
            "png" => Format::Png,
            "webp" => Format::Webp,
            "avif" => Format::Avif,
            "pdf" => Format::Pdf,
            _ => Format::Auto
        }

    }
    
    pub fn as_str(&self) -> &str {
        match self {
            Format::Auto => "auto",
            Format::Jpg => "jpg",
            Format::Png => "png",
            Format::Webp => "webp",
            Format::Avif => "avif",
            Format::Pdf => "pdf"
        }
    }

}