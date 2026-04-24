use std::path::PathBuf;

use anyhow::{Context, Ok, bail};
use git2::{ApplyLocation, Diff, Index, Repository, Signature, build::CheckoutBuilder};

use crate::{
    patcher::{
        internal_state::InternalState, patch_series::PatchSeries, sync_strategy::SyncStrategy,
    },
    utils::{self, patch_utils::generate_patch_name},
};

pub struct Patcher {
    pub root: PathBuf,
    pub root_repo: Repository,
    pub upstream_path: PathBuf,
    pub upstream_repo: Repository,
    pub state: InternalState,
    pub patches: PathBuf,
    pub series: PathBuf,
}

impl Patcher {
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        let root_repo = Repository::open(&root)?;
        let upstream_path = root.join("upstream");
        let upstream_repo = Repository::open(&upstream_path)?;

        let state = InternalState::load(root.join(".git-patcher"))?;

        Ok(Self {
            root: root.clone(),
            root_repo,
            upstream_path,
            upstream_repo,
            state,
            patches: root.join("patches"),
            series: root.join("patches").join("series"),
        })
    }

    pub fn get_patch_series(&self) -> anyhow::Result<PatchSeries> {
        PatchSeries::new(&self.patches, &self.series)
    }

    pub fn push(&mut self) -> anyhow::Result<()> {
        self.state.target_revision = Some(self.upstream_repo.head()?.target().unwrap().to_string());
        self.state.save()?;

        let mut patch_series = self.get_patch_series()?;
        let mut patches_consumed = false;
        for patch in patch_series.consumer() {
            let patch_bytes = patch.1?;
            println!("Applying patch: {}", patch.0.display());
            let diff = Diff::from_buffer(&patch_bytes)?;
            self.upstream_repo
                .apply(&diff, ApplyLocation::Index, None)?;

            let mut index = self.upstream_repo.index()?;
            let tree_id = index.write_tree()?;
            let tree = self.upstream_repo.find_tree(tree_id)?;

            let parent = self.upstream_repo.head()?.peel_to_commit()?;
            let patch_metadata = utils::patch_utils::parse_patch_metadata(&patch_bytes)?;

            let new_commit_oid = self.upstream_repo.commit(
                Some("HEAD"),
                &patch_metadata.author.as_signature()?,
                &patch_metadata.committer.as_signature()?,
                &patch_metadata.commit_message,
                &tree,
                &[&parent],
            )?;
            validate_hash(
                &parent.id(),
                &patch_metadata.parent_hash,
                &new_commit_oid,
                &patch_metadata.commit_hash,
            );

            patches_consumed = true;
        }

        if patches_consumed {
            patch_series.save()?;

            let mut opts = CheckoutBuilder::new();
            opts.force();
            self.upstream_repo.checkout_head(Some(&mut opts))?;
        }

        Ok(())
    }

    pub fn pop(&mut self) -> anyhow::Result<()> {
        let Some(target_revision) = &self.state.target_revision else {
            bail!("No target revision found in state. Cannot pop.");
        };

        let target_oid = git2::Oid::from_str(target_revision)?;

        let mut patch_series = self.get_patch_series()?;
        let mut revwalk = self.upstream_repo.revwalk()?;
        revwalk.push_head()?;
        let diff_commits = revwalk
            .filter_map(|oid| oid.ok())
            .take_while(|oid| *oid != target_oid)
            .collect::<Vec<_>>()
            .into_iter()
            .rev();

        let target_commit = self.upstream_repo.find_commit(target_oid)?;

        let mut parent = target_commit.clone();
        let mut patches_pushed = false;

        let offset = patch_series.len();
        let patch_count = diff_commits.len() + offset;
        let mut index = offset;

        for oid in diff_commits {
            println!("Generating patch for commit: {}", oid);
            let commit = self.upstream_repo.find_commit(oid)?;

            let patch_path = self
                .patches
                .join(generate_patch_name(&commit, index, patch_count));
            utils::patch_utils::write_patch_to_file(
                &parent,
                &commit,
                &self.upstream_repo,
                &patch_path,
            )?;
            patch_series.push_patch(patch_path)?;

            parent = commit;
            patches_pushed = true;
            index += 1;
        }

        if patches_pushed {
            patch_series.save()?;

            self.upstream_repo
                .reset(target_commit.as_object(), git2::ResetType::Hard, None)?;
        }

        self.state.target_revision = None;
        self.state.save()?;
        Ok(())
    }

    pub fn sync_source(&mut self, sync_strategy: &SyncStrategy) -> anyhow::Result<()> {
        let branch_name = sync_strategy.get_branch_name();
        match sync_strategy {
            SyncStrategy::Snapshot => self.sync_source_by_snapshot(branch_name)?,
            SyncStrategy::Reconstruct => self.sync_source_by_reconstruct(branch_name)?,
        }
        Ok(())
    }

    fn sync_source_by_snapshot(&mut self, branch_name: &str) -> anyhow::Result<()> {
        // fetch upstream tree to root
        let upstream_head_tree = self.upstream_repo.head()?.peel_to_tree()?;
        let mut remote = self
            .root_repo
            .remote_anonymous(&self.upstream_path.to_string_lossy())?;
        println!(
            "Fetching upstream head tree {} to root repository",
            upstream_head_tree.id()
        );
        remote.fetch(&[upstream_head_tree.id().to_string()], None, None)?;

        let mut tree = self.root_repo.find_tree(upstream_head_tree.id())?;
        let patch_series = self.get_patch_series()?;
        for patch in patch_series.peeker() {
            let patch_bytes = patch.1?;
            println!("Applying patch: {}", patch.0.display());
            let diff = Diff::from_buffer(&patch_bytes)?;
            tree = self.root_repo.find_tree(
                self.root_repo
                    .apply_to_tree(&tree, &diff, None)?
                    .write_tree_to(&self.root_repo)?,
            )?;
        }

        let branch_ref = format!("refs/heads/{}", branch_name);
        let sig = Signature::now("Git Patcher", "git-patcher@internal.invalid")?;
        let msg = format!(
            "snapshot for {}",
            self.root_repo.head()?.peel_to_commit()?.id()
        );
        let parent = self
            .root_repo
            .find_reference(&branch_ref)
            .and_then(|r| r.peel_to_commit());
        self.root_repo.commit(
            Some(&branch_ref),
            &sig,
            &sig,
            &msg,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )?;

        Ok(())
    }

    fn sync_source_by_reconstruct(&mut self, branch_name: &str) -> anyhow::Result<()> {
        // fetch upstream commit to root
        let upstream_head = self.upstream_repo.head()?.peel_to_commit()?;
        let mut remote = self
            .root_repo
            .remote_anonymous(&self.upstream_path.to_string_lossy())?;
        println!(
            "Fetching upstream head {} to root repository",
            upstream_head.id()
        );
        remote.fetch(&[upstream_head.id().to_string()], None, None)?;

        let mut parent = self.root_repo.find_commit(upstream_head.id())?;
        let mut tree = parent.tree()?;

        let patch_series = self.get_patch_series()?;
        for patch in patch_series.peeker() {
            let patch_bytes = patch.1?;
            println!("Applying patch: {}", patch.0.display());
            let diff = Diff::from_buffer(&patch_bytes)?;
            tree = self.root_repo.find_tree(
                self.root_repo
                    .apply_to_tree(&tree, &diff, None)?
                    .write_tree_to(&self.root_repo)?,
            )?;

            let patch_metadata = utils::patch_utils::parse_patch_metadata(&patch_bytes)?;

            let new_commit_oid = self.root_repo.commit(
                None,
                &patch_metadata.author.as_signature()?,
                &patch_metadata.committer.as_signature()?,
                &patch_metadata.commit_message,
                &tree,
                &[&parent],
            )?;
            validate_hash(
                &parent.id(),
                &patch_metadata.parent_hash,
                &new_commit_oid,
                &patch_metadata.commit_hash,
            );

            parent = self.root_repo.find_commit(new_commit_oid)?;
        }

        self.root_repo.branch(branch_name, &parent, true)?;

        Ok(())
    }
}

fn validate_hash(
    parent: &git2::Oid,
    parent_expected: &git2::Oid,
    commit: &git2::Oid,
    commit_expected: &git2::Oid,
) {
    if parent != parent_expected {
        eprintln!(
            "Warning: Parent hash mismatch: expected {}, got {}. Unable to guarantee the integrity of the patch.",
            parent_expected, parent
        );
        return;
    }
    if commit != commit_expected {
        eprintln!(
            "Warning: Commit hash mismatch: expected {}, got {}. This may indicate that the patch was modified after being generated.",
            commit_expected, commit
        );
    }
}
