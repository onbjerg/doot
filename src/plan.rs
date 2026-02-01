use crate::ignore::IgnoreRules;
use crate::store::Store;
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Same,
    Create,
    Overwrite,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub status: FileStatus,
}

#[derive(Debug)]
pub struct GroupPlan {
    pub group_name: String,
    pub entries: Vec<FileEntry>,
}

impl GroupPlan {
    pub fn has_changes(&self) -> bool {
        self.entries.iter().any(|e| e.status != FileStatus::Same)
    }

    pub fn count_by_status(&self, status: FileStatus) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }
}

#[derive(Debug)]
pub struct Plan {
    pub groups: Vec<GroupPlan>,
}

impl Plan {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_group(&mut self, group_name: String, entries: Vec<FileEntry>) {
        self.groups.push(GroupPlan {
            group_name,
            entries,
        });
    }

    pub fn has_changes(&self) -> bool {
        self.groups.iter().any(|g| g.has_changes())
    }

    pub fn total_count_by_status(&self, status: FileStatus) -> usize {
        self.groups
            .iter()
            .map(|g| g.count_by_status(status.clone()))
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.iter().all(|g| g.entries.is_empty())
    }
}

pub struct PlanBuilder<'a> {
    store: &'a dyn Store,
    ignore_rules: &'a IgnoreRules,
}

impl<'a> PlanBuilder<'a> {
    pub fn new(store: &'a dyn Store, ignore_rules: &'a IgnoreRules) -> Self {
        Self {
            store,
            ignore_rules,
        }
    }

    pub fn build_import(&self, group_dir: &Path, resolved_path: &Path) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(resolved_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let full_path = entry.path();
            let relative = full_path.strip_prefix(resolved_path)?;
            let relative_str = relative.to_string_lossy();

            if !self.ignore_rules.is_included(&relative_str) {
                continue;
            }

            let destination = group_dir.join(relative);
            let status = self.compute_status(full_path, &destination);

            entries.push(FileEntry {
                relative_path: relative.to_path_buf(),
                source: full_path.to_path_buf(),
                destination,
                status,
            });
        }

        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    pub fn build_export(&self, group_dir: &Path, resolved_path: &Path) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(group_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let full_path = entry.path();
            let relative = full_path.strip_prefix(group_dir)?;
            let relative_str = relative.to_string_lossy();

            if relative_str == ".dootignore" {
                continue;
            }

            if !self.ignore_rules.is_included(&relative_str) {
                continue;
            }

            let destination = resolved_path.join(relative);
            let status = self.compute_status(full_path, &destination);

            entries.push(FileEntry {
                relative_path: relative.to_path_buf(),
                source: full_path.to_path_buf(),
                destination,
                status,
            });
        }

        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    fn compute_status(&self, source: &Path, destination: &Path) -> FileStatus {
        if !self.store.exists(destination) {
            FileStatus::Create
        } else if self.store.compare(source, destination).unwrap_or(false) {
            FileStatus::Same
        } else {
            FileStatus::Overwrite
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockStore {
        files: HashMap<PathBuf, Vec<u8>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }

        fn with_file(mut self, path: &str, content: &[u8]) -> Self {
            self.files.insert(PathBuf::from(path), content.to_vec());
            self
        }
    }

    impl Store for MockStore {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn read(&self, path: &Path) -> Result<Vec<u8>> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("File not found"))
        }

        fn write(&self, _path: &Path, _content: &[u8]) -> Result<()> {
            Ok(())
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.contains_key(path)
        }

        fn remove(&self, _path: &Path) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn plan_tracks_changes_across_groups() {
        let mut plan = Plan::new();

        plan.add_group(
            "group1".to_string(),
            vec![FileEntry {
                relative_path: PathBuf::from("file1"),
                source: PathBuf::from("/src/file1"),
                destination: PathBuf::from("/dst/file1"),
                status: FileStatus::Same,
            }],
        );

        plan.add_group(
            "group2".to_string(),
            vec![FileEntry {
                relative_path: PathBuf::from("file2"),
                source: PathBuf::from("/src/file2"),
                destination: PathBuf::from("/dst/file2"),
                status: FileStatus::Create,
            }],
        );

        assert!(plan.has_changes());
        assert_eq!(plan.total_count_by_status(FileStatus::Same), 1);
        assert_eq!(plan.total_count_by_status(FileStatus::Create), 1);
    }

    #[test]
    fn plan_with_no_changes() {
        let mut plan = Plan::new();
        plan.add_group(
            "group".to_string(),
            vec![FileEntry {
                relative_path: PathBuf::from("file"),
                source: PathBuf::from("/src/file"),
                destination: PathBuf::from("/dst/file"),
                status: FileStatus::Same,
            }],
        );

        assert!(!plan.has_changes());
    }

    #[test]
    fn status_create_when_destination_missing() {
        let store = MockStore::new().with_file("/src/file", b"content");
        let ignore = IgnoreRules::parse("").unwrap();
        let builder = PlanBuilder::new(&store, &ignore);

        let status = builder.compute_status(Path::new("/src/file"), Path::new("/dst/file"));
        assert_eq!(status, FileStatus::Create);
    }

    #[test]
    fn status_same_when_content_matches() {
        let store = MockStore::new()
            .with_file("/src/file", b"content")
            .with_file("/dst/file", b"content");
        let ignore = IgnoreRules::parse("").unwrap();
        let builder = PlanBuilder::new(&store, &ignore);

        let status = builder.compute_status(Path::new("/src/file"), Path::new("/dst/file"));
        assert_eq!(status, FileStatus::Same);
    }

    #[test]
    fn status_overwrite_when_content_differs() {
        let store = MockStore::new()
            .with_file("/src/file", b"new content")
            .with_file("/dst/file", b"old content");
        let ignore = IgnoreRules::parse("").unwrap();
        let builder = PlanBuilder::new(&store, &ignore);

        let status = builder.compute_status(Path::new("/src/file"), Path::new("/dst/file"));
        assert_eq!(status, FileStatus::Overwrite);
    }
}
