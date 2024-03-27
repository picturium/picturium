use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Thumbnail {
    pub page: u16
}

impl Default for Thumbnail {
    fn default() -> Self {
        Thumbnail {
            page: 1
        }
    }
}

impl Thumbnail {

    pub fn from(value: &Option<String>) -> Option<Self> {

        // Format: page,dpi
        let value = match value {
            Some(value) => value,
            None => return None
        };

        let parts: Vec<&str> = value.split(',').collect();

        let page = match parts.first() {
            Some(page) => page.parse::<u16>().unwrap_or_else(|_| Self::default().page),
            None => Self::default().page
        };

        Some(Thumbnail {
            page
        })

    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_default() {
        let thumbnail = Thumbnail::default();
        assert_eq!(thumbnail.page, 1);
    }

    #[test]
    fn test_thumbnail_from_none() {
        let thumbnail = Thumbnail::from(&None);
        assert_eq!(thumbnail, None);
    }

    #[test]
    fn test_thumbnail_from_empty() {
        let thumbnail = Thumbnail::from(&Some("".to_string()));
        assert_eq!(thumbnail, Some(Thumbnail::default()));
    }

    #[test]
    fn test_thumbnail_from_valid_page() {
        let thumbnail = Thumbnail::from(&Some("2".to_string())).unwrap();
        assert_eq!(thumbnail.page, 2);
    }
}