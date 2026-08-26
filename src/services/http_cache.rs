use crate::enums::output_format::OutputFormat;
use crate::params::parsed::Parameters;
use axum::http::response::Builder as ResponseBuilder;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::Config;

pub const NO_STORE: &str = "no-store";

pub fn seed(config: &Config) -> String {
    let mut hasher = Sha256::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(format!("{config:?}").as_bytes());

    hex::encode(&hasher.finalize()[..8])
}

pub struct Validators {
    pub etag: String,
    pub last_modified: Option<String>,
    modified: Option<SystemTime>,
}

impl Validators {
    pub fn new(seed: &str, source: &Metadata, parameters: &Parameters, format: &OutputFormat) -> Self {
        let modified = source.modified().ok();

        let nanos = modified
            .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        hasher.update(nanos.to_le_bytes());
        hasher.update(source.len().to_le_bytes());
        hasher.update(format!("{parameters:?}").as_bytes());
        hasher.update(format!("{format:?}").as_bytes());

        Self {
            etag: format!("\"{}\"", hex::encode(&hasher.finalize()[..8])),
            last_modified: modified.map(httpdate::fmt_http_date),
            modified,
        }
    }

    pub fn is_not_modified(&self, headers: &HeaderMap) -> bool {
        is_not_modified(headers, Some(&self.etag), self.modified)
    }

    pub fn apply(&self, builder: ResponseBuilder, cache_control: &str, vary: Option<&str>) -> ResponseBuilder {
        apply(
            builder,
            Some(&self.etag),
            self.last_modified.as_deref(),
            cache_control,
            vary,
        )
    }

    pub fn not_modified(&self, cache_control: &str, vary: Option<&str>) -> ResponseBuilder {
        self.apply(
            Response::builder().status(StatusCode::NOT_MODIFIED),
            cache_control,
            vary,
        )
    }
}

pub fn is_not_modified(headers: &HeaderMap, etag: Option<&str>, modified: Option<SystemTime>) -> bool {
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH).and_then(|value| value.to_str().ok()) {
        return etag.is_some_and(|etag| {
            if_none_match.trim() == "*" || if_none_match.split(',').any(|value| value.trim() == etag)
        });
    }

    let Some(modified) = modified else {
        return false;
    };

    let Some(if_modified_since) = headers.get(header::IF_MODIFIED_SINCE).and_then(|value| value.to_str().ok()) else {
        return false;
    };

    httpdate::parse_http_date(if_modified_since).is_ok_and(|since| modified <= since)
}

pub fn apply(mut builder: ResponseBuilder, etag: Option<&str>, last_modified: Option<&str>, cache_control: &str, vary: Option<&str>) -> ResponseBuilder {
    builder = builder.header(header::CACHE_CONTROL, cache_control);

    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }

    if let Some(last_modified) = last_modified {
        builder = builder.header(header::LAST_MODIFIED, last_modified);
    }

    if let Some(vary) = vary {
        builder = builder.header(header::VARY, vary);
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::params::RequestParams;
    use std::sync::Arc;
    use std::time::Duration;

    fn metadata() -> Metadata {
        std::fs::metadata(file!()).unwrap()
    }

    fn parameters() -> Parameters {
        Parameters::new(&Arc::new(Config::default()), RequestParams::default())
    }

    fn etag(seed: &str, parameters: &Parameters, format: &OutputFormat) -> String {
        Validators::new(seed, &metadata(), parameters, format).etag
    }

    fn headers(name: header::HeaderName, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, value.parse().unwrap());
        headers
    }

    #[test]
    fn the_same_request_gets_the_same_etag() {
        let parameters = parameters();

        assert_eq!(
            etag("seed", &parameters, &OutputFormat::Webp),
            etag("seed", &parameters, &OutputFormat::Webp)
        );
    }

    #[test]
    fn the_format_the_seed_and_the_parameters_all_change_the_etag() {
        let parameters = parameters();
        let unchanged = etag("seed", &parameters, &OutputFormat::Webp);

        assert_ne!(unchanged, etag("seed", &parameters, &OutputFormat::Avif));
        assert_ne!(unchanged, etag("other", &parameters, &OutputFormat::Webp));

        let mut resized = parameters;
        resized.width = Some(400);

        assert_ne!(unchanged, etag("seed", &resized, &OutputFormat::Webp));
    }

    #[test]
    fn a_config_change_changes_the_seed() {
        let mut config = Config::default();
        let unchanged = seed(&config);
        config.output.quality = crate::enums::output_quality::OutputQuality::High;

        assert_ne!(unchanged, seed(&config));
    }

    #[test]
    fn if_none_match_matches_a_list_and_a_wildcard() {
        assert!(is_not_modified(
            &headers(header::IF_NONE_MATCH, "\"a\", \"b\""),
            Some("\"b\""),
            None
        ));
        assert!(is_not_modified(
            &headers(header::IF_NONE_MATCH, "*"),
            Some("\"b\""),
            None
        ));
        assert!(!is_not_modified(
            &headers(header::IF_NONE_MATCH, "\"a\""),
            Some("\"b\""),
            None
        ));
    }

    #[test]
    fn if_none_match_wins_over_a_matching_if_modified_since() {
        let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let mut headers = headers(header::IF_NONE_MATCH, "\"stale\"");
        headers.insert(
            header::IF_MODIFIED_SINCE,
            httpdate::fmt_http_date(modified).parse().unwrap(),
        );

        assert!(!is_not_modified(&headers, Some("\"fresh\""), Some(modified)));
    }

    #[test]
    fn if_modified_since_rejects_a_source_modified_after_it() {
        let since = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let headers = headers(header::IF_MODIFIED_SINCE, &httpdate::fmt_http_date(since));

        assert!(is_not_modified(&headers, None, Some(since)));
        assert!(!is_not_modified(
            &headers,
            None,
            Some(since + Duration::from_secs(1))
        ));
    }
}
