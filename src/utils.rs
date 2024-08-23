use chrono::Local;
use log::{error, info};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

pub fn now() -> String {
    Local::now().format("%F %T").to_string()
}

pub fn format_println(name: &str, msg: &str) {
    info!("[{}] [{}]: {}", now(), name, msg);
}
pub fn format_error(name: &str, msg: &str) {
    error!("[{}] [{}]: {}", now(), name, msg);
}

pub fn generate_nonce(length: usize) -> String {
    let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = thread_rng();
    let result: String = (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();
    result
}

pub fn generate_hash(data: &str) -> String {
    let hash = Sha256::digest(data);
    format!("{:x}", hash)
}
