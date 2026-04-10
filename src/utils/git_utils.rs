use std::{
    io::Write,
    path::{Path, PathBuf},
};

use git2::Repository;

/// Tries to find the root (not a submodule) of the git repository by traversing up the directory tree.
pub fn find_root(path: &Path) -> Option<PathBuf> {
    match Repository::discover(path) {
        Ok(repo) => {
            let workdir = repo.workdir().unwrap().to_path_buf();
            if let Some(parent) = workdir.parent() {
                if let Ok(_) = Repository::discover(parent) {
                    return find_root(parent);
                }
            }
            Some(workdir)
        }
        Err(_) => None,
    }
}

pub fn write_patch_to_file(diff: &git2::Diff, path: &Path) -> anyhow::Result<()> {
    let mut file = std::fs::File::create(path)?;
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        let origin = line.origin();
        match origin {
            ' ' | '+' | '-' => {
                if file.write_all(&[origin as u8]).is_err() {
                    return false;
                }
            }
            _ => (),
        };

        file.write_all(line.content()).is_ok()
    })?;
    Ok(())
}
