# Code Standards & Conventions

## Rust Version & Edition

- **Edition:** 2021
- **Toolchain:** stable-x86_64-pc-windows-msvc (1.93.1)
- **MSRV:** Rust 1.70+ (recommended for clap 4 compatibility)

## File Organization

### Module Structure
```
src/
├── main.rs           — entry point, command dispatch, run_batch()
├── cli.rs            — clap argument parsing structs
├── repo_scanner.rs   — directory walk, metadata collection
├── git_runner.rs     — system Git wrapper; npm/yarn audit
├── ui.rs             — terminal UI (selection, table)
├── text_utils.rs     — Unicode display width utilities
├── commit_graph.rs   — commit graph parsing and pagination
├── tui/              — interactive full-screen TUI (6 modules)
│   ├── mod.rs
│   ├── app.rs
│   ├── ui.rs
│   ├── events.rs
│   ├── batch_ops.rs
│   └── modal.rs      — modal dialog system
└── setup.rs          — installer binary (separate binary target)
```

**Naming Convention:** snake_case for .rs files; each module handles a single domain.

### Module Boundaries

- **cli.rs** — only argument parsing; no business logic
- **repo_scanner.rs** — discovery and metadata; depends on git_runner
- **git_runner.rs** — system Git only; no UI or scanning logic
- **ui.rs** — presentation only; depends on repo_scanner types
- **main.rs** — orchestration only; calls other modules, doesn't duplicate logic

## Code Style

### Naming Conventions

- **Functions:** snake_case (Rust standard)
  - `scan_repos()`, `run_git()`, `format_repo_line()`
- **Types/Structs:** PascalCase (Rust standard)
  - `RepoInfo`, `Cli`, `Commands`
- **Constants:** UPPER_SNAKE_CASE (if used)
  - Not heavily used; prefer module-level derived values
- **Variables:** snake_case
  - `repo_path`, `selected_indices`, `branch_w`

### Doc Comments

All public functions must have doc comments. Use triple-slash `///`:

```rust
/// Brief description (one line).
///
/// Longer description if needed, explaining args, returns, and edge cases.
///
/// # Arguments
/// * `repo_path` — path to the repo directory
///
/// # Returns
/// Current branch name or error if git fails
pub fn get_branch(repo_path: &Path) -> Result<String> { }
```

Exceptions:
- Main entry point `main()` — documented via clap Cli struct
- Private functions — doc comment if behavior is non-obvious (>5 lines)

### Line Length

Target 100 chars; 120 char absolute limit. Rationale: readability on standard monitors.

```rust
// Good: wrapped logically
println!(
    "{:<name_w$}  {}  {} {}{}",
    repo.name, branch_col, hash_col, repo.last_commit_msg, dirty,
);

// Bad: single long line
println!("{:<name_w$}  {}  {} {}{}", repo.name, branch_col, hash_col, repo.last_commit_msg, dirty);
```

### Imports

- Use absolute paths from crate root (`use crate::module_name;`)
- Group stdlib, external crates, internal modules (in that order)
- Avoid wildcard imports except for prelude-style modules

```rust
use anyhow::Result;
use std::path::Path;
use std::process::Command;

use crate::git_runner;
```

## Error Handling

### Error Type
Use `anyhow::Result<T>` everywhere:

```rust
fn scan_repos(workspace_dir: &Path) -> Result<Vec<RepoInfo>> {
    // ...
}
```

**Never:**
- Bare `.unwrap()` in library code (only in tests/main)
- `.expect()` in hot paths
- Ignore errors silently (use `.ok()` only if documented)

### Adding Context
Use `.context()` for operation-level errors:

```rust
fs::create_dir_all(&install_dir)
    .context("failed to create install directory")?;

env::var("USERPROFILE")
    .map(PathBuf::from)
    .context("USERPROFILE environment variable not set")
```

### Fallback Values
In non-critical paths (metadata collection), use `.unwrap_or_else()` to provide sensible defaults:

```rust
let branch = git_runner::get_branch(&path)
    .unwrap_or_else(|_| "?".to_string());

let has_uncommitted_changes = git_runner::has_changes(&path)
    .unwrap_or(false);
```

This prevents one bad repo from crashing the entire scan.

## Concurrency

### Parallelization with Rayon
Use `par_iter()` for independent operations only. Current usage:

```rust
let mut repos: Vec<RepoInfo> = repo_paths
    .into_par_iter()
    .filter_map(|path| build_repo_info(path).ok())
    .collect();
```

**Safety rules:**
- No shared mutable state
- Each iteration produces independent result
- `.filter_map()` on Results is safe for fallback

**Never** use rayon for:
- Operations with shared resources (file I/O to same directory)
- Dependencies between iterations
- UI operations (dialoguer is single-threaded)

## System Calls

### Git Invocation Pattern
All Git commands use `git -C <path>` to avoid directory changes:

```rust
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
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        Err(anyhow::anyhow!("{}", msg))
    }
}
```

**Why git -C:** Avoids process synchronization issues; each call is independent.

### npm/yarn Audit Invocation (Phase 8.2)
Windows-specific handling: npm and yarn are `.cmd` scripts requiring `cmd.exe /C`:

```rust
#[cfg(windows)]
fn run_npm_audit(repo_path: &Path) -> Result<String> {
    let output = Command::new("cmd")
        .args(&["/C", "npm audit --json"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run npm audit")?;
    // ... parse JSON response
}

#[cfg(not(windows))]
fn run_npm_audit(repo_path: &Path) -> Result<String> {
    let output = Command::new("npm")
        .args(&["audit", "--json"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run npm audit")?;
    // ... parse JSON response
}
```

**Why conditional compilation:** npm/yarn are .cmd scripts on Windows, requiring cmd.exe wrapper. Unix systems execute directly.

## Terminal I/O

### Color Output
Use `colored` crate; apply color after padding to preserve column width:

```rust
// GOOD: pad first, color second
let branch_col = format!("{:<branch_w$}", repo.branch).cyan().to_string();

// BAD: color first, padding second (breaks alignment)
let branch_col = repo.branch.cyan();
let padded = format!("{:<branch_w$}", branch_col);
```

### Interactive UI
dialoguer handles its own rendering. Provide plain-text items:

```rust
let items: Vec<String> = repos
    .iter()
    .map(|r| format_repo_line(r, name_w, branch_w))
    .collect();

let selections = MultiSelect::new()
    .with_prompt("Select repositories...")
    .items(&items)
    .interact()?;
```

## Testing

### Test Organization
Place tests in same file using `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior() {
        // ...
    }
}
```

### Test Isolation
Use `tempfile::TempDir` for filesystem isolation:

```rust
#[test]
fn test_scan_finds_repos() {
    let workspace = TempDir::new().unwrap();
    init_bare_git_repo(workspace.path(), "alpha");
    let repos = scan_repos(workspace.path()).unwrap();
    assert_eq!(repos.len(), 1);
}
```

### Test Naming
`test_<subject>_<expectation>`:
- `test_scan_finds_repos` — scan discovers repos
- `test_has_changes_true_after_modification` — dirty detection works
- `test_get_branch_returns_current_branch` — branch retrieval works

### Coverage
- Happy path: basic operation succeeds
- Error path: graceful failure on git error
- Edge case: empty input, special characters, permission errors

**Current Coverage:** 54 tests across scanner (4), git_runner (20), commit_graph (13), ui/modal (8), and TUI (9).

## Dependencies

### Approved Crates
These are vetted for production use:

| Crate | Version | Purpose | Rationale |
|-------|---------|---------|-----------|
| clap | 4 | CLI parsing | Standard, actively maintained |
| dialoguer | 0.11 | Interactive TUI | Stable API, lightweight |
| colored | 2 | ANSI colors | Lightweight, no dependencies |
| walkdir | 2 | Directory traversal | Trusted, efficient |
| anyhow | 1 | Error context | Minimal overhead, ergonomic |
| rayon | 1 | Parallelization | Battle-tested work-stealing |
| console | 0.15 | Terminal utilities | dialoguer dependency |
| serde_json | 1 | JSON parsing | Standard, used for npm/yarn audit |
| unicode-width | 0.1 | Display width | Unicode-aware text layout |
| unicode-segmentation | 1.10 | Unicode support | Grapheme cluster handling |
| ratatui | 0.26 | Terminal UI framework | Modern, cross-platform TUI |
| crossterm | 0.27 | Terminal manipulation | ratatui dependency, cross-platform |
| winreg | 0.52 | Windows registry | setup.rs only, Windows-specific |

### Adding New Dependencies
Before adding a new crate:
1. Check if stdlib or approved crates already solve it
2. Verify maintenance status (recent commits, open PRs)
3. Check security audit: `cargo audit`
4. Update Cargo.toml with feature specifications (don't use `*` versions)
5. Run `cargo tree` to check transitive deps

## Performance Considerations

### Scanning Performance
- Parallel metadata collection: ~40ms per repo on modern hardware
- Use `.filter_map().collect()` to skip errors without allocating result vector
- WalkDir at depth 2 avoids deep traversal

### Batch Execution
- Sequential per-repo execution (network-bound anyway)
- Print headers before operation (shows progress)
- No in-memory buffering of output (print immediately)

### Memory
- Allocate repo list once, reuse for all operations
- ui::select_repos returns indices (not clones)
- No unnecessary String allocations in hot loops

## Platform-Specific Code

### Conditional Compilation
Use `#[cfg(...)]` attributes for platform-specific logic:

```rust
#[cfg(windows)]
let name = "repo.exe";
#[cfg(not(windows))]
let name = "repo";

#[cfg(target_os = "windows")]
fn add_to_registry(dir: &Path) -> Result<()> { }

#[cfg(unix)]
fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
```

### Tested Platforms
- **Windows 11** (primary; native build)
- **macOS** (should work; test before release)
- **Linux** (should work; test before release)

## Build Configuration

### Release Profile
```toml
[profile.release]
opt-level = 3
lto = true
strip = true
```

- `opt-level = 3`: aggressive optimization
- `lto = true`: link-time optimization for smaller binary
- `strip = true`: strip symbols for smaller size

### Build Checks
Before commit:
```bash
cargo clippy -- -D warnings
cargo fmt --check
cargo test
cargo build --release
```

## Unsafe Code

**Policy:** No unsafe code in this project.

If necessary in future (e.g., FFI):
1. Isolate in separate module with `SAFETY:` comment
2. Document invariants clearly
3. Require code review from at least two developers
4. Add comprehensive tests

## Comments & Documentation

### Inline Comments
- Explain "why", not "what"
- Keep to one line when possible

```rust
// Good: explains intent
let sep = if cfg!(windows) { ';' } else { ':' }; // PATH separator differs by OS

// Bad: obvious from code
let sep = if cfg!(windows) { ';' } else { ':' }; // set sep to semicolon or colon
```

### Doc Comments
- Public functions: mandatory
- Private functions: if non-obvious (>5 lines)
- Examples: provide for entry points (scan_repos, run_git)

```rust
/// Scan `workspace_dir` for direct-child git repos, collecting metadata in parallel.
/// Results are sorted alphabetically by repo name.
///
/// # Examples
/// ```ignore
/// let repos = scan_repos(Path::new("."))?;
/// assert!(!repos.is_empty());
/// ```
pub fn scan_repos(workspace_dir: &Path) -> Result<Vec<RepoInfo>> { }
```

## Commit Standards

- **Format:** Conventional commits (feat:, fix:, docs:, refactor:, test:, chore:)
- **Scope:** Optional (e.g., feat(scanner): add parallel metadata)
- **Body:** Explain why, not what
- **Footer:** Reference issues (Closes #123)

Example:
```
feat(ui): add color support for dirty repos

Use yellow indicator for uncommitted changes in table output.
Improves at-a-glance recognition of repos needing attention.

Closes #45
```

## Version Strategy

- **Current:** 0.1.0 (initial release)
- **Semver:** MAJOR.MINOR.PATCH
- **Breaking changes:** require MAJOR bump (not expected for CLI flags)
- **New features:** MINOR bump
- **Bug fixes:** PATCH bump

## Future Considerations

### Refactoring Candidates
- None currently (code is lean and modular)

### Optimization Opportunities
- Parallel batch execution (if network latency allows)
- Caching branch/commit info (if run in tight loops)
- Lazy metadata loading (if scanning becomes bottleneck)

### Extension Points
- Custom Git commands (subcommand plugin system?)
- Repository aliases (group frequently used repos)
- Configuration file (~/.repo/config for workspace root)
