# System Architecture

## Architecture Overview

repo is a single-process, interactive CLI application with three distinct phases:

1. **Discovery Phase** — Scan workspace directory for Git repositories (parallel)
2. **Selection Phase** — Interactive TUI for user to select repos (single-threaded)
3. **Execution Phase** — Run batch operation on selected repos (sequential)

## High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│ User invokes: repo <command> [args]                │
└────────────┬────────────────────────────────────────┘
             │
             ▼
    ┌────────────────────┐
    │  cli.rs            │
    │  Parse args        │
    │  via clap 4        │
    └────────┬───────────┘
             │
      ┌──────▼──────────────────────────────────────┐
      │ main.rs — command dispatch                  │
      │                                             │
      │  List → scan + print_table                 │
      │  Other → scan + select + run_batch         │
      └──────┬──────────────────────────────────────┘
             │
    ┌────────▼─────────────────────────────────────┐
    │ DISCOVERY PHASE                              │
    │ repo_scanner.rs::scan_repos()                │
    │                                              │
    │ WalkDir(depth=2) → Vec<PathBuf>             │
    │       │                                      │
    │       ▼ par_iter (rayon)                     │
    │ build_repo_info() × n                        │
    │   ├─ git_runner::get_branch()                │
    │   ├─ git_runner::get_last_commit()           │
    │   └─ git_runner::has_changes()               │
    │       │                                      │
    │       ▼                                      │
    │ Vec<RepoInfo> sorted alphabetically          │
    └────────┬────────────────────────────────────┘
             │
    ┌────────▼─────────────────────────────────────┐
    │ SELECTION PHASE (non-List commands)          │
    │ ui.rs::select_repos()                        │
    │                                              │
    │ dialoguer::MultiSelect → Vec<usize>         │
    │ (user selects with space/enter)              │
    └────────┬────────────────────────────────────┘
             │
    ┌────────▼─────────────────────────────────────┐
    │ EXECUTION PHASE                              │
    │ main.rs::run_batch()                         │
    │                                              │
    │ for each selected index:                     │
    │   ├─ Print green header with repo name       │
    │   ├─ git_runner::operation(repo_path)        │
    │   └─ Print result or error (red)             │
    └────────────────────────────────────────────────┘
```

## Module Architecture

### Dependency Graph

```
main.rs (entry point, orchestration)
├── cli.rs (argument parsing)
├── repo_scanner.rs (discovery)
│   └── git_runner.rs (system git)
├── ui.rs (presentation)
│   └── repo_scanner.rs (RepoInfo type)
└── git_runner.rs (git operations)

setup.rs (installer, independent binary)
├── std::env
├── std::fs
├── colored (for output)
└── winreg (Windows-only)
```

**Key:** No circular dependencies. Data flows through RepoInfo struct only.

### Module Responsibilities

#### cli.rs
- **Input:** Command-line arguments (ARGV)
- **Output:** `Cli` struct with parsed command
- **Responsibility:** Define schema and parse
- **Size:** 37 LOC
- **Dependencies:** clap 4

#### repo_scanner.rs
- **Input:** Workspace directory path
- **Output:** Vec<RepoInfo> (name, path, branch, commit, dirty flag)
- **Responsibility:** Discover repos, collect metadata in parallel, sort
- **Size:** 129 LOC
- **Dependencies:** walkdir, rayon, git_runner
- **Parallelization:** par_iter on repo paths; no shared state

#### git_runner.rs
- **Input:** Repo path, git command arguments
- **Output:** String (git stdout)
- **Responsibility:** Execute git commands safely via `git -C`
- **Size:** 147 LOC
- **Dependencies:** std::process::Command, anyhow
- **Key Design:** All operations use `git -C <path>` to avoid directory changes

#### ui.rs
- **Input:** Vec<RepoInfo> from scanner
- **Output:** Vec<usize> (indices of selected repos) or table output
- **Responsibility:** Interactive selection UI, column-aligned table display
- **Size:** 82 LOC
- **Dependencies:** dialoguer, colored
- **Key Design:** Pad before colorizing to preserve column widths; return indices not references

#### main.rs
- **Input:** Parsed CLI args + current working directory
- **Output:** Process exit status + printed output
- **Responsibility:** Orchestrate discovery→selection→execution; error handling
- **Size:** 87 LOC
- **Key Function:** `run_batch<F>(repos, selected, operation)` — generic batch executor

#### setup.rs
- **Input:** Environment (PATH, USERPROFILE/HOME)
- **Output:** Binary installed to ~/.repo/bin/; PATH updated
- **Responsibility:** Install repo binary to system; update PATH
- **Size:** 188 LOC
- **Platform-Specific:** Windows (winreg), Unix (shell profile)

## Data Flow

### Discovery Phase Data Flow

```
Workspace Dir
    │
    ▼
WalkDir::new(dir)
    │ min_depth(2), max_depth(2)
    ▼
Filter: only ".git" folders
    │
    ▼
Vec<PathBuf> (parent dirs of .git)
    │
    ▼
par_iter for each repo path:
    ├─ git log -1 --format=%h\t%s    → (hash, message)
    ├─ git rev-parse --abbrev-ref HEAD → branch
    └─ git status --porcelain         → has_changes
    │
    ▼
RepoInfo {
    name: String,
    path: PathBuf,
    branch: String,
    last_commit_hash: String,
    last_commit_msg: String,
    has_uncommitted_changes: bool,
}
    │
    ▼
Sort by name
    │
    ▼
Vec<RepoInfo>
```

**Error Handling:** If git fails on a repo, `.filter_map().ok()` silently excludes it.

### Selection Phase Data Flow

```
Vec<RepoInfo>
    │
    ▼
format_repo_line() for each repo
    │ (plain text, no ANSI codes)
    ▼
Vec<String> (formatted lines)
    │
    ▼
dialoguer::MultiSelect
    │ (user: space to toggle, enter to confirm)
    ▼
Vec<usize> (indices of selected repos)
```

### Execution Phase Data Flow

```
(Vec<RepoInfo>, Vec<usize>, operation closure)
    │
    ▼
for each index in Vec<usize>:
    │
    ├─ Print header: "=== repo_name ==="  (green, bold)
    │
    ├─ Call operation(&repos[idx].path)
    │
    ├─ On Ok(output):
    │   └─ Print output or "Done."
    │
    └─ On Err(e):
        └─ Print "Error: {e}" (red)
```

**Key:** Errors are caught, printed in red, and execution continues.

## Data Structures

### RepoInfo Struct
```rust
pub struct RepoInfo {
    pub name: String,                    // "api-service"
    pub path: PathBuf,                   // /workspace/api-service
    pub branch: String,                  // "develop" or "?"
    pub last_commit_hash: String,        // "a12bc3" (7-char short hash)
    pub last_commit_msg: String,         // "fix auth bug"
    pub has_uncommitted_changes: bool,   // true if git status --porcelain not empty
}
```

**Lifetime:** Owned; no references. Safe to pass around and store.

### Cli Struct (clap)
```rust
pub struct Cli {
    pub command: Commands,
}

pub enum Commands {
    List,
    Checkout { branch: String },
    Pull,
    Push,
    Commit { #[arg(short, long)] message: String },
    Status,
}
```

**Lifetime:** Scope of main(); parsed once.

## Control Flow — Example: `repo pull`

1. **User:** `repo pull`
2. **cli.rs:** Parse → `Commands::Pull`
3. **main.rs:** Match Pull arm
4. **repo_scanner.rs:** `scan_repos(cwd)`
   - WalkDir depth 2
   - par_iter: get branch, commit, dirty for each
   - Return Vec<RepoInfo> sorted
5. **ui.rs:** `select_repos(repos)`
   - Print multi-select prompt
   - User selects with space/enter
   - Return Vec<usize> indices
6. **main.rs:** `run_batch(repos, selected, git_runner::pull)`
   - for idx in selected:
     - Print "=== {name} ===" (green)
     - Call `git_runner::pull(&repos[idx].path)`
     - On success: print "Already up to date" or pull output
     - On error: print error in red, continue
7. **Process exits:** Return 0

## Parallelization Strategy

### Phase 1: Discovery (Parallel)
- **Why:** Git calls (network or I/O bound) on different repos are independent
- **How:** rayon `par_iter` on Vec<PathBuf>
- **Benefit:** 50 repos in ~2 seconds vs. ~100 seconds sequential
- **Safety:** No shared state; each repo produces independent RepoInfo

### Phase 2: Selection (Single-threaded)
- **Why:** dialoguer is single-threaded; terminal I/O not parallelizable
- **How:** Simple iteration and string formatting

### Phase 3: Execution (Sequential)
- **Why:** User expects synchronous feedback per repo; network-bound anyway
- **How:** for loop with error handling
- **Trade-off:** Could parallelize, but reduces per-repo visibility and complicates error reporting

## Error Handling Strategy

### Scan Phase
- **Per-repo errors:** Logged as fallback values ("?", "no commits") but repo included
- **Catastrophic errors:** Propagate to main (disk full, cwd inaccessible)

```rust
let branch = git_runner::get_branch(&path)
    .unwrap_or_else(|_| "?".to_string());
```

Result: Incomplete repos still appear in list; user sees them and can investigate.

### Selection Phase
- **Empty repo list:** Print message and return empty selection
- **User cancels:** Return empty Vec<usize>

### Execution Phase
- **Per-repo operation fails:** Print error in red; continue to next repo
- **All repos fail:** Process exits with exit code 1 (TBD; currently exits 0)

```rust
match operation(&repo.path) {
    Ok(output) => println!("{}", output),
    Err(e) => eprintln!("{}", format!("Error: {e}").red()),
}
```

## Git Command Abstraction

### run_git() Pattern
All git operations use:
```rust
Command::new("git")
    .arg("-C").arg(repo_path)
    .args(args)
    .output()
```

**Rationale:**
- No directory changes needed
- Each call is isolated
- Easier to stub for testing

### Git Command Variants

| Function | Args | Output | Fallback |
|----------|------|--------|----------|
| `get_branch()` | `["rev-parse", "--abbrev-ref", "HEAD"]` | "main" | "?" |
| `get_last_commit()` | `["log", "-1", "--format=%h\t%s"]` | ("a12bc3", "msg") | ("???????", "no commits") |
| `has_changes()` | `["status", "--porcelain"]` | bool | false |
| `checkout(branch)` | `["checkout", branch]` | success msg | error |
| `pull()` | `["pull"]` | pull output | error |
| `push()` | `["push"]` | push output | error |
| `commit(msg)` | `["add", "-A"]` then `["commit", "-m", msg]` | commit output | error |
| `status()` | `["status"]` | status output | error |

## Performance Characteristics

### Time Complexity
- **Scan:** O(n) where n = repos; parallel execution: ~40ms/repo
- **Select:** O(n) dialoguer render; <100ms for 50 repos
- **Batch:** O(m) where m = selected repos; network-dependent
- **Total (50 repos):** ~5 seconds (2s scan + 1s select + 2s network)

### Space Complexity
- **Vec<RepoInfo>:** O(n); ~1KB per repo
- **50 repos:** ~50KB metadata in memory
- **No caching between commands:** Fresh scan per invocation

### Bottlenecks
1. **Network latency** (pull/push) — dominant cost
2. **Disk I/O** (WalkDir traversal) — secondary
3. **Parallel metadata collection** — hidden by above

## Security Considerations

### Input Validation
- **Branch name:** Not sanitized; passed to `git checkout`
  - Risk: Shell injection (e.g., `repo checkout "a; rm -rf ."`)
  - Mitigation: Use array-arg pattern; never shell interpolation
- **Commit message:** Not sanitized; passed to `git commit -m`
  - Risk: Message quoting issues; accepted as-is
  - Mitigation: Use `-m` flag (not stdin)
- **Repo paths:** Resolved from WalkDir; safe
  - No user-supplied paths accepted

### System Interaction
- **Git binary:** Assumed in PATH; error if missing
- **Filesystem:** Scans workspace only; no escaping allowed
- **Registry (Windows):** winreg crate handles escaping; safe

### No Dangerous Operations
- Never `git push --force`
- Never `git reset --hard`
- Never `git clean -fd`
- Write-safe operations only

## Extensibility Points

### Adding New Commands
1. Add variant to `Commands` enum in cli.rs
2. Add match arm in main.rs
3. Implement git_runner function or chain existing ones
4. Test via unit tests + manual

Example: `repo fetch`
```rust
// cli.rs
Fetch,

// main.rs
Commands::Fetch => {
    let repos = repo_scanner::scan_repos(&cwd)?;
    let selected = ui::select_repos(&repos)?;
    run_batch(&repos, &selected, git_runner::fetch);
}

// git_runner.rs
pub fn fetch(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["fetch"])
}
```

### Adding Custom Output
Currently: `print_repo_table()` and `run_batch()` handle output.

To add custom formatting:
1. Create new function in ui.rs
2. Call from main.rs before/after operation
3. Example: JSON output for scripting

### Configuration File
Future: `~/.repo/config` for:
- Default workspace directory
- Repository aliases
- Custom batch operation scripts
- Environment overrides

## Testing Architecture

### Unit Test Scope
- **repo_scanner:** Discovery, sorting, error handling
- **git_runner:** Git invocation, output parsing, error cases
- **ui:** (currently untested; would require mocking dialoguer)

### Test Isolation
- `tempfile::TempDir` for filesystem
- `git init` in temp dir for git operations
- No system state modified

### CI/CD Integration
Currently: Manual testing. Future:
- GitHub Actions: build + test on Windows/macOS/Linux
- Linting: `cargo clippy`
- Formatting: `cargo fmt --check`

## Deployment Architecture

### Build Artifacts
```
cargo build --release
├── target/release/repo.exe (or repo)       → main binary
└── target/release/setup.exe (or setup)     → installer
```

### Installation Flow
User runs `setup.exe`:
1. Locate `repo.exe` in same directory
2. Create `~/.repo/bin/`
3. Copy `repo.exe` to `~/.repo/bin/repo.exe`
4. Update PATH:
   - **Windows:** Add entry to HKCU\Environment\Path (registry)
   - **Unix:** Append export to ~/.profile

### Post-Install
User can invoke: `repo --help` from anywhere

## Future Architecture Improvements

### Potential Enhancements
1. **Configuration file** — reduce command-line args
2. **Plugin system** — user-defined batch operations
3. **Caching** — skip scanning if config unchanged
4. **Logging** — audit trail of batch operations
5. **Undo capability** — simple commit history per repo
6. **Parallel execution** — optional `--parallel` flag for push/pull
7. **Filtering** — branch filter, status filter before selection
