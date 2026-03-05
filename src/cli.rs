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
        /// Branch name to checkout
        branch: String,
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
}
