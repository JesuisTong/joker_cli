use clap::{Parser, Subcommand};
use colog;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, SET_COOKIE};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::sleep;

mod utils;

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        short = 'c',
        value_name = "cookie",
        help = "your website cookie",
        global = true
    )]
    cookie: Option<String>,

    #[clap(
        short = 'S',
        long = "session_cookie",
        help = "session_cookie",
        default_value = "",
        global = true
    )]
    session_cookie: Option<String>,

    #[arg(long, short = 'A', help = "authorization.", global = true)]
    authorization: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
struct MineArgs {
    #[arg(
        long,
        value_name = "cores",
        help = "Cpu core you use",
        default_value = "2"
    )]
    cores: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Start mining")]
    Mine(MineArgs),
}

#[derive(Serialize, Deserialize, Debug)]
struct TapData {
    number_gem: f32,
    number_ec: i32,
    level: i32,
    base_rate: f32,
    min_ec: i32,
    number_tap: i64,
}

#[derive(Debug)]
enum JokerErr {
    GetMissionErr,
}

impl Display for JokerErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for JokerErr {}

struct Joker {
    name: String,
    cookie: String,
    session_cookie: String,
    authorization: String,
    core: u8,
}

impl Joker {
    fn new(
        name: String,
        cookie: String,
        session_cookie: String,
        authorization: String,
        core: u8,
    ) -> Self {
        Self {
            name,
            cookie,
            session_cookie,
            authorization,
            core,
        }
    }

    fn request(&self) -> (reqwest::Client, HeaderMap) {
        let client = reqwest::Client::new();
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

    async fn get_mission(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let (client, headers) = self.request();

        let response = client
            .post("https://test2.blockjoker.org/api/v1/missions")
            .headers(headers)
            .send()
            .await?;

        if response.status() == StatusCode::OK {
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
                return Ok(bui["result"].as_str().unwrap().to_string());
            }
        }

        utils::format_error(&self.name, "get_mission failed");
        Err(Box::new(JokerErr::GetMissionErr))
    }

    async fn do_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut claim_cnt = 0;
        let mut total_time = 0f64;
        loop {
            let mission_hash = self.get_mission().await?;
            utils::format_println(&self.name, &format!("get mission: {}", mission_hash));
            let timer = Instant::now();
            let nonce = self.find_hash_par(&mission_hash, self.core).await;
            self.claim(nonce).await?;
            total_time += timer.elapsed().as_secs_f64();
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

    async fn find_hash_par(&self, mission_hash: &str, cores: u8) -> String {
        let core_ids = core_affinity::get_core_ids().unwrap();
        let global_match_nonce = Arc::new(RwLock::new("".to_string()));

        let handles = core_ids
            .into_iter()
            .map(|i| {
                let global_match_nonce = Arc::clone(&global_match_nonce);

                std::thread::spawn({
                    let mission_hash = mission_hash.to_owned().clone();
                    move || {
                        // Return if core should not be used
                        if (i.id as u8).ge(&cores) {
                            return String::from("");
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
                            if best_match.starts_with("00000") {
                                #[cfg(debug_assertions)]
                                println!(
                                    "Hash found: {} ({}s)\nNonce: {}",
                                    best_match,
                                    timer.elapsed().as_secs_f64(),
                                    nonce
                                );

                                let copy_best_match_nonce = nonce.clone();
                                *global_match_nonce.write().unwrap() = copy_best_match_nonce;
                                return nonce;
                            }

                            let global_match_hash = global_match_nonce.read().unwrap().clone();
                            if global_match_hash != "" {
                                break;
                            }
                        }

                        String::from("")
                    }
                })
            })
            .map(|x| x.join())
            .filter(|x| {
                if let Ok(nonce) = x {
                    return nonce != "";
                }
                false
            })
            .take(1)
            .next()
            .unwrap();

        // Join handles and return best nonce
        handles.unwrap()
        // let mut best_nonce = String::from("");
        // for h in handles {
        //     if let Ok(nonce) = h {
        //         best_nonce = nonce;
        //         break;
        //     }
        // }

        // best_nonce
    }

    async fn claim(&mut self, nonce: String) -> Result<(), Box<dyn std::error::Error>> {
        let (client, headers) = self.request();
        let response = client
            .post("https://test2.blockjoker.org/api/v1/missions/nonce")
            .headers(headers)
            .body(
                json!({
                    "nonce": nonce
                })
                .to_string(),
            )
            .send()
            .await?;

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
        } else {
            utils::format_error(
                &self.name,
                &format!("claim failed {}, {}", status, response.text().await?),
            );
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();
    let args: Args = Args::parse();

    let cores = match args.command {
        Commands::Mine(mine_args) => mine_args.cores,
    };

    let mut joker = Joker::new(
        "Joker".to_string(),
        args.cookie.unwrap(),
        args.session_cookie.unwrap(),
        args.authorization.unwrap(),
        cores,
    );

    joker.do_loop().await?;

    Ok(())
}
