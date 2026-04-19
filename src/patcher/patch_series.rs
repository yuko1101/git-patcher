use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use ouroboros::self_referencing;

pub struct PatchSeries {
    patches_dir: PathBuf,
    patches: VecDeque<PathBuf>,
    lock: PatchSeriesLock,
}

impl PatchSeries {
    pub fn new(patches_dir: &Path, series_path: &Path) -> anyhow::Result<Self> {
        let mut patches = VecDeque::new();

        let series_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(series_path)?;
        let mut series_lock = PatchSeriesLockTryBuilder {
            lock: fd_lock::RwLock::new(series_file),
            write_guard_builder: |lock| lock.write(),
        }
        .try_build()?;

        let series_content =
            series_lock.with_write_guard_mut(|guard| std::io::read_to_string(&**guard))?;
        for line in series_content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let patch_file = patches_dir.join(line.trim());
            if !patch_file.starts_with(patches_dir) {
                anyhow::bail!(
                    "Patch file {} listed in series is outside of patches directory",
                    patch_file.display()
                );
            }
            if patch_file.exists() {
                patches.push_back(patch_file);
            } else {
                anyhow::bail!(
                    "Patch file {} listed in series not found",
                    patch_file.display()
                );
            }
        }
        Ok(Self {
            patches_dir: patches_dir.to_path_buf(),
            patches,
            lock: series_lock,
        })
    }

    /// Requires path to exists in file system
    pub fn push_patch(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if !path.exists() {
            bail!(
                "Patch file {} which does not exist cannot be pushed to series",
                path.display()
            );
        }
        self.patches.push_back(path);
        Ok(())
    }

    /// Automatically removes the patch file from file system if it exists
    pub fn consume_patch(&mut self) -> Option<(PathBuf, anyhow::Result<Vec<u8>>)> {
        let Some(path) = self.patches.pop_front() else {
            return None;
        };
        let content = read_patch(&path).and_then(|content| {
            std::fs::remove_file(&path).with_context(|| {
                format!(
                    "Failed to remove patch file {} after reading",
                    path.display()
                )
            })?;
            Ok(content)
        });

        Some((path, content))
    }

    pub fn len(&self) -> usize {
        self.patches.len()
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut series_content = String::new();
        for patch in &self.patches {
            let relative_path = patch.strip_prefix(&self.patches_dir)?;
            series_content.push_str(relative_path.to_str().unwrap());
            series_content.push('\n');
        }

        self.lock.with_write_guard_mut(|guard| {
            guard.set_len(0)?;
            guard.seek(std::io::SeekFrom::Start(0))?;
            guard.write_all(series_content.as_bytes())?;
            Ok(())
        })
    }

    pub fn consumer(&mut self) -> PatchSeriesConsumer<'_> {
        PatchSeriesConsumer { patch_series: self }
    }

    pub fn peeker(&self) -> impl Iterator<Item = (PathBuf, anyhow::Result<Vec<u8>>)> {
        self.patches
            .iter()
            .map(|path| (path.clone(), read_patch(path)))
    }
}

#[self_referencing]
struct PatchSeriesLock {
    lock: fd_lock::RwLock<File>,
    #[borrows(mut lock)]
    #[not_covariant]
    write_guard: fd_lock::RwLockWriteGuard<'this, File>,
}

pub struct PatchSeriesConsumer<'a> {
    patch_series: &'a mut PatchSeries,
}

impl<'a> Iterator for PatchSeriesConsumer<'a> {
    type Item = (PathBuf, anyhow::Result<Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        self.patch_series.consume_patch()
    }
}

fn read_patch(path: &Path) -> anyhow::Result<Vec<u8>> {
    if !path.exists() {
        bail!("Patch file {} not found", path.display());
    }
    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read patch file {}", path.display()))?;
    Ok(content)
}
