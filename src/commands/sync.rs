use anyhow::ensure;
use std::collections::HashSet;
use structopt::StructOpt;

use crate::utils::*;

#[derive(StructOpt, Debug)]
pub struct SyncOptions {}

impl SyncOptions {
    pub fn perform(&self) -> anyhow::Result<()> {
        log::info!("Fetching remote");
        git("fetch", ["--prune"])?;
        log::info!("Remote fetched");

        ensure!(
            is_working_directory_clean()?,
            "Cannot sync a dirty working directory"
        );

        self.push_local_branches()?;

        Ok(())
    }

    fn push_local_branches(&self) -> anyhow::Result<()> {
        let all_branches = self.all_branches()?;
        let local_branches = all_branches.iter().filter(|s| !s.starts_with("remotes/"));

        for branch in local_branches {
            let remote = match all_branches.get(&format!("remotes/origin/{}", &branch)) {
                Some(r) => r,
                None => continue,
            };

            let local_commit = git("rev-parse", [&branch])?;
            let remote_commit = git("rev-parse", [&remote])?;
            if local_commit.trim() == remote_commit.trim() {
                log::info!("Local branch '{}' matches remote, continuing", branch);
                continue;
            }

            let mut confirm = dialoguer::Confirm::new();
            confirm
                .default(false)
                .wait_for_newline(true)
                .with_prompt(format!("Force push {0} to origin/{0}?", branch));

            if confirm.interact()? {
                log::warn!("Pushing local branch '{}'", branch);
                git("push", ["--force", "origin", branch])?;
            }
        }

        Ok(())
    }

    fn all_branches(&self) -> anyhow::Result<HashSet<String>> {
        Ok(git("branch", ["--all"])?
            .split("\n")
            .map(|s| s.trim().trim_start_matches("* "))
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }
}
