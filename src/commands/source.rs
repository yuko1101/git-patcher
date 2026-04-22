use anyhow::Context;

use crate::patcher::{patcher::Patcher, sync_strategy::SyncStrategy};

pub fn sync_source(patcher: &mut Patcher) -> anyhow::Result<()> {
    let sync_strategy = std::env::var("GIT_PATCHER_SYNC_STRATEGY")
        .context("GIT_PATCHER_SYNC_STRATEGY environment variable is not set.")?
        .parse::<SyncStrategy>()?;
    patcher.sync_source(sync_strategy)?;
    Ok(())
}
