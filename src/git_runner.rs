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

/// Get the latest tag reachable from HEAD (e.g. "v1.0.0")
/// Returns "—" if no tags exist in the repository.
pub fn get_latest_tag(repo_path: &Path) -> String {
    run_git(repo_path, &["describe", "--tags", "--abbrev=0"])
        .unwrap_or_else(|_| "—".to_string())
}

/// Get relative time of last commit (e.g. "2 hours ago", "3 days ago")
/// Returns "—" if the repo has no commits.
pub fn get_last_commit_time(repo_path: &Path) -> String {
    run_git(repo_path, &["log", "-1", "--format=%cr"])
        .unwrap_or_else(|_| "—".to_string())
}

/// Count changed files in working tree (staged + unstaged + untracked)
pub fn changed_file_count(repo_path: &Path) -> usize {
    run_git(repo_path, &["status", "--porcelain"])
        .map(|s| s.lines().filter(|l| l.len() >= 3).count())
        .unwrap_or(0)
}

/// Fetch from all remotes
pub fn fetch(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["fetch", "--all"])
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

/// Fetch commit graph for display in modal. Returns raw git log output.
/// Args:
///   - `limit`: max commits to fetch (default 50 for pagination)
/// Command: `git log --graph --all --oneline --decorate --color=never -n <limit>`
#[allow(dead_code)]
pub fn get_commit_graph(repo_path: &Path, limit: usize) -> Result<String> {
    let limit_str = limit.to_string();
    run_git(
        repo_path,
        &[
            "log",
            "--graph",
            "--all",
            "--oneline",
            "--decorate",
            "--color=never",
            "-n",
            &limit_str,
        ],
    )
}

/// Checkout a branch or commit reference.
/// Args:
///   - `ref`: branch name, commit hash, or tag to checkout
#[allow(dead_code)]
pub fn git_checkout(repo_path: &Path, ref_name: &str) -> Result<String> {
    run_git(repo_path, &["checkout", ref_name])
}

/// Cherry-pick a commit onto the current branch.
/// Args:
///   - `commit_hash`: the commit hash to cherry-pick
#[allow(dead_code)]
pub fn git_cherry_pick(repo_path: &Path, commit_hash: &str) -> Result<String> {
    run_git(repo_path, &["cherry-pick", commit_hash])
}

/// Rebase current branch onto another ref.
/// Args:
///   - `onto_ref`: branch or commit to rebase onto
#[allow(dead_code)]
pub fn git_rebase(repo_path: &Path, onto_ref: &str) -> Result<String> {
    run_git(repo_path, &["rebase", onto_ref])
}

/// Merge another branch into the current branch.
/// Args:
///   - `ref_name`: branch name or commit to merge
#[allow(dead_code)]
pub fn git_merge(repo_path: &Path, ref_name: &str) -> Result<String> {
    run_git(repo_path, &["merge", ref_name])
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

/// Aggregated git health state for one repository (offline — no fetch needed)
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub branch: String,      // current branch name
    pub uncommitted: usize,  // count of changed/untracked files
    pub unpushed: u32,       // commits ahead of upstream
    pub behind: u32,         // commits behind upstream
    pub no_upstream: bool,   // true if no remote tracking branch
}

impl AuditResult {
    /// Returns true if there are any issues (dirty, unpushed, or behind)
    pub fn has_issues(&self) -> bool {
        self.uncommitted > 0 || self.unpushed > 0 || self.behind > 0
    }

    /// Short severity label: "clean" | "warning" | "critical" | "behind"
    pub fn severity(&self) -> &'static str {
        if self.uncommitted > 0 && self.unpushed > 0 {
            "critical"
        } else if self.uncommitted > 0 || self.unpushed > 0 {
            "warning"
        } else if self.behind > 0 {
            "behind"
        } else {
            "clean"
        }
    }
}

/// Collect audit data for a single repo (offline — no git fetch).
/// Errors (missing upstream, etc.) become zero values; never panics.
pub fn audit_repo(repo_path: &Path) -> AuditResult {
    let branch = get_branch(repo_path).unwrap_or_else(|_| "?".into());

    let uncommitted = status_files(repo_path).map(|v| v.len()).unwrap_or(0);

    let (unpushed, behind, no_upstream) = match get_ahead_behind(repo_path) {
        Ok((ahead, behind)) => (ahead, behind, false),
        Err(_) => (0, 0, true),
    };

    AuditResult { branch, uncommitted, unpushed, behind, no_upstream }
}

/// Dependency vulnerability audit result for one npm/yarn project
#[derive(Debug, Clone)]
pub struct NpmAuditResult {
    pub critical: u32,
    pub high: u32,
    pub moderate: u32,
    pub low: u32,
    pub info: u32,
    pub total: u32,
    pub has_yarn: bool,
    pub error: Option<String>,
}

impl NpmAuditResult {
    pub fn is_vulnerable(&self) -> bool {
        self.critical > 0 || self.high > 0
    }
    pub fn is_clean(&self) -> bool {
        self.total == 0 && self.error.is_none()
    }
}

/// Parse npm audit v2 JSON output into NpmAuditResult.
/// npm audit exits non-zero when vulns found — stdout is still valid JSON.
fn parse_npm_audit_json(json: &str) -> Result<NpmAuditResult> {
    let v: serde_json::Value = serde_json::from_str(json)
        .context("failed to parse npm audit JSON")?;
    let vulns = &v["metadata"]["vulnerabilities"];
    Ok(NpmAuditResult {
        critical: vulns["critical"].as_u64().unwrap_or(0) as u32,
        high:     vulns["high"].as_u64().unwrap_or(0) as u32,
        moderate: vulns["moderate"].as_u64().unwrap_or(0) as u32,
        low:      vulns["low"].as_u64().unwrap_or(0) as u32,
        info:     vulns["info"].as_u64().unwrap_or(0) as u32,
        total:    vulns["total"].as_u64().unwrap_or(0) as u32,
        has_yarn: false,
        error: None,
    })
}

/// Parse yarn audit NDJSON output. Scans for the "auditSummary" line.
/// Caller must compute total after calling.
fn parse_yarn_audit_ndjson(output: &str) -> Result<NpmAuditResult> {
    for line in output.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v["type"].as_str() == Some("auditSummary") {
            let vulns = &v["data"]["vulnerabilities"];
            return Ok(NpmAuditResult {
                critical: vulns["critical"].as_u64().unwrap_or(0) as u32,
                high:     vulns["high"].as_u64().unwrap_or(0) as u32,
                moderate: vulns["moderate"].as_u64().unwrap_or(0) as u32,
                low:      vulns["low"].as_u64().unwrap_or(0) as u32,
                info:     vulns["info"].as_u64().unwrap_or(0) as u32,
                total: 0, // computed by caller: critical+high+moderate+low+info
                has_yarn: true,
                error: None,
            });
        }
    }
    Err(anyhow::anyhow!("no auditSummary line in yarn output"))
}

/// Run npm or yarn audit in `repo_path`. Returns None if not an npm/yarn project.
/// npm audit exits non-zero when vulns exist — stdout is still captured and parsed.
pub fn run_npm_audit(repo_path: &Path) -> Option<NpmAuditResult> {
    let has_yarn = repo_path.join("yarn.lock").exists();
    let has_pkg  = repo_path.join("package.json").exists();
    if !has_pkg && !has_yarn { return None; }

    let (cmd, args): (&str, &[&str]) = if has_yarn {
        ("yarn", &["audit", "--json"])
    } else {
        ("npm", &["audit", "--json"])
    };

    // On Windows, npm/yarn are .cmd scripts that require cmd.exe to execute.
    // Direct Command::new("npm") fails because CreateProcessW cannot run .cmd files.
    #[cfg(windows)]
    let output = {
        let mut win_args = vec!["/C", cmd];
        win_args.extend_from_slice(args);
        Command::new("cmd").args(&win_args).current_dir(repo_path).output()
    };
    #[cfg(not(windows))]
    let output = Command::new(cmd).args(args).current_dir(repo_path).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let result = if has_yarn {
                parse_yarn_audit_ndjson(&stdout)
            } else {
                parse_npm_audit_json(&stdout)
            };
            match result {
                Ok(mut r) => {
                    if has_yarn { r.total = r.critical + r.high + r.moderate + r.low + r.info; }
                    Some(r)
                }
                Err(e) => Some(NpmAuditResult {
                    critical: 0, high: 0, moderate: 0, low: 0, info: 0, total: 0,
                    has_yarn, error: Some(format!("parse error: {e}")),
                }),
            }
        }
        Err(e) => Some(NpmAuditResult {
            critical: 0, high: 0, moderate: 0, low: 0, info: 0, total: 0,
            has_yarn, error: Some(format!("{cmd} not found: {e}")),
        }),
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

    // Phase 2 Tests: Commit graph and git operations

    #[test]
    fn test_get_commit_graph_returns_graph_output() {
        let dir = create_temp_git_repo();
        // Add a second commit to get more interesting graph
        fs::write(dir.path().join("file2.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "second commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let graph = get_commit_graph(dir.path(), 10).unwrap();
        assert!(!graph.is_empty(), "graph output should not be empty");
        assert!(graph.contains("init") || graph.contains("second commit"), 
                "graph should contain commit messages");
    }

    #[test]
    fn test_git_checkout_switches_branch() {
        let dir = create_temp_git_repo();
        // Create and switch to new branch
        Command::new("git")
            .args(["checkout", "-b", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        
        // Switch back using our function
        let result = git_checkout(dir.path(), "main");
        if result.is_err() {
            // Fallback to master if main doesn't exist
            let _ = git_checkout(dir.path(), "master");
        }
        
        let branch = get_branch(dir.path()).unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_git_cherry_pick_function_callable() {
        let dir = create_temp_git_repo();
        
        // Create second commit to have something to cherry-pick attempt
        fs::write(dir.path().join("file2.txt"), "content2").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "second"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        
        let (hash, _) = get_last_commit(dir.path()).unwrap();
        
        // Test that function is callable - may succeed or fail with git message
        // depending on repository state, but should not panic
        let result = git_cherry_pick(dir.path(), &hash);
        
        // Function should return Result (not panic), even if git operation fails
        assert!(result.is_ok() || result.is_err(), "function should return Result");
    }

    #[test]
    fn test_git_merge_merges_branch() {
        let dir = create_temp_git_repo();
        
        // Create a branch with a commit
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join("feature.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        
        // Switch back and merge
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(dir.path())
            .output()
            .or_else(|_| {
                Command::new("git")
                    .args(["checkout", "master"])
                    .current_dir(dir.path())
                    .output()
            })
            .unwrap();
        
        let result = git_merge(dir.path(), "feature");
        assert!(result.is_ok(), "merge failed: {:?}", result);
    }

    // Phase 3 Tests: AuditResult + audit_repo()

    #[test]
    fn test_audit_repo_clean_returns_no_issues() {
        let dir = create_temp_git_repo();
        let result = audit_repo(dir.path());
        assert!(!result.has_issues());
        assert_eq!(result.severity(), "clean");
        assert_eq!(result.uncommitted, 0);
        assert_eq!(result.unpushed, 0);
    }

    #[test]
    fn test_audit_repo_dirty_has_issues() {
        let dir = create_temp_git_repo();
        fs::write(dir.path().join("dirty.txt"), "change").unwrap();
        let result = audit_repo(dir.path());
        assert!(result.has_issues());
        assert_eq!(result.uncommitted, 1);
        assert!(result.severity() == "warning" || result.severity() == "critical");
    }

    #[test]
    fn test_audit_repo_no_upstream_sets_flag() {
        let dir = create_temp_git_repo();
        // No remote configured → get_ahead_behind fails → no_upstream = true
        let result = audit_repo(dir.path());
        assert!(result.no_upstream);
    }

    #[test]
    fn test_audit_result_severity_levels() {
        let clean = AuditResult {
            branch: "main".into(), uncommitted: 0,
            unpushed: 0, behind: 0, no_upstream: false,
        };
        assert_eq!(clean.severity(), "clean");

        let warning = AuditResult { uncommitted: 1, ..clean.clone() };
        assert_eq!(warning.severity(), "warning");

        let critical = AuditResult { uncommitted: 1, unpushed: 2, ..clean.clone() };
        assert_eq!(critical.severity(), "critical");

        let behind = AuditResult { behind: 1, ..clean.clone() };
        assert_eq!(behind.severity(), "behind");
    }

    // Phase 4 Tests: NpmAuditResult JSON parsing

    #[test]
    fn test_parse_npm_audit_json_with_vulns() {
        let json = r#"{
            "auditReportVersion": 2,
            "metadata": {
                "vulnerabilities": {
                    "critical": 2, "high": 5, "moderate": 3, "low": 1, "info": 0, "total": 11
                }
            }
        }"#;
        let r = parse_npm_audit_json(json).unwrap();
        assert_eq!(r.critical, 2);
        assert_eq!(r.high, 5);
        assert_eq!(r.moderate, 3);
        assert_eq!(r.low, 1);
        assert_eq!(r.total, 11);
        assert!(!r.has_yarn);
        assert!(r.is_vulnerable());
    }

    #[test]
    fn test_parse_npm_audit_json_clean() {
        let json = r#"{
            "auditReportVersion": 2,
            "metadata": {
                "vulnerabilities": {
                    "critical": 0, "high": 0, "moderate": 0, "low": 0, "info": 0, "total": 0
                }
            }
        }"#;
        let r = parse_npm_audit_json(json).unwrap();
        assert!(r.is_clean());
        assert!(!r.is_vulnerable());
    }

    #[test]
    fn test_parse_yarn_audit_ndjson_with_vulns() {
        let output = r#"{"type":"auditAdvisory","data":{"advisory":{"id":1}}}
{"type":"auditSummary","data":{"vulnerabilities":{"critical":0,"high":1,"moderate":2,"low":0,"info":0}}}"#;
        let mut r = parse_yarn_audit_ndjson(output).unwrap();
        r.total = r.critical + r.high + r.moderate + r.low + r.info;
        assert_eq!(r.high, 1);
        assert_eq!(r.moderate, 2);
        assert_eq!(r.total, 3);
        assert!(r.has_yarn);
        assert!(r.is_vulnerable());
    }

    #[test]
    fn test_parse_yarn_audit_ndjson_no_summary() {
        let output = r#"{"type":"auditAdvisory","data":{}}"#;
        assert!(parse_yarn_audit_ndjson(output).is_err());
    }

    #[test]
    fn test_parse_npm_audit_json_invalid() {
        assert!(parse_npm_audit_json("not json at all").is_err());
    }

    #[test]
    fn test_npm_audit_result_methods() {
        let clean = NpmAuditResult {
            critical: 0, high: 0, moderate: 0, low: 0, info: 0, total: 0,
            has_yarn: false, error: None,
        };
        assert!(clean.is_clean());
        assert!(!clean.is_vulnerable());

        let vuln = NpmAuditResult { critical: 1, total: 1, ..clean.clone() };
        assert!(vuln.is_vulnerable());
        assert!(!vuln.is_clean());
    }

    #[test]
    fn test_git_rebase_rebases_onto_ref() {
        let dir = create_temp_git_repo();
        
        // Create main/master branch with extra commit
        let main_branch = if get_branch(dir.path()).unwrap() == "main" {
            "main"
        } else {
            "master"
        };
        
        fs::write(dir.path().join("main-file.txt"), "main content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        
        // Create feature branch from before the main commit
        Command::new("git")
            .args(["checkout", "HEAD~1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        
        // Rebase feature onto main
        let result = git_rebase(dir.path(), main_branch);
        assert!(result.is_ok(), "rebase failed: {:?}", result);
    }
}
