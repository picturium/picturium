use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Thumbnail {
    pub page: Option<u32>,
}

impl Default for Thumbnail {
    fn default() -> Self {
        Thumbnail { 
            page: Some(1)
        }
    }
}

impl Thumbnail {
    pub fn from(value: &Option<String>) -> Self {
        let mut thumbnail = Self::default();

        // Format: p:{page}
        let value = match value {
            Some(value) => value,
            None => return thumbnail,
        };

        let parts: Vec<&str> = value.split(',').collect();

        for part in parts {
            let pair: Vec<&str> = part.split(':').collect();
            let key = pair.first().unwrap();
            let value = pair.last().unwrap();

            if *key == "p" {
                thumbnail.page = Some(
                    value
                        .parse::<u32>()
                        .unwrap_or_else(|_| Self::default().page.unwrap())
                        .max(1),
                )
            }
        }

        thumbnail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_default() {
        let thumbnail = Thumbnail::default();
        assert_eq!(thumbnail.page, Some(1));
    }

    #[test]
    fn test_thumbnail_from_none() {
        let thumbnail = Thumbnail::from(&None);
        assert_eq!(thumbnail, Thumbnail::default());
    }

    #[test]
    fn test_thumbnail_from_empty() {
        let thumbnail = Thumbnail::from(&Some("".to_string()));
        assert_eq!(thumbnail, Thumbnail::default());
    }

    #[test]
    fn test_thumbnail_from_valid_page() {
        let thumbnail = Thumbnail::from(&Some("p:2".to_string()));
        assert_eq!(thumbnail.page, Some(2));
    }
}
