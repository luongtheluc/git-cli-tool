use std::sync::mpsc;

use crate::git_runner;
use crate::repo_scanner::RepoInfo;
use super::app::BatchResult;

/// Batch operation types that can be executed on selected repos
pub enum BatchOp {
    Pull,
    Push,
    Fetch,
    Checkout(String),
    Commit(String),
    Git(Vec<String>),
    Audit,
}

impl BatchOp {
    /// Human-readable name for progress display
    pub fn display_name(&self) -> &str {
        match self {
            BatchOp::Pull => "Pulling",
            BatchOp::Push => "Pushing",
            BatchOp::Fetch => "Fetching",
            BatchOp::Checkout(_) => "Checking out",
            BatchOp::Commit(_) => "Committing",
            BatchOp::Git(_) => "Running git",
            BatchOp::Audit => "Auditing",
        }
    }
}

/// Execute batch operation asynchronously using threads + mpsc channel.
/// Each repo runs in its own thread, sending results as they complete.
pub fn execute_batch_async(
    repos: &[RepoInfo],
    indices: &[usize],
    op: &BatchOp,
) -> mpsc::Receiver<BatchResult> {
    let (tx, rx) = mpsc::channel();

    // Collect data needed by threads (clone to avoid lifetime issues)
    let tasks: Vec<(String, std::path::PathBuf)> = indices
        .iter()
        .filter_map(|&i| repos.get(i))
        .map(|r| (r.name.clone(), r.path.clone()))
        .collect();

    // Clone op data for threads
    let op_args: OpArgs = match op {
        BatchOp::Pull => OpArgs::Simple(SimpleOp::Pull),
        BatchOp::Push => OpArgs::Simple(SimpleOp::Push),
        BatchOp::Fetch => OpArgs::Simple(SimpleOp::Fetch),
        BatchOp::Checkout(b) => OpArgs::WithArg(ArgOp::Checkout, b.clone()),
        BatchOp::Commit(m) => OpArgs::WithArg(ArgOp::Commit, m.clone()),
        BatchOp::Git(args) => OpArgs::Git(args.clone()),
        BatchOp::Audit => OpArgs::Simple(SimpleOp::Audit),
    };

    for (name, path) in tasks {
        let tx = tx.clone();
        let op_args = op_args.clone();
        std::thread::spawn(move || {
            let result = match &op_args {
                OpArgs::Simple(SimpleOp::Pull) => git_runner::pull(&path),
                OpArgs::Simple(SimpleOp::Push) => git_runner::push(&path),
                OpArgs::Simple(SimpleOp::Fetch) => git_runner::fetch(&path),
                OpArgs::Simple(SimpleOp::Audit) => {
                    // Encode audit result as pipe-delimited string: branch|uncommitted|unpushed|behind|no_upstream
                    let a = git_runner::audit_repo(&path);
                    Ok(format!("{}|{}|{}|{}|{}", a.branch, a.uncommitted, a.unpushed, a.behind, a.no_upstream))
                }
                OpArgs::WithArg(ArgOp::Checkout, branch) => {
                    git_runner::checkout(&path, branch, false)
                }
                OpArgs::WithArg(ArgOp::Commit, msg) => git_runner::commit(&path, msg),
                OpArgs::Git(args) => {
                    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    git_runner::run_git(&path, &str_args)
                }
            };
            let _ = tx.send(BatchResult {
                repo_name: name,
                success: result.is_ok(),
                message: match result {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                },
            });
        });
    }

    rx
}

/// Thread-safe operation type (cloneable for thread spawning)
#[derive(Clone)]
enum OpArgs {
    Simple(SimpleOp),
    WithArg(ArgOp, String),
    Git(Vec<String>),
}

#[derive(Clone)]
enum SimpleOp {
    Pull,
    Push,
    Fetch,
    Audit,
}

#[derive(Clone)]
enum ArgOp {
    Checkout,
    Commit,
}
