use crate::config::Config;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use hex::decode;
use tracing::debug;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(config: &Config, path: &str, query: Option<&str>) -> bool {
    if !config.security.signature_enabled {
        return true;
    }

    // Extract the signature from "token" URL parameter
    let path = path.trim_start_matches('/');
    let query = query.unwrap_or("");

    let params: Vec<&str> = query.split('&').collect();
    let token = params.iter().find(|&p| p.starts_with("token=")).map(|p| p.split('=').nth(1).unwrap_or(""));

    if let Some(signature) = token {
        let query = params.into_iter().filter(|&p| !p.starts_with("token=")).collect::<Vec<&str>>().join("&");

        let uri = if query.is_empty() {
          path.to_string()
        } else {
          format!("{path}?{query}")
        };

        // Verify signature using SHA-256 HMAC
        let secret_key = config.security.signature_secret.as_bytes();
        let mut mac = HmacSha256::new_from_slice(secret_key).unwrap();
        mac.update(uri.as_bytes());

        let signature_bytes = match decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => {
                debug!("Invalid signature: {signature}");
                return false;
            }
        };

        return mac.verify_slice(&signature_bytes).is_ok();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(secret: &str, data: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(data.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn verifies_path_with_diacritics_and_spaces() {
        let mut config = Config::default();
        config.security.signature_enabled = true;
        config.security.signature_secret = "secret".into();

        let path = "data/uploads/Ponozkový október.jpg";
        let token = sign("secret", &format!("{path}?w=820"));

        assert!(verify_signature(&config, path, Some(&format!("w=820&token={token}"))));
        assert!(!verify_signature(&config, path, Some(&format!("w=821&token={token}"))));
        assert!(!verify_signature(&config, path, Some("w=820")));
    }
}
