use crate::patcher::patcher::Patcher;

pub fn push(patcher: &mut Patcher) -> anyhow::Result<()> {
    patcher.push()?;
    Ok(())
}
