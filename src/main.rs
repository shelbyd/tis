use anyhow::ensure;
use std::process::Output;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    Sync(SyncOptions),
}

#[derive(StructOpt, Debug)]
struct SyncOptions {}

fn main() -> anyhow::Result<()> {
    let opts = Options::from_args();

    match &opts.command {
        Command::Sync(opts) => do_sync(opts),
    }
}

fn do_sync(_opts: &SyncOptions) -> anyhow::Result<()> {
    git("fetch", ["--prune"])?;

    ensure!(is_working_directory_clean()?, "Cannot sync a dirty working directory");

    unimplemented!("do_sync");
}

fn git<S: AsRef<str>>(command: &str, args: impl IntoIterator<Item = S>) -> anyhow::Result<String> {
    let args = args
        .into_iter()
        .map(|s| String::from(s.as_ref()))
        .collect::<Vec<_>>();

    let output = git_raw(command, &args)?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    Err(anyhow::anyhow!(
        "git command `{command} {args}`\n  failed with exit code {code}\n{stderr}",
        command = command,
        args = args.join(" "),
        code = output.status,
        stderr = String::from_utf8_lossy(&output.stderr),
    ))
}

fn git_raw(command: &str, args: &[String]) -> anyhow::Result<Output> {
    Ok(std::process::Command::new("git")
        .args([command].into_iter().chain(args.iter().map(|s| s.as_str())))
        .output()?)
}

fn is_working_directory_clean() -> anyhow::Result<bool> {
    Ok(git("status", ["--porcelain"])?.is_empty())
}
