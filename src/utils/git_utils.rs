use std::path::{Path, PathBuf};

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
