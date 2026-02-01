use crate::config::Config;
use crate::ignore::IgnoreRules;
use crate::resolver;
use crate::store::Store;
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupStatus {
    InSync,
    OutOfSync,
    New,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    InSync,
    Modified,
    New,
}

#[derive(Debug, Clone)]
pub struct FileStatusEntry {
    pub relative_path: String,
    pub state: FileState,
}

#[derive(Debug)]
pub struct GroupStatusResult {
    pub name: String,
    pub status: GroupStatus,
    pub files: Vec<FileStatusEntry>,
}

#[derive(Debug)]
pub struct PlanStatusResult {
    pub name: String,
    pub status: GroupStatus,
}

pub struct StatusChecker<'a> {
    config: &'a Config,
    store: &'a dyn Store,
    resolver: String,
}

impl<'a> StatusChecker<'a> {
    pub fn new(config: &'a Config, store: &'a dyn Store, resolver: String) -> Self {
        Self {
            config,
            store,
            resolver,
        }
    }

    pub fn check_group(&self, group_name: &str) -> Result<GroupStatusResult> {
        let resolved_path = match self.config.get_resolver(group_name, &self.resolver) {
            Ok(path) => path,
            Err(_) => {
                return Ok(GroupStatusResult {
                    name: group_name.to_string(),
                    status: GroupStatus::Skipped,
                    files: Vec::new(),
                });
            }
        };

        let resolved_path = resolver::resolve_path(resolved_path)?;
        let cwd = std::env::current_dir()?;
        let group_dir = cwd.join(group_name);

        if !group_dir.exists() {
            return Ok(GroupStatusResult {
                name: group_name.to_string(),
                status: GroupStatus::New,
                files: Vec::new(),
            });
        }

        let ignore_path = group_dir.join(".dootignore");
        let ignore_rules = IgnoreRules::load(&ignore_path)?;

        let mut files = Vec::new();
        let mut has_changes = false;
        let mut all_new = true;

        let walker = WalkBuilder::new(&group_dir)
            .standard_filters(false)
            .add_custom_ignore_filename(".dootignore")
            .build();

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let full_path = entry.path();
            let relative = full_path.strip_prefix(&group_dir)?;
            let relative_str = relative.to_string_lossy();

            if !ignore_rules.is_included(&relative_str) {
                continue;
            }

            let destination = resolved_path.join(relative);
            let state = self.compute_file_state(full_path, &destination);

            match state {
                FileState::New => has_changes = true,
                FileState::Modified => {
                    has_changes = true;
                    all_new = false;
                }
                FileState::InSync => all_new = false,
            }

            files.push(FileStatusEntry {
                relative_path: relative_str.to_string(),
                state,
            });
        }

        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let status = if files.is_empty() {
            GroupStatus::New
        } else if !has_changes {
            GroupStatus::InSync
        } else if all_new {
            GroupStatus::New
        } else {
            GroupStatus::OutOfSync
        };

        Ok(GroupStatusResult {
            name: group_name.to_string(),
            status,
            files,
        })
    }

    fn compute_file_state(&self, source: &Path, destination: &Path) -> FileState {
        if !self.store.exists(destination) {
            FileState::New
        } else if self.store.compare(source, destination).unwrap_or(false) {
            FileState::InSync
        } else {
            FileState::Modified
        }
    }

    pub fn check_all_groups(&self) -> Result<Vec<GroupStatusResult>> {
        let mut results = Vec::new();
        let mut group_names: Vec<_> = self.config.groups.keys().collect();
        group_names.sort();

        for group_name in group_names {
            results.push(self.check_group(group_name)?);
        }

        Ok(results)
    }

    pub fn check_plan(
        &self,
        plan_name: &str,
        group_results: &[GroupStatusResult],
    ) -> PlanStatusResult {
        let plan_groups = self.config.plans.get(plan_name);

        let groups_in_plan: Vec<String> = match plan_groups {
            Some(Some(groups)) => groups.clone(),
            Some(None) => self.config.groups.keys().cloned().collect(),
            None => Vec::new(),
        };

        let mut status = GroupStatus::InSync;
        let mut has_any_group = false;

        for group_name in &groups_in_plan {
            if let Some(group_result) = group_results.iter().find(|g| &g.name == group_name) {
                match group_result.status {
                    GroupStatus::Skipped => continue,
                    GroupStatus::OutOfSync => {
                        status = GroupStatus::OutOfSync;
                        has_any_group = true;
                    }
                    GroupStatus::New => {
                        if status != GroupStatus::OutOfSync {
                            status = GroupStatus::New;
                        }
                        has_any_group = true;
                    }
                    GroupStatus::InSync => {
                        has_any_group = true;
                    }
                }
            }
        }

        if !has_any_group {
            status = GroupStatus::Skipped;
        }

        PlanStatusResult {
            name: plan_name.to_string(),
            status,
        }
    }

    pub fn check_all_plans(&self, group_results: &[GroupStatusResult]) -> Vec<PlanStatusResult> {
        let mut results = Vec::new();
        let mut plan_names: Vec<_> = self.config.plans.keys().collect();
        plan_names.sort();

        for plan_name in plan_names {
            results.push(self.check_plan(plan_name, group_results));
        }

        results
    }
}
