use anyhow::ensure;
use dialoguer::*;
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
            let eq = self.compare_to_remote(branch)?;

            let delta = match eq {
                BranchEq::Eq => {
                    log::info!("{}: Local branch matches origin, continuing", branch);
                    continue;
                }
                BranchEq::RemoteMissing => {
                    let input: String = Input::new()
                        .with_prompt(format!(
                            "{}: Remote does not have branch.\n\
                               (d) Delete local\n\
                               (p) Push to origin\n\
                               (n) Do nothing\n",
                            branch
                        ))
                        .default("n".to_string())
                        .interact_text()?;
                    match input.chars().next() {
                        Some('d') => {
                            log::warn!("{}: Deleting local branch", branch);
                            git("branch", ["-D", branch])?;
                        }
                        Some('p') => todo!("push to origin"),
                        Some('n') => {}
                        c => log::warn!("Unrecognized start of input '{:?}', doing nothing", c),
                    }
                    continue;
                }
                BranchEq::NotEq(delta) => delta,
            };

            match delta {
                BranchDelta::LocalAhead => {
                    log::warn!("{}: Pushing to origin", branch);
                    git("push", ["origin", branch])?;
                }
                BranchDelta::RemoteAhead(commit) => {
                    log::warn!("{}: Setting local branch to remote", branch);
                    git("branch", ["-f", branch, &commit]).or_else(|err| {
                        let should_pull = err
                            .to_string()
                            .contains("Cannot force update the current branch.");
                        if should_pull {
                            git("pull", ["--ff-only"])
                        } else {
                            Err(err)
                        }
                    })?;
                }
                BranchDelta::Diverged => {
                    log::warn!("{}: Local and origin have diverged. Doing nothing.", branch);
                }
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

    fn compare_to_remote(&self, branch: &str) -> anyhow::Result<BranchEq> {
        let remote_branch = format!("remotes/origin/{}", branch);

        let local_commit = git("rev-parse", [&branch])?;
        let remote_commit = match git("rev-parse", [&remote_branch]) {
            Ok(remote) => remote,
            Err(e) if e.to_string().contains("unknown revision") => {
                return Ok(BranchEq::RemoteMissing)
            }
            Err(e) => return Err(e),
        };

        if local_commit.trim() == remote_commit.trim() {
            return Ok(BranchEq::Eq);
        }

        let local_ahead_of_remote =
            git("merge-base", ["--is-ancestor", &remote_branch, branch]).is_ok();
        if local_ahead_of_remote {
            return Ok(BranchEq::NotEq(BranchDelta::LocalAhead));
        }

        let remote_ahead_of_local =
            git("merge-base", ["--is-ancestor", branch, &remote_branch]).is_ok();
        if remote_ahead_of_local {
            return Ok(BranchEq::NotEq(BranchDelta::RemoteAhead(
                remote_commit.trim().to_string(),
            )));
        }

        Ok(BranchEq::NotEq(BranchDelta::Diverged))
    }
}

#[derive(Debug)]
enum BranchEq {
    Eq,
    NotEq(BranchDelta),
    RemoteMissing,
}

#[derive(Debug)]
enum BranchDelta {
    LocalAhead,
    RemoteAhead(String),
    Diverged,
}
