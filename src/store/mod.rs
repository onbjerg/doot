mod file;
mod link;

pub use file::FileStore;
pub use link::LinkStore;

use anyhow::Result;
use std::path::Path;

pub trait Store: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    fn read(&self, path: &Path) -> Result<Vec<u8>>;

    fn write(&self, path: &Path, content: &[u8]) -> Result<()>;

    fn exists(&self, path: &Path) -> bool;

    #[allow(dead_code)]
    fn remove(&self, path: &Path) -> Result<()>;

    fn hash(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let content = self.read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hex::encode(hasher.finalize()))
    }

    fn compare(&self, a: &Path, b: &Path) -> Result<bool> {
        if !self.exists(a) || !self.exists(b) {
            return Ok(false);
        }
        let hash_a = self.hash(a)?;
        let hash_b = self.hash(b)?;
        Ok(hash_a == hash_b)
    }
}

pub fn create_store(mode: crate::config::Mode) -> Box<dyn Store> {
    match mode {
        crate::config::Mode::File => Box::new(FileStore),
        crate::config::Mode::Link => Box::new(LinkStore),
    }
}
