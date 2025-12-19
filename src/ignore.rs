use anyhow::Result;
use std::path::Path;

pub struct IgnoreRules {
    patterns: Vec<IgnorePattern>,
}

struct IgnorePattern {
    pattern: glob::Pattern,
    negated: bool,
}

impl IgnoreRules {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self { patterns: vec![] });
        }

        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut patterns = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let line = if let Some(idx) = line.find(" #") {
                line[..idx].trim()
            } else {
                line
            };

            let (pattern_str, negated) = if let Some(stripped) = line.strip_prefix('!') {
                (stripped, true)
            } else {
                (line, false)
            };

            let pattern = glob::Pattern::new(pattern_str)
                .map_err(|e| anyhow::anyhow!("Invalid pattern '{}': {}", pattern_str, e))?;

            patterns.push(IgnorePattern { pattern, negated });
        }

        Ok(Self { patterns })
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        let mut ignored = false;

        for pattern in &self.patterns {
            if pattern.pattern.matches(path) {
                ignored = !pattern.negated;
            }
        }

        ignored
    }

    pub fn is_included(&self, path: &str) -> bool {
        !self.is_ignored(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_all_except_specific_files() {
        let rules = IgnoreRules::parse("*\n!.bashrc\n!.profile").unwrap();

        assert!(rules.is_ignored("random.txt"));
        assert!(rules.is_ignored(".bash_history"));
        assert!(!rules.is_ignored(".bashrc"));
        assert!(!rules.is_ignored(".profile"));
    }

    #[test]
    fn empty_rules_include_everything() {
        let rules = IgnoreRules::parse("").unwrap();
        assert!(rules.is_included("anything"));
        assert!(rules.is_included(".hidden"));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let rules = IgnoreRules::parse(
            r#"
# This is a comment
*.log

# Another comment
*.tmp
"#,
        )
        .unwrap();

        assert!(rules.is_ignored("debug.log"));
        assert!(rules.is_ignored("cache.tmp"));
        assert!(!rules.is_ignored("config.yaml"));
    }

    #[test]
    fn inline_comments_are_stripped() {
        let rules = IgnoreRules::parse("*.bak # backup files").unwrap();
        assert!(rules.is_ignored("file.bak"));
        assert!(!rules.is_ignored("file.bak # backup files"));
    }

    #[test]
    fn later_rules_override_earlier() {
        let rules = IgnoreRules::parse("*\n!*.txt\n*.txt").unwrap();
        assert!(rules.is_ignored("file.txt"));

        let rules = IgnoreRules::parse("*.txt\n!*.txt").unwrap();
        assert!(!rules.is_ignored("file.txt"));
    }

    #[test]
    fn glob_patterns_work() {
        let rules = IgnoreRules::parse("*.log\ntemp_*\n?.tmp").unwrap();

        assert!(rules.is_ignored("error.log"));
        assert!(rules.is_ignored("temp_file"));
        assert!(rules.is_ignored("a.tmp"));
        assert!(!rules.is_ignored("ab.tmp"));
        assert!(!rules.is_ignored("keep.txt"));
    }

    #[test]
    fn negation_without_prior_ignore_does_nothing() {
        let rules = IgnoreRules::parse("!.bashrc").unwrap();
        assert!(!rules.is_ignored(".bashrc"));
        assert!(!rules.is_ignored("other"));
    }
}
