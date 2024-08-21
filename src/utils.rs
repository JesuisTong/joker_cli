use chrono::Local;
use log::{error, info};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, PRAGMA, REFERRER_POLICY, USER_AGENT,
};

pub fn now() -> String {
    Local::now().format("%F %T").to_string()
}

pub fn format_println(name: &str, msg: &str) {
    info!("[{}] [{}]: {}", now(), name, msg);
}
pub fn format_error(name: &str, msg: &str) {
    error!("[{}] [{}]: {}", now(), name, msg);
}

pub fn init_headers(h: &mut HeaderMap) -> &mut HeaderMap {
    h.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    h.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6"),
    );
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    h.insert("priority", HeaderValue::from_static("u=1, i"));
    h.insert("sec-ch-ua", HeaderValue::from_static("\"\""));
    h.insert("sec-ch-ua-mobile", HeaderValue::from_static("?1"));
    h.insert("sec-ch-ua-platform", HeaderValue::from_static("\"\""));
    h.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    h.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    h.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    h.insert(
        REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1 Edg/126.0.0.0"));

    h
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
