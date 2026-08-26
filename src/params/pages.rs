use std::fmt;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;

const MAX_PAGES: usize = 1000;

#[derive(Debug, Clone, Default)]
pub struct Pages(pub Vec<u32>);

#[derive(Debug)]
pub struct PagesParseError(String);

impl fmt::Display for PagesParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PagesParseError {}

pub fn parse_pages(value: &str) -> Result<Vec<u32>, PagesParseError> {
    let mut pages = Vec::new();

    for part in value.split(',') {
        let (first, last) = match part.split_once('-') {
            Some((first, last)) => (parse_page(first)?, parse_page(last)?),
            None => {
                let page = parse_page(part)?;
                (page, page)
            }
        };

        if last < first {
            return Err(PagesParseError(format!("Page range must ascend, got '{part}'")));
        }

        if pages.len() + (last - first + 1) as usize > MAX_PAGES {
            return Err(PagesParseError(format!("Page selection must be at most {MAX_PAGES} pages")));
        }

        pages.extend(first..=last);
    }

    Ok(pages)
}

fn parse_page(value: &str) -> Result<u32, PagesParseError> {
    match value.parse::<u32>() {
        Ok(page) if page >= 1 => Ok(page),
        Ok(page) => Err(PagesParseError(format!("Page value must be between 1 and {}, got '{page}'", u32::MAX))),
        Err(_) => Err(PagesParseError(format!("Invalid page value: '{value}'"))),
    }
}

impl FromStr for Pages {
    type Err = PagesParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_pages(s).map(Pages)
    }
}

impl<'de> Deserialize<'de> for Pages {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Pages;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "pages in format pages=1,4-7")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        d.deserialize_str(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_comma_separated_list() {
        assert_eq!(parse_pages("1,4").unwrap(), vec![1, 4]);
        assert_eq!(parse_pages("2").unwrap(), vec![2]);
    }

    #[test]
    fn rejects_a_page_below_one() {
        assert!(parse_pages("0").is_err());
        assert!(parse_pages("1,0").is_err());
    }

    #[test]
    fn rejects_a_non_numeric_page() {
        assert!(parse_pages("abc").is_err());
        assert!(parse_pages("1,").is_err());
    }

    #[test]
    fn expands_a_range() {
        assert_eq!(parse_pages("2-4").unwrap(), vec![2, 3, 4]);
        assert_eq!(parse_pages("1,4-7,9").unwrap(), vec![1, 4, 5, 6, 7, 9]);
        assert_eq!(parse_pages("3-3").unwrap(), vec![3]);
    }

    #[test]
    fn rejects_a_descending_range() {
        assert!(parse_pages("7-4").is_err());
    }

    #[test]
    fn rejects_a_selection_larger_than_the_cap() {
        assert_eq!(parse_pages(&format!("1-{MAX_PAGES}")).unwrap().len(), MAX_PAGES);
        assert!(parse_pages(&format!("1-{}", MAX_PAGES + 1)).is_err());
        assert!(parse_pages(&format!("1,1-{MAX_PAGES}")).is_err());
        assert!(parse_pages(&format!("1-{}", u32::MAX)).is_err());
    }
}
