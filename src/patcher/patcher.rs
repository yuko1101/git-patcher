use std::path::PathBuf;

use anyhow::bail;
use git2::{ApplyLocation, Diff, Repository, Signature, build::CheckoutBuilder};

use crate::{
    patcher::{internal_state::InternalState, patch_series::PatchSeries},
    utils,
};

pub struct Patcher {
    pub root: PathBuf,
    pub upstream_path: PathBuf,
    pub upstream_repo: Repository,
    pub state: InternalState,
    pub patches: PathBuf,
    pub series: PathBuf,
}

impl Patcher {
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        let upstream_path = root.join("upstream");
        let upstream_repo = Repository::open(&upstream_path)?;

        let state = InternalState::load(root.join(".git-patcher"))?;

        Ok(Self {
            root: root.clone(),
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

            self.upstream_repo.commit(
                Some("HEAD"),
                &patch_metadata.author.as_signature()?,
                &patch_metadata.committer.as_signature()?,
                &patch_metadata.commit_message,
                &tree,
                &[&parent],
            )?;
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
        for oid in diff_commits {
            println!("Generating patch for commit: {}", oid);
            let commit = self.upstream_repo.find_commit(oid)?;

            let patch_path = self.patches.join(format!("{}.patch", oid)); // TODO: use commit message or something more descriptive
            utils::patch_utils::write_patch_to_file(
                &parent,
                &commit,
                &self.upstream_repo,
                &patch_path,
            )?;
            patch_series.push_patch(patch_path)?;

            parent = commit;
            patches_pushed = true;
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
}
