use crate::{config::config::Config, patcher::patcher::Patcher};

pub fn sync_source(patcher: &mut Patcher, sync_strategy: &Config) -> anyhow::Result<()> {
    patcher.sync_source(&sync_strategy.sync_strategy)?;
    Ok(())
}
