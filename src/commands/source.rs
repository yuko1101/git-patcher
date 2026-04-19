use crate::patcher::patcher::Patcher;

pub fn sync_source(patcher: &mut Patcher) -> anyhow::Result<()> {
    patcher.sync_source()?;
    Ok(())
}
