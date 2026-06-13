use crate::config::Config;
use axum::http::Uri;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use hex::decode;
use tracing::debug;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(config: &Config, uri: &Uri) -> bool {
    if !config.security.signature_enabled {
        return true;
    }

    // Extract the signature from "token" URL parameter
    let path = uri.path().trim_start_matches('/');
    let query = uri.query().unwrap_or("");

    let params: Vec<&str> = query.split('&').collect();
    let token = params.iter().find(|&p| p.starts_with("token=")).map(|p| p.split('=').nth(1).unwrap_or(""));

    if let Some(signature) = token {
        let query = params.into_iter().filter(|&p| !p.starts_with("token=")).collect::<Vec<&str>>().join("&");
        let uri = format!("{}?{}", path, query);

        // Verify signature using SHA-256 HMAC
        let secret_key = config.security.signature_secret.as_bytes();
        let mut mac = HmacSha256::new_from_slice(secret_key).unwrap();
        mac.update(uri.as_bytes());

        let signature_bytes = match decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => {
                debug!("Invalid signature: {}", signature);
                return false;
            }
        };

        return mac.verify_slice(&signature_bytes).is_ok();
    }

    false
}