use clap::Parser;

#[derive(Parser, Debug)]
pub struct MineArgs {
    #[arg(
        long,
        value_name = "cores",
        help = "Cpu cores you use",
        default_value = "2"
    )]
    pub cores: u8,
}
