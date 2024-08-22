use clap::{Parser, Subcommand};
use colog;
use joker::BaseJoker;

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
    cookie: String,

    #[clap(
        short = 'S',
        long = "session_cookie",
        help = "session_cookie",
        default_value = "",
        global = true
    )]
    session_cookie: Option<String>,

    #[arg(long, short = 'A', help = "authorization", global = true)]
    authorization: String,

    #[arg(long, help = "cf_response", global = true)]
    cf_response: Option<String>,

    #[arg(long, short = 'P', help = "proxy", global = true)]
    proxy: Option<String>,

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

fn create_joker_instance(args: &Args) -> joker::Joker {
    joker::Joker::new(
        "Joker".to_string(),
        args.cookie.clone(),
        args.session_cookie.clone().unwrap(),
        format!("Bearer {}", args.authorization.clone()),
        args.cf_response.clone(),
        None,
        args.proxy.clone(),
        2,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();
    let args: Args = Args::parse();

    let mut joker_ins = create_joker_instance(&args);

    match args.command {
        Commands::Mine(mine_args) => {
            joker_ins.set_threads(mine_args.threads);
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
