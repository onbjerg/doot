use anyhow::Result;
use std::path::PathBuf;

pub fn resolve_path(path: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(path)
        .map_err(|e| anyhow::anyhow!("Failed to expand path '{}': {}", path, e))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_tilde() {
        let home = dirs::home_dir().unwrap();
        let resolved = resolve_path("~").unwrap();
        assert_eq!(resolved, home);
    }

    #[test]
    fn test_resolve_tilde_path() {
        let home = dirs::home_dir().unwrap();
        let resolved = resolve_path("~/.bashrc").unwrap();
        assert_eq!(resolved, home.join(".bashrc"));
    }
}
