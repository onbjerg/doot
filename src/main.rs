mod cli;
mod config;
mod executor;
mod ignore;
mod plan;
mod resolver;
mod store;

use anyhow::{Context, Result};
use std::path::PathBuf;

use cli::{Command, Target};
use config::Config;
use executor::Executor;
use ignore::IgnoreRules;
use plan::{Plan, PlanBuilder};
use store::create_store;

fn main() -> Result<()> {
    env_logger::init();

    let args = cli::parse();
    let config = Config::load(&args.config)?;
    let store = create_store(config.mode);

    match args.command {
        Command::Import { target } => run_import(&config, &*store, &target, args.yes),
        Command::Export { target } => run_export(&config, &*store, &target, args.yes),
        Command::List => run_list(&config),
    }
}

fn run_import(
    config: &Config,
    store: &dyn store::Store,
    target: &Target,
    skip_confirm: bool,
) -> Result<()> {
    let groups = resolve_groups(config, target)?;
    let resolver_name = get_resolver_name(target);
    let operation = get_operation_name("Import", target);

    let mut plan = Plan::new();

    for group_name in groups {
        let resolved_path = config.get_resolver(&group_name, &resolver_name)?;
        let resolved_path = resolver::resolve_path(resolved_path)?;
        let group_dir = get_group_dir(&group_name)?;

        let ignore_path = group_dir.join(".dootignore");
        let ignore_rules = IgnoreRules::load(&ignore_path)?;

        let plan_builder = PlanBuilder::new(store, &ignore_rules);
        let entries = plan_builder.build_import(&group_dir, &resolved_path)?;
        plan.add_group(group_name, entries);
    }

    let executor = Executor::new(store, config.mode);
    executor.run(&plan, &operation, skip_confirm)?;

    Ok(())
}

fn run_export(
    config: &Config,
    store: &dyn store::Store,
    target: &Target,
    skip_confirm: bool,
) -> Result<()> {
    let groups = resolve_groups(config, target)?;
    let resolver_name = get_resolver_name(target);
    let operation = get_operation_name("Export", target);

    let mut plan = Plan::new();

    for group_name in groups {
        let resolved_path = config.get_resolver(&group_name, &resolver_name)?;
        let resolved_path = resolver::resolve_path(resolved_path)?;
        let group_dir = get_group_dir(&group_name)?;

        let ignore_path = group_dir.join(".dootignore");
        let ignore_rules = IgnoreRules::load(&ignore_path)?;

        let plan_builder = PlanBuilder::new(store, &ignore_rules);
        let entries = plan_builder.build_export(&group_dir, &resolved_path)?;
        plan.add_group(group_name, entries);
    }

    let executor = Executor::new(store, config.mode);
    executor.run(&plan, &operation, skip_confirm)?;

    Ok(())
}

fn resolve_groups(config: &Config, target: &Target) -> Result<Vec<String>> {
    match target {
        Target::Group { name, .. } => {
            config.get_group(name)?;
            Ok(vec![name.clone()])
        }
        Target::Plan { name, .. } => config.get_plan_groups(name),
    }
}

fn get_resolver_name(target: &Target) -> String {
    match target {
        Target::Group { resolver, .. } | Target::Plan { resolver, .. } => resolver.clone(),
    }
}

fn get_operation_name(action: &str, target: &Target) -> String {
    match target {
        Target::Group { name, .. } => format!("{} group '{}'", action, name),
        Target::Plan { name, .. } => format!("{} plan '{}'", action, name),
    }
}

fn get_group_dir(group_name: &str) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    Ok(cwd.join(group_name))
}

fn run_list(config: &Config) -> Result<()> {
    let mut plans: Vec<_> = config.plans.keys().collect();
    plans.sort();

    let mut groups: Vec<_> = config.groups.keys().collect();
    groups.sort();

    println!("Plans");
    for (i, plan) in plans.iter().enumerate() {
        let is_last = i == plans.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let plan_groups = config.plans.get(*plan).unwrap();
        match plan_groups {
            None => println!("{prefix}{plan} (all groups)"),
            Some(group_list) => {
                println!("{prefix}{plan}");
                println!("{child_prefix}└── {}", group_list.join(", "));
            }
        }
    }

    println!();
    println!("Groups");
    for (i, group) in groups.iter().enumerate() {
        let is_last = i == groups.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        println!("{prefix}{group}");

        let resolvers = config.groups.get(*group).unwrap();
        let mut resolver_names: Vec<_> = resolvers.keys().collect();
        resolver_names.sort();

        for (j, resolver) in resolver_names.iter().enumerate() {
            let is_last_resolver = j == resolver_names.len() - 1;
            let resolver_prefix = if is_last_resolver {
                "└── "
            } else {
                "├── "
            };
            let path = resolvers.get(*resolver).unwrap();
            println!("{child_prefix}{resolver_prefix}{resolver} → {path}");
        }
    }

    Ok(())
}
