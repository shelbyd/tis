use std::process::Output;

pub fn git<S: AsRef<str>>(
    command: &str,
    args: impl IntoIterator<Item = S>,
) -> anyhow::Result<String> {
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

pub fn git_raw(command: &str, args: &[String]) -> anyhow::Result<Output> {
    log::debug!("git {} {}", command, args.join(" "));

    Ok(std::process::Command::new("git")
        .args([command].into_iter().chain(args.iter().map(|s| s.as_str())))
        .output()?)
}

pub fn is_working_directory_clean() -> anyhow::Result<bool> {
    Ok(git("status", ["--porcelain"])?.is_empty())
}

pub fn run<S>(command: &str, args: impl IntoIterator<Item = S>) -> anyhow::Result<Output>
where
    S: AsRef<str>,
{
    Ok(std::process::Command::new(command)
        .args(
            args.into_iter()
                .map(|s| String::from(s.as_ref()))
                .collect::<Vec<_>>(),
        )
        .output()?)
}
