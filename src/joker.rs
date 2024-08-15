use std::sync::{Arc, RwLock};

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, SET_COOKIE},
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

    async fn get_records(&self) -> Result<(), Box<dyn std::error::Error>>;

    async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Joker1 {
    name: String,
    cookie: String,
    session_cookie: String,
    authorization: String,
    proxy: Option<String>,
    core: u8,
}

impl Joker1 {
    pub fn new(
        name: String,
        cookie: String,
        session_cookie: String,
        authorization: String,
        proxy: Option<String>,
        core: u8,
    ) -> Self {
        Self {
            name,
            cookie,
            session_cookie,
            authorization,
            proxy,
            core,
        }
    }
}

impl BaseJoker for Joker1 {
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
            HeaderValue::from_str(&format!("Bearer {}", &self.authorization)).unwrap(),
        );

        (client, headers)
    }

    async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut claim_cnt = 0;
        let mut total_time = 0f64;
        loop {
            let (mission_hash, require) = self.get_mission().await?;
            utils::format_println(&self.name, &format!("get mission: {}", mission_hash));
            let timer = Instant::now();
            let (nonce, hash) = self.find_hash(&mission_hash, &require, self.core).await;
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
                .post("https://test.blockjoker.org/api/v1/missions")
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

                if bui["result"].is_string() {
                    return Ok((
                        bui["result"].as_str().unwrap().to_string(),
                        String::from("000000"),
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
            #[cfg(debug_assertions)]
            println!("nonce: {}\nhash: {}", nonce, hash);

            let response = client
                .post("https://test.blockjoker.org/api/v1/missions/nonce")
                .headers(headers.clone())
                .body(
                    json!({
                        "nonce": nonce,
                        "hash": hash,
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
                return Ok(());
            } else {
                utils::format_error(
                    &self.name,
                    &format!("claim failed {}, {}", status, response.text().await?),
                );
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

    async fn get_records(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("v1 don't have records");

        Ok(())
    }

    async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .get("https://test2.blockjoker.org/api/v1/accounts")
            .headers(headers)
            .send()
            .await?;

        println!("status: {:?}", response.status());
        println!("account info: {:#?}", response.text().await?);

        Ok(())
    }
}
pub struct Joker2 {
    name: String,
    cookie: String,
    session_cookie: String,
    authorization: String,
    proxy: Option<String>,
    core: u8,
}

impl Joker2 {
    pub fn new(
        name: String,
        cookie: String,
        session_cookie: String,
        authorization: String,
        proxy: Option<String>,
        core: u8,
    ) -> Self {
        Self {
            name,
            cookie,
            session_cookie,
            authorization,
            proxy,
            core,
        }
    }
}

impl BaseJoker for Joker2 {
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
            HeaderValue::from_str(&format!("Bearer {}", &self.authorization)).unwrap(),
        );

        (client, headers)
    }

    async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut claim_cnt = 0;
        let mut total_time = 0f64;
        loop {
            let (mission_hash, require) = self.get_mission().await?;
            utils::format_println(&self.name, &format!("get mission: {}", mission_hash));
            let timer = Instant::now();
            let (nonce, hash) = self.find_hash(&mission_hash, &require, self.core).await;
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

    async fn claim(
        &mut self,
        nonce: String,
        hash: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();

        loop {
            #[cfg(debug_assertions)]
            println!("nonce: {}\nhash: {}", nonce, hash);

            let response = client
                .post("https://test2.blockjoker.org/api/v2/missions/nonce")
                .headers(headers.clone())
                .body(
                    json!({
                        "nonce": nonce
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
                return Ok(());
            } else {
                utils::format_error(
                    &self.name,
                    &format!("claim failed {}, {}", status, response.text().await?),
                );
            }
            sleep(Duration::from_millis(300)).await;
        }
    }

    async fn get_mission(&mut self) -> Result<(String, String), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();

        loop {
            let response = client
                .post("https://test2.blockjoker.org/api/v2/missions")
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

    async fn get_records(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .get("https://test2.blockjoker.org/api/v2/missions/records")
            .headers(headers)
            .send()
            .await?;

        println!("status: {:?}", response.status());
        println!("records: {:#?}", response.text().await?);

        Ok(())
    }

    async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .get("https://test2.blockjoker.org/api/v2/accounts")
            .headers(headers)
            .send()
            .await?;

        println!("status: {:?}", response.status());
        println!("account info: {:#?}", response.text().await?);

        Ok(())
    }
}

pub enum JokerEnum {
    Joker1(Joker1),
    Joker2(Joker2),
}

impl JokerEnum {
    pub fn set_cores(&mut self, cores: u8) {
        match self {
            JokerEnum::Joker1(j) => j.core = cores,
            JokerEnum::Joker2(j) => j.core = cores,
        }
    }

    pub async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            JokerEnum::Joker1(j) => j.do_loop().await,
            JokerEnum::Joker2(j) => j.do_loop().await,
        }
    }

    pub async fn get_records(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            JokerEnum::Joker1(j) => j.get_records().await,
            JokerEnum::Joker2(j) => j.get_records().await,
        }
    }

    pub async fn get_account_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            JokerEnum::Joker1(j) => j.get_account_info().await,
            JokerEnum::Joker2(j) => j.get_account_info().await,
        }
    }
}
