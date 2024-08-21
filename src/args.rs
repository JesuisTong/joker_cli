use clap::Parser;

#[derive(Parser, Debug)]
pub struct MineArgs {
    #[arg(
        long,
        value_name = "threads",
        help = "How many threads you use",
        default_value = "2"
    )]
    pub threads: u8,
}
