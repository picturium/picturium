use data_encoding::HEXLOWER;
use ring::hmac;
use xxhash_rust::xxh3::xxh3_64;

pub fn verify_hmac(data: &str, key: &str, code: &str) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes());
    let code = HEXLOWER.decode(code.as_bytes()).unwrap();
    hmac::verify(&key, data.as_bytes(), code.as_ref()).is_ok()
}

pub fn string_hash(data: &str) -> String {
    xxh3_64(data.as_bytes()).to_string()
}

pub fn json_hash<T: serde::Serialize>(data: &T) -> String {
    xxh3_64(serde_json::to_string(data).unwrap().as_bytes()).to_string()
}