use std::sync::{Arc, RwLock};

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, ORIGIN, REFERER, SET_COOKIE},
    StatusCode,
};
use serde_json::json;
use tokio::time::{sleep, Duration, Instant};

use crate::utils;

pub trait BaseJoker {
    fn request(&self) -> (reqwest::Client, HeaderMap);

    async fn get_mission(&mut self) -> Result<(String, String), Box<dyn std::error::Error>>;

    async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    async fn find_hash(&self, mission_hash: &str, require: &str, cores: u8) -> (String, String);

    async fn claim(
        &mut self,
        nonce: String,
        hash: String,
    ) -> Result<(), Box<dyn std::error::Error>>;

    async fn get_records(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Joker {
    name: String,
    cookie: String,
    session_cookie: String,
    authorization: String,
    cf_response: String,
    pow_id: Option<String>,
    proxy: Option<String>,
    threads: u8,
}

impl Joker {
    pub fn new(
        name: String,
        cookie: String,
        session_cookie: String,
        authorization: String,
        cf_response: String,
        pow_id: Option<String>,
        proxy: Option<String>,
        core: u8,
    ) -> Self {
        Self {
            name,
            cookie,
            session_cookie,
            authorization,
            cf_response,
            pow_id,
            proxy,
            threads: core,
        }
    }

    pub fn set_threads(&mut self, core: u8) {
        self.threads = core;
    }
}

impl BaseJoker for Joker {
    fn request(&self) -> (reqwest::Client, HeaderMap) {
        let client = if let Some(p) = &self.proxy {
            reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(p).unwrap())
                .build()
                .unwrap()
        } else {
            reqwest::Client::new()
        };
        let mut headers = HeaderMap::new();
        utils::init_headers(&mut headers);

        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("{} {}", &self.cookie, &self.session_cookie)).unwrap(),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&self.authorization).unwrap(),
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://blockjoker.org/home"),
        );
        headers.insert(ORIGIN, HeaderValue::from_static("https://blockjoker.org"));

        (client, headers)
    }

    async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut claim_cnt = 0;
        let mut total_time = 0f64;
        let records = self.get_records().await?;
        self.pow_id = records.into_iter().next();
        loop {
            let (mission_hash, require) = self.get_mission().await?;
            utils::format_println(&self.name, &format!("get mission: {}", mission_hash));
            let timer = Instant::now();
            let (nonce, hash) = self.find_hash(&mission_hash, &require, self.threads).await;
            total_time += timer.elapsed().as_secs_f64();
            self.claim(nonce, hash).await?;
            claim_cnt += 1;
            utils::format_println(
                &self.name,
                &format!(
                    "cal avg time: ({} secs)\nclaim count: {}",
                    total_time / claim_cnt as f64,
                    claim_cnt
                ),
            );
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn get_mission(&mut self) -> Result<(String, String), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();

        loop {
            let response = client
                .post("https://blockjoker.org/api/v2/missions")
                .body(
                    json!({
                    "cf_response": self.cf_response
                    })
                    .to_string(),
                )
                .headers(headers.clone())
                .send()
                .await;

            if response.is_err() {
                utils::format_error(
                    &self.name,
                    &format!("get mission failed {:?}", response.err()),
                );
                sleep(Duration::from_millis(1000)).await;
                continue;
            }

            let response = response.unwrap();
            let status = response.status();
            if status == StatusCode::OK {
                let set_headers: Vec<String> = response
                    .headers()
                    .get_all(SET_COOKIE)
                    .iter()
                    .map(|v| {
                        let ck = cookie::Cookie::parse(v.to_str().unwrap()).unwrap();
                        let (name, value) = ck.name_value();
                        let name_value = name.to_owned() + "=" + value;
                        name_value
                    })
                    .collect();
                self.session_cookie = set_headers.join("; ");

                let bui: &serde_json::Value = &response.json().await?;

                if bui["result"].is_object() {
                    return Ok((
                        bui["result"]["payload"].as_str().unwrap().to_string(),
                        bui["result"]["require"].as_str().unwrap().to_string(),
                    ));
                }
            }
            utils::format_error(&self.name, &format!("get_mission failed, {:?}", status));
            sleep(Duration::from_millis(1000)).await;
        }
    }

    async fn claim(
        &mut self,
        nonce: String,
        hash: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();

        loop {
            utils::format_println(
                &self.name,
                &format!(
                    "nonce: {}\nhash: {}\npow_id: {:?}",
                    nonce, hash, &self.pow_id
                ),
            );

            if let Some(p) = &self.pow_id {
                let response = client
                    .post("https://blockjoker.org/api/v2/missions/nonce")
                    .headers(headers.clone())
                    .body(
                        json!({
                            "nonce": nonce,
                            "pow_id": p,
                        })
                        .to_string(),
                    )
                    .send()
                    .await;

                if response.is_err() {
                    utils::format_error(&self.name, &format!("claim failed {:?}", response.err()));
                    sleep(Duration::from_millis(300)).await;
                    continue;
                }

                let response = response.unwrap();
                let status = response.status();
                if status == StatusCode::OK {
                    let set_headers: Vec<String> = response
                        .headers()
                        .get_all(SET_COOKIE)
                        .iter()
                        .map(|v| {
                            let ck = cookie::Cookie::parse(v.to_str().unwrap()).unwrap();
                            let (name, value) = ck.name_value();
                            let name_value = name.to_owned() + "=" + value;
                            name_value
                        })
                        .collect();
                    self.session_cookie = set_headers.join("; ");

                    let result: serde_json::Value = response.json().await?;
                    let v: Vec<String> = result["result"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x["pow_id"].as_str().unwrap().to_string())
                        .collect();
                    if v.len() > 0 {
                        self.pow_id = v.into_iter().next();
                    }

                    return Ok(());
                } else {
                    utils::format_error(
                        &self.name,
                        &format!("claim failed {}, {}", status, response.text().await?),
                    );
                }
            }
            sleep(Duration::from_millis(300)).await;
        }
    }

    async fn find_hash(&self, mission_hash: &str, require: &str, cores: u8) -> (String, String) {
        let core_ids = core_affinity::get_core_ids().unwrap();
        let global_match_nonce = Arc::new(RwLock::new("".to_string()));

        let handles = core_ids
            .into_iter()
            .map(|i| {
                let global_match_nonce = Arc::clone(&global_match_nonce);

                std::thread::spawn({
                    let mission_hash = mission_hash.to_owned().clone();
                    let require = require.to_owned().clone();
                    move || {
                        // Return if core should not be used
                        if (i.id as u8).ge(&cores) {
                            return (String::from(""), String::from(""));
                        }

                        // Pin to core
                        let _ = core_affinity::set_for_current(i);

                        // Start hashing
                        #[cfg(debug_assertions)]
                        let timer = Instant::now();

                        loop {
                            // Create hash
                            let nonce = utils::generate_nonce(48);
                            let str = format!("{}{}", mission_hash, nonce);
                            let best_match = utils::generate_hash(&str);

                            // Check if hash is valid
                            if best_match.starts_with(&require) {
                                #[cfg(debug_assertions)]
                                println!(
                                    "Hash found: {} ({}s)\nNonce: {}",
                                    best_match,
                                    timer.elapsed().as_secs_f64(),
                                    nonce
                                );

                                let copy_best_match_nonce = nonce.clone();
                                *global_match_nonce.write().unwrap() = copy_best_match_nonce;
                                return (nonce, best_match);
                            }

                            let global_match_hash = global_match_nonce.read().unwrap().clone();
                            if global_match_hash != "" {
                                break;
                            }
                        }

                        (String::from(""), String::from(""))
                    }
                })
            })
            .map(|x| x.join())
            .filter(|x| {
                if let Ok((v, _)) = x {
                    return v != "";
                }
                false
            })
            .take(1)
            .next()
            .unwrap();

        handles.unwrap()
    }

    async fn get_records(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .get("https://blockjoker.org/api/v2/missions/pow-records")
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        println!("status: {:?}", status);
        if status != StatusCode::OK {
            utils::format_error(&self.name, &format!("get_records failed, {:?}", status));
            return Ok(vec![]);
        }

        let result: serde_json::Value = response.json().await?;

        let v: Vec<String> = result["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["pow_id"].as_str().unwrap().to_string())
            .collect();

        Ok(v)
    }

    async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .get("https://blockjoker.org/api/v2/accounts")
            .headers(headers)
            .send()
            .await?;

        println!("status: {:?}", response.status());
        println!("account info: {:#?}", response.text().await?);

        Ok(())
    }
}
