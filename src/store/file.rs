use super::Store;
use anyhow::{Context, Result};
use std::path::Path;

pub struct FileStore;

impl Store for FileStore {
    fn name(&self) -> &'static str {
        "file"
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
        path.exists()
    }

    fn remove(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to remove: {}", path.display()))?;
        }
        Ok(())
    }
}
