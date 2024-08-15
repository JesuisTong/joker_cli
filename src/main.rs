use clap::{Parser, Subcommand};
use colog;

mod args;
mod joker;
mod utils;

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        short = 'c',
        value_name = "COOKIE",
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

    #[arg(long, short = 'P', help = "proxy", global = true)]
    proxy: Option<String>,

    #[arg(long, help = "joker version", global = true, default_value = "2")]
    version: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Start mining")]
    Mine(args::MineArgs),

    #[command(about = "Account info")]
    Info,

    #[command(about = "Mine records")]
    Records,
}

fn create_joker_instance(args: &Args) -> joker::JokerEnum {
    match args.version {
        2u8 => joker::JokerEnum::Joker2(joker::Joker2::new(
            "Joker".to_string(),
            args.cookie.clone().unwrap(),
            args.session_cookie.clone().unwrap(),
            args.authorization.clone().unwrap(),
            args.proxy.clone(),
            2,
        )),
        _ => joker::JokerEnum::Joker1(joker::Joker1::new(
            "Joker".to_string(),
            args.cookie.clone().unwrap(),
            args.session_cookie.clone().unwrap(),
            args.authorization.clone().unwrap(),
            args.proxy.clone(),
            2,
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();
    let args: Args = Args::parse();

    let mut joker_ins = create_joker_instance(&args);

    match args.command {
        Commands::Mine(mine_args) => {
            joker_ins.set_cores(mine_args.cores);
            joker_ins.do_loop().await?;
        }
        Commands::Info => {
            joker_ins.get_account_info().await?;
        }
        Commands::Records => {
            joker_ins.get_records().await?;
        }
    };

    Ok(())
}
