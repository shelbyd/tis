use structopt::StructOpt;

mod commands;
pub mod utils;

pub use utils::*;

#[derive(StructOpt, Debug)]
struct Options {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    /// Timestamp (sec, ms, ns, none)
    #[structopt(short = "t", long = "timestamp")]
    ts: Option<stderrlog::Timestamp>,

    /// Command to run.
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Synchronize local and remote repositories.
    Sync(commands::SyncOptions),
}

fn main() -> anyhow::Result<()> {
    let opts = Options::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(opts.quiet)
        .verbosity(match opts.verbose {
            0 => log::Level::Warn,
            1 => log::Level::Info,
            2 => log::Level::Debug,
            3 | _ => log::Level::Trace,
        })
        .timestamp(opts.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()?;

    match &opts.command {
        Command::Sync(opts) => opts.perform(),
    }
}
