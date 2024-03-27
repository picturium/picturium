use data_encoding::HEXLOWER;
use ring::hmac;

pub fn verify_hmac(data: &str, key: &str, code: &str) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes());
    let code = HEXLOWER.decode(code.as_bytes()).unwrap();
    hmac::verify(&key, data.as_bytes(), code.as_ref()).is_ok()
}

pub fn json_hash<T: serde::Serialize>(data: &T) -> String {
    let json = serde_json::to_string(data).unwrap();
    let hash = ring::digest::digest(&ring::digest::SHA256, json.as_bytes());
    HEXLOWER.encode(hash.as_ref())
}