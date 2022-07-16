use anyhow::ensure;
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

        unimplemented!("perform");
    }
}
