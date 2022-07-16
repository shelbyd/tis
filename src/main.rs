use structopt::StructOpt;

mod commands;
pub mod utils;

pub use utils::*;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    Sync(commands::SyncOptions),
}

fn main() -> anyhow::Result<()> {
    let opts = Options::from_args();

    match &opts.command {
        Command::Sync(opts) => opts.perform(),
    }
}
