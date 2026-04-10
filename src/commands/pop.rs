use crate::patcher::patcher::Patcher;

pub fn pop(patcher: &mut Patcher) -> anyhow::Result<()> {
    patcher.pop()?;
    Ok(())
}
