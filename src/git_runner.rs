use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Execute a git command in the given repo path, returning trimmed stdout.
/// Uses `git -C <path>` so the caller never needs to change directory.
pub fn run_git(repo_path: &Path, args: &[&str]) -> Result<String> {
    let path_str = repo_path.to_str().unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(path_str)
        .args(args)
        .output()
        .context("Failed to run git — is git installed and in PATH?")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Some git commands write useful info to stdout even on failure (e.g. already on branch)
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        Err(anyhow::anyhow!("{}", msg))
    }
}

/// Get the current branch name (e.g. "main", "develop")
pub fn get_branch(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Get last commit as (short_hash, first_line_of_message)
pub fn get_last_commit(repo_path: &Path) -> Result<(String, String)> {
    // Use tab separator to safely split hash and message
    let output = run_git(repo_path, &["log", "-1", "--format=%h\t%s"])?;
    let mut parts = output.splitn(2, '\t');
    let hash = parts.next().unwrap_or("???????").to_string();
    let msg = parts.next().unwrap_or("no commits").to_string();
    Ok((hash, msg))
}

/// Check if the working tree has any uncommitted changes (staged or unstaged)
pub fn has_changes(repo_path: &Path) -> Result<bool> {
    let output = run_git(repo_path, &["status", "--porcelain"])?;
    Ok(!output.is_empty())
}

/// Returns (ahead, behind) commit counts relative to the tracking remote branch.
/// Returns (0, 0) when there is no upstream configured or the repo has no remote.
pub fn get_ahead_behind(repo_path: &Path) -> Result<(u32, u32)> {
    let output = run_git(
        repo_path,
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    )?;
    // Output format: "N\tM"  (N = commits ahead, M = commits behind)
    let mut parts = output.split('\t');
    let ahead = parts.next().unwrap_or("0").trim().parse::<u32>().unwrap_or(0);
    let behind = parts.next().unwrap_or("0").trim().parse::<u32>().unwrap_or(0);
    Ok((ahead, behind))
}

/// Checkout an existing branch
pub fn checkout(repo_path: &Path, branch: &str) -> Result<String> {
    run_git(repo_path, &["checkout", branch])
}

/// Pull from the tracking remote
pub fn pull(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["pull"])
}

/// Push to the tracking remote
pub fn push(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["push"])
}

/// Stage all changes (git add -A) then commit with the given message
pub fn commit(repo_path: &Path, message: &str) -> Result<String> {
    run_git(repo_path, &["add", "-A"])?;
    run_git(repo_path, &["commit", "-m", message])
}

/// Return parsed porcelain status as `(xy_code, filepath)` pairs.
/// `xy_code` is the 2-char status (e.g. "M ", " M", "??", "A ").
/// Returns an empty vec for a clean working tree.
pub fn status_files(repo_path: &Path) -> Result<Vec<(String, String)>> {
    let output = run_git(repo_path, &["status", "--porcelain"])?;
    let entries = output
        .lines()
        .filter(|l| l.len() >= 3)
        .map(|l| {
            let code = l[..2].to_string();
            let file = l[3..].to_string();
            (code, file)
        })
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Initialize a bare git repo in a temp directory with one initial commit
    fn create_temp_git_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        Command::new("git").args(["init"]).current_dir(p).output().unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(p)
            .output()
            .unwrap();
        // Create initial commit so HEAD resolves to a branch
        fs::write(p.join("readme.txt"), "hello").unwrap();
        Command::new("git").args(["add", "."]).current_dir(p).output().unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(p)
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn test_get_branch_returns_current_branch() {
        let dir = create_temp_git_repo();
        let branch = get_branch(dir.path()).unwrap();
        assert!(
            branch == "main" || branch == "master",
            "Expected main or master, got: {branch}"
        );
    }

    #[test]
    fn test_get_last_commit_returns_hash_and_message() {
        let dir = create_temp_git_repo();
        let (hash, msg) = get_last_commit(dir.path()).unwrap();
        assert!(!hash.is_empty() && hash != "???????", "Expected real hash, got: {hash}");
        assert_eq!(msg, "init");
    }

    #[test]
    fn test_has_changes_false_on_clean_repo() {
        let dir = create_temp_git_repo();
        assert!(!has_changes(dir.path()).unwrap());
    }

    #[test]
    fn test_has_changes_true_after_modification() {
        let dir = create_temp_git_repo();
        fs::write(dir.path().join("readme.txt"), "modified").unwrap();
        assert!(has_changes(dir.path()).unwrap());
    }

    #[test]
    fn test_commit_stages_and_commits() {
        let dir = create_temp_git_repo();
        fs::write(dir.path().join("new_file.txt"), "content").unwrap();
        let result = commit(dir.path(), "add new file");
        assert!(result.is_ok(), "commit failed: {:?}", result);
        // After commit, working tree should be clean
        assert!(!has_changes(dir.path()).unwrap());
    }
}
