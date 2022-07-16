use anyhow::ensure;
use std::collections::HashSet;
use structopt::StructOpt;

use crate::utils::*;

#[derive(StructOpt, Debug)]
pub struct SyncOptions {}

impl SyncOptions {
    pub fn perform(&self) -> anyhow::Result<()> {
        git("fetch", ["--prune"])?;

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
                continue;
            }

            log::warn!("Pushing local branch '{}'", branch);
            git("push", ["--force", "origin", branch])?;
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
