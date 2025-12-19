use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    File,
    Link,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub version: String,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub plans: HashMap<String, Option<Vec<String>>>,
    #[serde(default)]
    pub groups: HashMap<String, HashMap<String, String>>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config =
            serde_yaml::from_str(&content).with_context(|| "Failed to parse doot.yaml")?;

        if config.version != "v1" {
            anyhow::bail!("Unsupported config version: {}", config.version);
        }

        Ok(config)
    }

    pub fn get_group(&self, name: &str) -> Result<&HashMap<String, String>> {
        self.groups
            .get(name)
            .with_context(|| format!("Group '{}' not found", name))
    }

    pub fn get_resolver(&self, group: &str, resolver: &str) -> Result<&str> {
        let group_resolvers = self.get_group(group)?;
        group_resolvers
            .get(resolver)
            .map(|s| s.as_str())
            .with_context(|| format!("Resolver '{}' not found in group '{}'", resolver, group))
    }

    pub fn get_plan_groups(&self, plan: &str) -> Result<Vec<String>> {
        let plan_groups = self
            .plans
            .get(plan)
            .with_context(|| format!("Plan '{}' not found", plan))?;

        match plan_groups {
            None => Ok(self.groups.keys().cloned().collect()),
            Some(groups) => Ok(groups.clone()),
        }
    }

    #[cfg(test)]
    pub fn parse(content: &str) -> Result<Self> {
        let config: Config =
            serde_yaml::from_str(content).with_context(|| "Failed to parse config")?;

        if config.version != "v1" {
            anyhow::bail!("Unsupported config version: {}", config.version);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let config = Config::parse("version: v1").unwrap();
        assert_eq!(config.version, "v1");
        assert_eq!(config.mode, Mode::File);
        assert!(config.groups.is_empty());
        assert!(config.plans.is_empty());
    }

    #[test]
    fn parse_rejects_unknown_version() {
        let err = Config::parse("version: v99").unwrap_err();
        assert!(err.to_string().contains("Unsupported config version"));
    }

    #[test]
    fn parse_mode_link() {
        let config = Config::parse("version: v1\nmode: link").unwrap();
        assert_eq!(config.mode, Mode::Link);
    }

    #[test]
    fn get_resolver_from_group() {
        let config = Config::parse(
            r#"
version: v1
groups:
  bash:
    nux: "~"
    mac: "$HOME"
"#,
        )
        .unwrap();

        assert_eq!(config.get_resolver("bash", "nux").unwrap(), "~");
        assert_eq!(config.get_resolver("bash", "mac").unwrap(), "$HOME");
    }

    #[test]
    fn get_resolver_missing_group() {
        let config = Config::parse("version: v1").unwrap();
        let err = config.get_resolver("nonexistent", "nux").unwrap_err();
        assert!(err.to_string().contains("Group 'nonexistent' not found"));
    }

    #[test]
    fn get_resolver_missing_resolver() {
        let config = Config::parse(
            r#"
version: v1
groups:
  bash:
    nux: "~"
"#,
        )
        .unwrap();

        let err = config.get_resolver("bash", "windows").unwrap_err();
        assert!(err.to_string().contains("Resolver 'windows' not found"));
    }

    #[test]
    fn empty_plan_returns_all_groups() {
        let config = Config::parse(
            r#"
version: v1
plans:
  all:
groups:
  bash:
    nux: "~"
  vim:
    nux: "~"
"#,
        )
        .unwrap();

        let mut groups = config.get_plan_groups("all").unwrap();
        groups.sort();
        assert_eq!(groups, vec!["bash", "vim"]);
    }

    #[test]
    fn explicit_plan_returns_listed_groups() {
        let config = Config::parse(
            r#"
version: v1
plans:
  minimal: [bash]
groups:
  bash:
    nux: "~"
  vim:
    nux: "~"
"#,
        )
        .unwrap();

        let groups = config.get_plan_groups("minimal").unwrap();
        assert_eq!(groups, vec!["bash"]);
    }
}
