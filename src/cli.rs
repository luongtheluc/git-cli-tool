use clap::{Parser, Subcommand};

/// Polyrepo manager — manage multiple git repositories from a workspace folder
#[derive(Parser)]
#[command(name = "repo", about, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all repositories with branch and last commit info
    List,

    /// Checkout a branch in selected repositories
    Checkout {
        /// Branch name to checkout (or create with -b)
        branch: String,

        /// Create the branch if it doesn't exist (git checkout -b)
        #[arg(short = 'b', long)]
        create: bool,
    },

    /// Pull latest changes in selected repositories
    Pull,

    /// Push commits to remote in selected repositories
    Push,

    /// Stage all changes and commit in selected repositories
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },

    /// Show git status in selected repositories
    Status,

    /// Run any git command in selected repositories
    Git {
        /// Arguments to pass to git (use -- before args starting with -)
        #[arg(trailing_var_arg = true, required = true)]
        args: Vec<String>,
    },

    /// Audit git health across all repositories (offline check, exit 1 if issues)
    Audit,

    /// Audit npm/yarn dependency vulnerabilities across all repositories
    AuditDeps,

    /// Launch the interactive TUI (lazygit-like interface)
    Ui,

    /// Run a named script in selected repositories (auto-detects build tool)
    Run {
        /// Script name to run: e.g. "build", "test", or a shell command
        script: String,

        /// Max repos running concurrently (0 = auto = cpu_count/2, 1 = sequential)
        #[arg(short, long, default_value_t = 1)]
        jobs: usize,
    },
}
