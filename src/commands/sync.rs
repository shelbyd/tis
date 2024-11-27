use dialoguer::*;
use std::collections::{BTreeSet, HashSet};
use structopt::StructOpt;

use crate::utils::*;

#[derive(StructOpt, Debug)]
pub struct SyncOptions {
    master: Option<String>,
}

impl SyncOptions {
    pub fn perform(&self) -> anyhow::Result<()> {
        log::info!("Fetching remote");
        git("fetch", ["--prune"])?;
        log::info!("Remote fetched");

        with_clean_directory(|| self.sync_local_branches())?;

        Ok(())
    }

    fn sync_local_branches(&self) -> anyhow::Result<()> {
        let master = match &self.master {
            Some(m) => m.clone(),
            None => {
                let remote = git("remote", ["show", "origin"])?;

                let Some(captures) = regex::Regex::new("HEAD branch: (.+)")?.captures(&remote)
                else {
                    anyhow::bail!("HEAD branch not found in `git remote show origin`");
                };

                captures[1].to_string()
            }
        };
        log::info!("Master branch: {master:?}");

        let all_branches = self.all_branches()?;
        let local_branches: BTreeSet<_> = all_branches
            .iter()
            .filter(|s| !s.starts_with("remotes/"))
            .collect();

        for branch in local_branches {
            let eq = self.compare_to_remote(branch)?;

            if branch == &master {
                match eq {
                    BranchEq::RemoteMissing => {
                        anyhow::bail!("Remote branch {} missing", master);
                    }

                    BranchEq::NotEq(BranchDelta::LocalAhead) => {
                        let prompt =
                            format!("Local branch {} ahead of origin. Reset to origin?", master);
                        if Confirm::new().with_prompt(prompt).interact()? {
                            set_local_to(&branch, &format!("origin/{}", branch))?;
                            continue;
                        }
                    }
                    BranchEq::NotEq(BranchDelta::Diverged) => {
                        let prompt = format!(
                            "Local branch {} diverged from origin. Reset to origin?",
                            master
                        );
                        if Confirm::new().with_prompt(prompt).interact()? {
                            set_local_to(&branch, &format!("origin/{}", branch))?;
                            continue;
                        }
                    }

                    BranchEq::NotEq(BranchDelta::RemoteAhead(_)) | BranchEq::Eq => {}
                }
            }

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
                            log::info!("{}: Deleting local branch", branch);

                            if git("rev-parse", ["--abbrev-ref", "HEAD"])?.trim() == branch {
                                log::warn!("Trying to delete currently checked out branch, checking out {}", master);
                                git("checkout", [&master])?;
                            }

                            git("branch", ["-D", branch])?;
                        }
                        Some('p') => {
                            push_branch(branch)?;
                        }
                        Some('n') => {}
                        c => log::warn!("Unrecognized start of input '{:?}', doing nothing", c),
                    }
                    continue;
                }
                BranchEq::NotEq(delta) => delta,
            };

            match delta {
                BranchDelta::LocalAhead => {
                    log::info!("{}: Pushing to origin", branch);
                    git("push", ["origin", branch])?;
                }
                BranchDelta::RemoteAhead(commit) => {
                    set_local_to(branch, &commit)?;
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

fn set_local_to(branch: &str, git_ref: &str) -> Result<(), anyhow::Error> {
    log::info!("{}: Setting local branch to remote", branch);
    git("branch", ["-f", branch, git_ref]).or_else(|err| {
        let should_pull = err.to_string().contains("cannot force update the branch");
        if should_pull {
            git("pull", ["--ff-only"])
        } else {
            Err(err)
        }
    })?;
    Ok(())
}

fn push_branch(branch: &String) -> Result<(), anyhow::Error> {
    log::info!("{}: Pushing to remote", branch);
    git("push", ["-u", "origin", branch])?;

    let open = ["xdg-open", "open"]
        .iter()
        .find(|cmd| run("which", [cmd]).is_ok_and(|o| o.status.success()));
    let Some(open) = open else {
        log::warn!("No command found to open PR");
        return Ok(());
    };

    if !Confirm::new().with_prompt("Open PR?").interact()? {
        return Ok(());
    }

    let url = git("remote", ["get-url", "origin"])?;
    let org_repo = url
        .trim()
        .strip_prefix("git@github.com:")
        .and_then(|no_prefix| no_prefix.strip_suffix(".git"))
        .ok_or_else(|| anyhow::anyhow!("Unrecognized origin remote url {url:?}"))?;

    let create_pr_url = format!("https://github.com/{org_repo}/compare/{branch}?expand=1");
    run(open, [create_pr_url])?;

    Ok(())
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
