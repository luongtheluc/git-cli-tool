# Codebase Summary

## Overview

Rust CLI application structured in 5 modules + 1 binary installer. Total source LOC: ~570 (excluding tests).

## File-by-File Breakdown

### main.rs (87 LOC)
**Purpose:** Application entry point, command dispatch, batch execution orchestration

**Key Components:**
- `main() -> Result<()>` — Parse CLI args, match command, invoke handlers
- `run_batch<F>(repos, selected_indices, operation)` — Execute operation sequentially per repo

**Responsibilities:**
- Parse command-line arguments via clap
- Scan workspace for repositories
- Show interactive selection UI
- Execute operation on each selected repo
- Handle per-repo errors (red print, continue)

**Dependencies:** anyhow, clap, colored (for styling), repo_scanner, ui, git_runner

**Key Logic:**
```rust
// Pattern: all commands follow this sequence
let repos = repo_scanner::scan_repos(&cwd)?;
let selected = ui::select_repos(&repos)?;
run_batch(&repos, &selected, |path| git_runner::operation(path, args...));
```

### cli.rs (37 LOC)
**Purpose:** Command-line argument definitions and parsing models

**Key Components:**
- `Cli` struct — clap Parser; contains `command: Commands` enum
- `Commands` enum — Subcommand variants: List, Checkout, Pull, Push, Commit, Status

**Responsibilities:**
- Define CLI argument schema via clap derive macros
- Parse user input into Rust structs
- Provide help text and version info

**Dependencies:** clap 4 (derive feature)

**Variants:**
- `List` — no args
- `Checkout { branch: String }` — positional branch name
- `Pull` — no args
- `Push` — no args
- `Commit { #[arg(short, long)] message: String }` — `-m` or `--message` flag
- `Status` — no args

### repo_scanner.rs (129 LOC)
**Purpose:** Discover Git repositories in workspace, collect metadata in parallel

**Key Components:**
- `RepoInfo` struct — name, path, branch, commit hash/msg, dirty flag
- `scan_repos(workspace_dir) -> Result<Vec<RepoInfo>>` — Main entry point
- `find_git_repos(workspace_dir) -> Vec<PathBuf>` — WalkDir at depth 2
- `build_repo_info(path) -> Result<RepoInfo>` — Collect metadata for one repo
- **4 unit tests** covering discovery, sorting, empty dirs, non-git exclusion

**Responsibilities:**
- Walk workspace directory to depth 2 only (avoids nested repos)
- Identify directories containing `.git` folder
- Parallelize metadata collection with rayon par_iter
- Sort results alphabetically
- Gracefully fall back on git errors (branch="?", message="no commits")

**Dependencies:** anyhow, rayon, walkdir, git_runner

**Key Design:**
- `min_depth(2).max_depth(2)` — only direct children
- `par_iter` on collected paths for parallel metadata
- `.filter_map(|path| build_repo_info(path).ok())` — silent on error repos
- Sort post-collection for alphabetical order

### git_runner.rs (147 LOC)
**Purpose:** Wrapper around system Git binary; execute Git commands safely

**Key Components:**
- `run_git(repo_path, args) -> Result<String>` — Core wrapper; uses `git -C <path>`
- Helper functions: `get_branch()`, `get_last_commit()`, `has_changes()`, `checkout()`, `pull()`, `push()`, `commit()`, `status()`
- **5 unit tests** covering branch, commit, changes detection, commits

**Responsibilities:**
- Invoke system Git via std::process::Command with `git -C` (avoid chdir)
- Parse and format git output (commit hash + message split by tab)
- Detect working tree changes with `git status --porcelain`
- Stage and commit with two-step sequence (add -A, then commit)
- Report errors from stdout or stderr
- Check Git installation before execution

**Dependencies:** anyhow, std::process::Command

**Key Logic:**
```rust
// All commands routed through run_git()
Command::new("git")
    .arg("-C").arg(path_str)
    .args(args)
    .output()
    // Parse stdout on success, stderr on failure
```

### ui.rs (82 LOC)
**Purpose:** Terminal UI — interactive repo selection and table display

**Key Components:**
- `select_repos(repos) -> Result<Vec<usize>>` — dialoguer MultiSelect returns indices
- `print_repo_table(repos)` — Aligned ASCII table with ANSI colors
- `format_repo_line(repo, name_w, branch_w) -> String` — Plain-text line for selection prompt
- `col_width(lengths, min) -> usize` — Compute column width with floor

**Responsibilities:**
- Present interactive multi-select menu
- Calculate column widths dynamically
- Pad strings BEFORE colorizing (preserves alignment with ANSI codes)
- Return repo indices (not references) to avoid lifetime issues
- Handle empty repo list gracefully

**Dependencies:** anyhow, colored, dialoguer, repo_scanner

**Key Design:**
- Padding applied before color: `format!("{:<name_w$}", repo.name).cyan()`
- Plain-text items for MultiSelect (dialoguer renders its own styling)
- Column widths: max(all values, minimum floor)
- Dirty indicator: yellow `*` appended after message

### setup.rs (188 LOC)
**Purpose:** Installer binary — copy repo binary to PATH, register in system

**Key Components:**
- `main()` — orchestrate install steps
- `find_source_binary()` — Locate repo.exe next to setup.exe
- `get_install_dir()` -> `~/.repo/bin/`
- `add_to_path(dir)` — Persistent PATH registration
- Platform-specific: Windows (winreg), Unix (shell profile)

**Responsibilities:**
- Locate `repo` binary in same directory as setup executable
- Create install directory (~/.repo/bin/)
- Copy binary to install directory
- Set executable permissions on Unix (chmod 0o755)
- Update persistent PATH (Windows registry or shell profile)
- Print user-friendly installation progress

**Dependencies:** anyhow, colored, std::env/fs, winreg (Windows-only), std::os::unix::fs (Unix-only)

**Platform Logic:**
- **Windows:** Read/write HKCU\Environment\Path via winreg
- **Unix:** Append `export PATH="$HOME/.repo/bin:$PATH"` to ~/.profile

## Test Coverage

### repo_scanner tests (4)
- `test_scan_finds_repos` — discovers 2 repos among non-repos
- `test_scan_empty_dir_returns_empty` — empty workspace returns []
- `test_repos_sorted_alphabetically` — 3 repos sorted by name
- `test_non_git_dirs_excluded` — non-.git dirs filtered out

### git_runner tests (5)
- `test_get_branch_returns_current_branch` — branch name matches current
- `test_get_last_commit_returns_hash_and_message` — commit info parsed correctly
- `test_has_changes_false_on_clean_repo` — clean repo returns false
- `test_has_changes_true_after_modification` — dirty repo returns true
- `test_commit_stages_and_commits` — two-step add + commit succeeds

All tests use `tempfile` crate for isolated test repos.

## Build Artifacts

```
Cargo.toml defines:
[[bin]]
name = "repo"
path = "src/main.rs"

[[bin]]
name = "setup"
path = "src/setup.rs"

Release build:
cargo build --release
→ target/release/repo.exe (or repo)
→ target/release/setup.exe (or setup)
```

## Dependency Analysis

| Crate | Version | Platform | Purpose |
|-------|---------|----------|---------|
| clap | 4 | all | CLI derive parsing |
| dialoguer | 0.11 | all | MultiSelect TUI |
| colored | 2 | all | ANSI terminal colors |
| walkdir | 2 | all | Recursive directory walk |
| anyhow | 1 | all | Error handling context |
| rayon | 1 | all | Parallel metadata collection |
| console | 0.15 | all | Terminal utilities (dialoguer dep) |
| winreg | 0.52 | Windows | Registry access for PATH setup |
| tempfile | 3 | all | Test temp directories |

## Module Interaction Graph

```
main.rs
├── cli (parse)
├── repo_scanner::scan_repos()
│   ├── find_git_repos() → Vec<PathBuf>
│   └── build_repo_info() → Vec<RepoInfo>
│       └── git_runner::{get_branch, get_last_commit, has_changes}()
├── ui::select_repos() → Vec<usize>
└── run_batch()
    └── git_runner::{checkout, pull, push, commit, status}()
        └── run_git() → Result<String>

setup.rs (independent binary)
├── find_source_binary()
├── add_to_path() [platform-specific]
│   └── winreg [Windows]
└── fs::copy() [Unix chmod]
```

## Key Code Patterns

### Error Handling
- All functions return `Result<T>` or `Result<()>` (anyhow)
- `.context()` adds context to errors
- `run_batch()` catches errors and continues (non-fatal)
- `.unwrap_or_else()` provides fallback values in repo metadata

### Parallelization
- `repo_scanner`: `paths.into_par_iter().filter_map(build_repo_info).collect()`
- No shared state; each repo metadata independent

### UI Alignment
- Compute max column width: `col_width(values, min_floor)`
- Pad raw strings: `format!("{:<width$}", value)`
- Apply color after padding: `.cyan().to_string()`

### Git Invocation
- All commands use `git -C <path> <args>`
- Never change directory
- Tab-separated output parsing: `output.splitn(2, '\t')`
- Graceful fallback on missing Git

## Performance Characteristics

- **Scan:** O(n) directory walks + O(n) parallel git calls; ~40ms per repo on modern hardware
- **Selection:** O(n) dialoguer render; <100ms for 50 repos
- **Batch:** O(m) sequential git calls where m = selected repos; network-bound (pull/push)

## Code Quality Metrics

- **Total Source LOC:** ~570 (excluding tests)
- **Test LOC:** ~130 (9 unit tests)
- **Cyclomatic Complexity:** Low (no nested loops, simple error handling)
- **Safe Code:** 100% (no unsafe blocks)
- **Documentation:** All public functions doc-commented
