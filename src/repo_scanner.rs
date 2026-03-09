use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::git_runner;

/// Metadata collected for a single discovered git repository
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub last_commit_hash: String,
    pub last_commit_msg: String,
    pub has_uncommitted_changes: bool,
    /// Commits local branch is ahead of its remote tracking branch (0 if no remote)
    pub ahead: u32,
    /// Commits local branch is behind its remote tracking branch (0 if no remote)
    pub behind: u32,
    /// Latest reachable tag (e.g. "v1.0.0") or "—" if none
    pub latest_tag: String,
    /// Relative time of last commit (e.g. "2 hours ago")
    pub last_commit_time: String,
    /// Number of changed files (staged + unstaged + untracked)
    pub changed_file_count: usize,
}

/// Scan `workspace_dir` for direct-child git repos, collecting metadata in parallel.
/// Results are sorted alphabetically by repo name.
pub fn scan_repos(workspace_dir: &Path) -> Result<Vec<RepoInfo>> {
    let repo_paths = find_git_repos(workspace_dir);
    let mut repos: Vec<RepoInfo> = repo_paths
        .into_par_iter()
        .filter_map(|path| build_repo_info(path).ok())
        .collect();
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

/// Walk `workspace_dir` at depth 2 to find directories containing a `.git` folder.
/// Only direct children of workspace_dir are considered (not nested repos).
/// Deduplicates by canonical path to handle NTFS junction points on Windows.
fn find_git_repos(workspace_dir: &Path) -> Vec<PathBuf> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    WalkDir::new(workspace_dir)
        .min_depth(2)
        .max_depth(2)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == ".git" && e.file_type().is_dir())
        .filter_map(|e| e.path().parent().map(PathBuf::from))
        .filter(|repo_path| {
            // Canonicalize to resolve junctions/symlinks; fall back to raw path if it fails
            let canonical = repo_path.canonicalize().unwrap_or_else(|_| repo_path.clone());
            seen.insert(canonical)
        })
        .collect()
}

/// Collect git metadata for a single repo path. Falls back to placeholder values on error.
fn build_repo_info(path: PathBuf) -> Result<RepoInfo> {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let branch = git_runner::get_branch(&path).unwrap_or_else(|_| "?".to_string());
    let (last_commit_hash, last_commit_msg) = git_runner::get_last_commit(&path)
        .unwrap_or_else(|_| ("???????".to_string(), "no commits".to_string()));
    let has_uncommitted_changes = git_runner::has_changes(&path).unwrap_or(false);
    let (ahead, behind) = git_runner::get_ahead_behind(&path).unwrap_or((0, 0));
    let latest_tag = git_runner::get_latest_tag(&path);
    let last_commit_time = git_runner::get_last_commit_time(&path);
    let changed_file_count = if has_uncommitted_changes {
        git_runner::changed_file_count(&path)
    } else {
        0
    };

    Ok(RepoInfo {
        name,
        path,
        branch,
        last_commit_hash,
        last_commit_msg,
        has_uncommitted_changes,
        ahead,
        behind,
        latest_tag,
        last_commit_time,
        changed_file_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_bare_git_repo(parent: &Path, name: &str) -> PathBuf {
        let repo_path = parent.join(name);
        fs::create_dir_all(&repo_path).unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        repo_path
    }

    #[test]
    fn test_scan_finds_repos() {
        let workspace = TempDir::new().unwrap();
        init_bare_git_repo(workspace.path(), "alpha");
        init_bare_git_repo(workspace.path(), "beta");
        fs::create_dir(workspace.path().join("not-a-repo")).unwrap();

        let repos = scan_repos(workspace.path()).unwrap();
        assert_eq!(repos.len(), 2);
        let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_scan_empty_dir_returns_empty() {
        let workspace = TempDir::new().unwrap();
        let repos = scan_repos(workspace.path()).unwrap();
        assert!(repos.is_empty());
    }

    #[test]
    fn test_repos_sorted_alphabetically() {
        let workspace = TempDir::new().unwrap();
        init_bare_git_repo(workspace.path(), "zebra");
        init_bare_git_repo(workspace.path(), "apple");
        init_bare_git_repo(workspace.path(), "mango");

        let repos = scan_repos(workspace.path()).unwrap();
        assert_eq!(repos[0].name, "apple");
        assert_eq!(repos[1].name, "mango");
        assert_eq!(repos[2].name, "zebra");
    }

    #[test]
    fn test_non_git_dirs_excluded() {
        let workspace = TempDir::new().unwrap();
        fs::create_dir(workspace.path().join("plain-dir")).unwrap();
        init_bare_git_repo(workspace.path(), "git-repo");

        let repos = scan_repos(workspace.path()).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "git-repo");
    }
}
