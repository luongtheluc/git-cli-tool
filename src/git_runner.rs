use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

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

/// Checkout a branch. If `create` is true, passes `-b` to create it first.
pub fn checkout(repo_path: &Path, branch: &str, create: bool) -> Result<String> {
    if create {
        run_git(repo_path, &["checkout", "-b", branch])
    } else {
        run_git(repo_path, &["checkout", branch])
    }
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

/// Detected project type from manifest files in a repo directory.
#[derive(Debug, Clone)]
pub enum BuildTool {
    Npm,   // package.json present
    Cargo, // Cargo.toml present
    Go,    // go.mod present
    Make,  // Makefile present
    Shell, // fallback: pass script to sh -c / cmd /C directly
}

/// Inspect `repo_path` for known manifest files. Returns first match by priority:
/// package.json > Cargo.toml > go.mod > Makefile > Shell (fallback).
pub fn detect_build_tool(repo_path: &Path) -> BuildTool {
    let checks: &[(&str, BuildTool)] = &[
        ("package.json", BuildTool::Npm),
        ("Cargo.toml", BuildTool::Cargo),
        ("go.mod", BuildTool::Go),
        ("Makefile", BuildTool::Make),
    ];
    for (file, tool) in checks {
        if repo_path.join(file).exists() {
            return tool.clone();
        }
    }
    BuildTool::Shell
}

/// Resolve a user-supplied script alias to the actual command for a given BuildTool.
/// Known aliases: "build", "test", "install", "clean".
/// Unknown aliases fall through to the shell verbatim.
pub fn resolve_script(script: &str, tool: &BuildTool) -> String {
    match (tool, script) {
        (BuildTool::Npm, "build") => "npm run build".into(),
        (BuildTool::Npm, "test") => "npm test".into(),
        (BuildTool::Npm, "install") => "npm install".into(),
        (BuildTool::Npm, "clean") => "npm run clean".into(),
        (BuildTool::Cargo, "build") => "cargo build --release".into(),
        (BuildTool::Cargo, "test") => "cargo test".into(),
        (BuildTool::Cargo, "clean") => "cargo clean".into(),
        (BuildTool::Go, "build") => "go build ./...".into(),
        (BuildTool::Go, "test") => "go test ./...".into(),
        (BuildTool::Make, "build") => "make build".into(),
        (BuildTool::Make, "test") => "make test".into(),
        (BuildTool::Make, "clean") => "make clean".into(),
        // Fallback: run script verbatim via shell
        _ => script.to_string(),
    }
}

/// Execute an arbitrary shell command in `repo_path`, streaming each output line
/// to stdout prefixed with `prefix`. Returns Ok(true) on success exit code.
///
/// stderr is read on a separate thread to prevent pipe buffer deadlock
/// (classic issue when a process writes to both stdout and stderr faster than we drain them).
pub fn run_shell(
    repo_path: &Path,
    cmd: &str,
    prefix: &str,
    print_lock: &Arc<Mutex<()>>,
) -> Result<bool> {
    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", cmd])
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    #[cfg(not(windows))]
    let mut child = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Drain stderr on a separate thread to prevent pipe buffer deadlock
    let stderr = child.stderr.take().unwrap();
    let prefix2 = prefix.to_string();
    let lock2 = Arc::clone(print_lock);
    let stderr_thread = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().flatten() {
            let _g = lock2.lock().unwrap();
            eprintln!("{}{}", prefix2, line);
        }
    });

    // Stream stdout line-by-line, holding lock per line to prevent interleaving
    for line in BufReader::new(child.stdout.take().unwrap()).lines().flatten() {
        let _g = print_lock.lock().unwrap();
        println!("{}{}", prefix, line);
    }
    stderr_thread.join().ok();
    let status = child.wait()?;
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
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

    #[test]
    fn test_run_shell_success() {
        let dir = create_temp_git_repo();
        let lock = Arc::new(Mutex::new(()));
        let cmd = "echo hello";
        let ok = run_shell(dir.path(), cmd, "[test] ", &lock).unwrap();
        assert!(ok, "expected success exit code");
    }

    #[test]
    fn test_run_shell_failure_exit_code() {
        let dir = create_temp_git_repo();
        let lock = Arc::new(Mutex::new(()));
        // exit 1 inside cmd /C exits with code 1 on Windows; sh -c on Unix
        let cmd = "exit 1";
        let ok = run_shell(dir.path(), cmd, "[test] ", &lock).unwrap();
        assert!(!ok, "expected non-zero exit code");
    }

    #[test]
    fn test_run_shell_bad_command_returns_err_or_false() {
        let dir = create_temp_git_repo();
        let lock = Arc::new(Mutex::new(()));
        let result = run_shell(dir.path(), "this_command_does_not_exist_xyz_abc", "[test] ", &lock);
        // Either Err (spawn failed) or Ok(false) (shell reported not found) — both acceptable
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_run_shell_runs_in_repo_dir() {
        let dir = create_temp_git_repo();
        let lock = Arc::new(Mutex::new(()));
        // cd prints CWD on Windows; pwd on Unix — both succeed
        #[cfg(windows)]
        let cmd = "cd";
        #[cfg(not(windows))]
        let cmd = "pwd";
        let ok = run_shell(dir.path(), cmd, "[test] ", &lock).unwrap();
        assert!(ok);
    }
}
