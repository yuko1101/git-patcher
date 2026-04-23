use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum SyncStrategy {
    /// Sync by creating a new commit using the tree of the new source.
    /// This records the current state as a single update, extending the history without modifying past commits.
    Snapshot,
    /// Sync by fetching the upstream repository and replaying the patches to reconstruct the commit history.
    /// Requires a force push if the existing history is modified.
    Reconstruct,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        SyncStrategy::Snapshot
    }
}

impl SyncStrategy {
    pub fn get_branch_name(&self) -> &'static str {
        match self {
            SyncStrategy::Snapshot => "git-patcher/snapshot",
            SyncStrategy::Reconstruct => "git-patcher/reconstructed",
        }
    }
}
