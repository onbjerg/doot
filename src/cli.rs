use bpaf::Bpaf;
use std::path::PathBuf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
pub struct Args {
    /// Skip confirmation prompt
    #[bpaf(short, long)]
    pub yes: bool,

    /// Path to config file
    #[bpaf(short, long, fallback(PathBuf::from("doot.yaml")))]
    pub config: PathBuf,

    #[bpaf(external)]
    pub command: Command,
}

#[derive(Debug, Clone, Bpaf)]
pub enum Command {
    /// Import files from system to dotfiles repo
    #[bpaf(command)]
    Import {
        #[bpaf(external)]
        target: Target,
    },

    /// Export files from dotfiles repo to system
    #[bpaf(command)]
    Export {
        #[bpaf(external)]
        target: Target,
    },

    /// List all plans, groups, and resolvers
    #[bpaf(command)]
    List,
}

#[derive(Debug, Clone, Bpaf)]
pub enum Target {
    /// Operate on a single group
    #[bpaf(command)]
    Group {
        /// Name of the group
        #[bpaf(positional("GROUP"))]
        name: String,

        /// Name of the resolver
        #[bpaf(positional("RESOLVER"))]
        resolver: String,
    },

    /// Operate on a plan (multiple groups)
    #[bpaf(command)]
    Plan {
        /// Name of the plan
        #[bpaf(positional("PLAN"))]
        name: String,

        /// Name of the resolver
        #[bpaf(positional("RESOLVER"))]
        resolver: String,
    },
}

pub fn parse() -> Args {
    args().run()
}
