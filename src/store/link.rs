use super::Store;
use anyhow::{Context, Result};
use std::path::Path;

pub struct LinkStore;

impl Store for LinkStore {
    fn name(&self) -> &'static str {
        "link"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).with_context(|| format!("Failed to read: {}", path.display()))
    }

    fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write: {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists() || path.is_symlink()
    }

    fn remove(&self, path: &Path) -> Result<()> {
        if path.exists() || path.is_symlink() {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove: {}", path.display()))?;
        }
        Ok(())
    }

    fn hash(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let content = self.read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hex::encode(hasher.finalize()))
    }
}

impl LinkStore {
    pub fn create_symlink(source: &Path, target: &Path) -> Result<()> {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        if target.exists() || target.is_symlink() {
            std::fs::remove_file(target)
                .with_context(|| format!("Failed to remove existing: {}", target.display()))?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(source, target).with_context(|| {
            format!(
                "Failed to create symlink: {} -> {}",
                target.display(),
                source.display()
            )
        })?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(source, target).with_context(|| {
            format!(
                "Failed to create symlink: {} -> {}",
                target.display(),
                source.display()
            )
        })?;

        Ok(())
    }
}
