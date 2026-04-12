use anyhow::bail;
use git2::Oid;

use crate::utils::git_utils;

pub fn get_patch(commit: Oid, parent: Option<Oid>) -> anyhow::Result<()> {
    let repo = git2::Repository::open(".")?;
    let commit = repo.find_commit(commit)?;
    let parent = match parent {
        Some(parent) => repo.find_commit(parent)?,
        None => match commit.parent_count() {
            0 => bail!("The specified commit has no parent."),
            1 => commit.parent(0)?,
            _ => bail!(
                "The specified commit has multiple parents. Please specify a parent commit with --parent option."
            ),
        },
    };
    let patch = git_utils::get_patch(&parent, &commit, &repo)?;
    println!("{}", patch);

    Ok(())
}
