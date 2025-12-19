use crate::config::Mode;
use crate::plan::{FileEntry, FileStatus, Plan};
use crate::store::{LinkStore, Store};
use anyhow::Result;
use colored::Colorize;
use similar::{ChangeTag, TextDiff};
use std::io::{self, Write};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

fn apply_diff_tint(highlighted: &str, tint: &str) -> String {
    highlighted.replace("\x1b[0m", &format!("\x1b[0m{}", tint)) + "\x1b[0m"
}

pub struct Executor<'a> {
    store: &'a dyn Store,
    mode: Mode,
}

impl<'a> Executor<'a> {
    pub fn new(store: &'a dyn Store, mode: Mode) -> Self {
        Self { store, mode }
    }

    pub fn display_plan(&self, plan: &Plan, operation: &str) {
        if plan.is_empty() {
            println!("No files to {}.", operation);
            return;
        }

        println!("\n{}:\n", operation);

        for group in &plan.groups {
            println!("  {}:", group.group_name.bold());

            if group.entries.is_empty() {
                println!("    {}", "(no files)".dimmed());
            } else {
                for entry in &group.entries {
                    let (icon, label) = match entry.status {
                        FileStatus::Same => ("✓".blue(), "same".blue()),
                        FileStatus::Create => ("+".green(), "create".green()),
                        FileStatus::Overwrite => ("~".yellow(), "overwrite".yellow()),
                    };

                    println!(
                        "    [{}] {} ({})",
                        icon,
                        entry.relative_path.display(),
                        label
                    );
                }
            }
            println!();
        }

        let same = plan.total_count_by_status(FileStatus::Same);
        let create = plan.total_count_by_status(FileStatus::Create);
        let overwrite = plan.total_count_by_status(FileStatus::Overwrite);

        println!(
            "Summary: {} same, {} to create, {} to overwrite",
            same, create, overwrite
        );
    }

    pub fn confirm(&self, plan: &Plan) -> Result<bool> {
        loop {
            print!("\nProceed? [y/N/d] ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim().to_ascii_lowercase().as_str() {
                "y" => return Ok(true),
                "n" | "" => return Ok(false),
                "d" => self.show_diffs(plan)?,
                _ => println!(
                    "Invalid option. Use 'y' to proceed, 'n' to abort, or 'd' to show diffs."
                ),
            }
        }
    }

    fn show_diffs(&self, plan: &Plan) -> Result<()> {
        println!();
        for group in &plan.groups {
            for entry in &group.entries {
                if entry.status == FileStatus::Same {
                    continue;
                }
                self.show_entry_diff(entry, &group.group_name)?;
            }
        }
        Ok(())
    }

    fn show_entry_diff(&self, entry: &FileEntry, group_name: &str) -> Result<()> {
        let old_content = if self.store.exists(&entry.destination) {
            String::from_utf8_lossy(&self.store.read(&entry.destination)?).into_owned()
        } else {
            String::new()
        };

        let new_content = String::from_utf8_lossy(&self.store.read(&entry.source)?).into_owned();

        println!(
            "{}",
            format!(
                "--- {}/{} (destination)",
                group_name,
                entry.relative_path.display()
            )
            .red()
        );
        println!(
            "{}",
            format!(
                "+++ {}/{} (source)",
                group_name,
                entry.relative_path.display()
            )
            .green()
        );
        println!("{}", "─".repeat(60).dimmed());

        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = &ts.themes["base16-ocean.dark"];

        let syntax = ps
            .find_syntax_for_file(&entry.relative_path)
            .ok()
            .flatten()
            .unwrap_or_else(|| ps.find_syntax_plain_text());

        let diff = TextDiff::from_lines(&old_content, &new_content);
        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                println!("{}", "───".dimmed());
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let tag = change.tag();
                    let line = change.value();

                    let line_num = match tag {
                        ChangeTag::Delete => change
                            .old_index()
                            .map(|n| format!("{:4}", n + 1))
                            .unwrap_or_else(|| "    ".to_string()),
                        ChangeTag::Insert | ChangeTag::Equal => change
                            .new_index()
                            .map(|n| format!("{:4}", n + 1))
                            .unwrap_or_else(|| "    ".to_string()),
                    };

                    let sign = match tag {
                        ChangeTag::Delete => "-".red(),
                        ChangeTag::Insert => "+".green(),
                        ChangeTag::Equal => " ".dimmed(),
                    };

                    print!("\x1b[48;2;40;40;50m{}\x1b[0m {} ", line_num.dimmed(), sign);

                    let highlighted = self.highlight_line(&ps, syntax, theme, line);

                    let styled = match tag {
                        ChangeTag::Delete => apply_diff_tint(&highlighted, "\x1b[31m"),
                        ChangeTag::Insert => apply_diff_tint(&highlighted, "\x1b[32m"),
                        ChangeTag::Equal => highlighted,
                    };
                    print!("{}", styled);
                    if !line.ends_with('\n') {
                        println!();
                    }
                }
            }
        }
        println!();
        Ok(())
    }

    fn highlight_line(
        &self,
        ps: &SyntaxSet,
        syntax: &syntect::parsing::SyntaxReference,
        theme: &syntect::highlighting::Theme,
        line: &str,
    ) -> String {
        let mut h = HighlightLines::new(syntax, theme);
        match h.highlight_line(line, ps) {
            Ok(ranges) => as_24_bit_terminal_escaped(&ranges, false),
            Err(_) => line.to_string(),
        }
    }

    pub fn execute(&self, plan: &Plan) -> Result<()> {
        for group in &plan.groups {
            if !group.has_changes() {
                continue;
            }

            println!("  {}:", group.group_name);
            for entry in &group.entries {
                if entry.status == FileStatus::Same {
                    continue;
                }
                self.execute_entry(entry)?;
            }
        }

        Ok(())
    }

    fn execute_entry(&self, entry: &FileEntry) -> Result<()> {
        match self.mode {
            Mode::File => {
                let content = self.store.read(&entry.source)?;
                self.store.write(&entry.destination, &content)?;
            }
            Mode::Link => {
                LinkStore::create_symlink(&entry.source, &entry.destination)?;
            }
        }

        let action = match entry.status {
            FileStatus::Create => "Created",
            FileStatus::Overwrite => "Updated",
            FileStatus::Same => "Skipped",
        };

        println!("    {} {}", action, entry.relative_path.display());
        Ok(())
    }

    pub fn run(&self, plan: &Plan, operation: &str, skip_confirm: bool) -> Result<()> {
        self.display_plan(plan, operation);

        if !plan.has_changes() {
            println!("\nNothing to do.");
            return Ok(());
        }

        let proceed = if skip_confirm {
            true
        } else {
            self.confirm(plan)?
        };

        if proceed {
            println!("\nExecuting...\n");
            self.execute(plan)?;
            println!("\nDone!");
        } else {
            println!("\nAborted.");
        }

        Ok(())
    }
}
