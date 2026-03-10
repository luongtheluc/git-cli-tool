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
│   └── git_runner.rs (system git + enhanced metadata)
├── ui.rs (presentation)
│   └── repo_scanner.rs (RepoInfo type)
├── git_runner.rs (git operations + npm audit)
│   ├── serde_json (JSON parsing for npm/yarn audit)
│   └── text_utils.rs (display width for formatting)
├── text_utils.rs (Unicode display width utilities)
├── commit_graph.rs (phase 8.3+: graph parsing — used by tui/graph_modal)
│   └── tui/app.rs (CommitNode type)
└── tui/ (interactive full-screen dashboard)
    ├── app.rs (state machine + CommitNode)
    ├── ui.rs (render sidebar + status + npm audit; uses text_utils)
    ├── events.rs (keyboard input dispatch; 'A' for audit, 'N' for npm audit)
    ├── batch_ops.rs (BatchOp enum + async executor; includes NpmAudit variant)
    ├── modal.rs (user input dialogs for commit messages, branches)
    ├── graph_modal.rs (phase 8.3+: modal rendering — uses commit_graph)
    └── git_runner.rs (batch operations)

setup.rs (installer, independent binary)
├── std::env
├── std::fs
├── colored (for output)
└── winreg (Windows-only)
```

**Key:** No circular dependencies. Data flows through RepoInfo struct. commit_graph provides parsing support for TUI modals. TUI runs as separate mode with its own event loop.

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
- **Input:** Repo path, git command arguments, package.json/yarn.lock presence
- **Output:** String (git stdout) or parsed operations (hashes, decorators, subjects, npm audit results)
- **Responsibility:** Execute git commands safely via `git -C`; provide graph data for parsing; run npm/yarn audits
- **Size:** 350 LOC (expanded with phase 2 graph + phase 8 npm audit)
- **Dependencies:** std::process::Command, anyhow, std::sync (for run_shell threading), serde_json
- **Key Design:** All operations use `git -C <path>` to avoid directory changes
- **Phase 2 Additions:**
  - `get_commit_graph()` - fetch raw git log with ASCII graph
  - `git_checkout()`, `git_cherry_pick()`, `git_rebase()`, `git_merge()` - advanced git operations
- **Phase 8.1 (Audit):**
  - `audit_repo()` - offline git health check (uncommitted, unpushed, behind)
- **Phase 8.2 (npm Audit):**
  - `npm_audit()`, `yarn_audit()` - package vulnerability scanning
  - Windows support: `cmd.exe /C` wrapper for .cmd scripts (npm, yarn on Windows)

#### commit_graph.rs (Phase 2)
- **Input:** Raw git log output string (from `git log --graph --all --oneline --decorate`)
- **Output:** `CommitGraph` struct with parsed nodes and pagination
- **Responsibility:** Parse ASCII graph, extract metadata, support pagination for display
- **Size:** 180 LOC
- **Dependencies:** anyhow, crate::tui::app::CommitNode
- **Key Functions:**
  - `parse_git_log_graph() -> Result<CommitGraph>` - main parser
  - `get_page() -> Vec<(String, CommitNode)>` - pagination support
  - Helper parsers: `extract_hash()`, `extract_decorators()`, `extract_subject()`
- **Key Design:**
  - Hand-written parser (no external crates)
  - Defensive parsing with `.unwrap_or_default()`
  - Preserves raw graph line for display
  - Returns CommitNode for easy UI rendering
  - Pagination ready for modal display (phase 3)

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

#### tui/ (5+ modules)
- **Input:** `Vec<RepoInfo>` from scanner; `CommitGraph` from parser (phase 3)
- **Output:** Full-screen terminal UI with batch operations and modals
- **Responsibility:** Lazygit-style TUI — enriched sidebar (checkbox, tag, time), status panel, batch operations, modals, keyboard navigation
- **Size:** ~450+ LOC across 5 files (phase 3 will add graph_modal.rs)
- **Dependencies:** ratatui 0.26, crossterm 0.27, tokio (async), git_runner, commit_graph (phase 3)
- **Key Design:**
  - Alternate screen + raw mode; 100 ms poll loop; panic hook guarantees terminal restore
  - Multi-select checkboxes (Space/a to toggle all)
  - Sidebar enriched: checkbox, dirty file count, branch w/ ahead/behind, tag, relative commit time
  - Batch operations (p=Pull, P=Push, f=Fetch, c=Commit, b=Branch) execute async in thread
  - Input mode for text entry (branch name, commit message)
  - Live progress feedback (✓ on success, ✗ on error)
  - **Phase 3 (In Progress):** Modal system for commit graph visualization with pagination

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

### npm/yarn Audit Phase Data Flow (Phase 8.2)

```
Vec<RepoInfo>
    │
    ├─ for each repo:
    │   ├─ Check if package.json or yarn.lock exists
    │   │
    │   ├─ If yarn.lock:  Run: yarn audit --json
    │   │                Parse JSON via serde_json
    │   │
    │   └─ Elif package.json: Run: npm audit --json
    │                        Parse JSON via serde_json
    │
    ▼
for each repo:
    │ (parallel rayon or sequential)
    ├─ Execute npm/yarn audit
    │
    ├─ Parse JSON output → NpmAuditResult {
    │      critical: usize,
    │      high: usize,
    │      medium: usize,
    │      low: usize,
    │   }
    │
    ├─ Format color-coded table row:
    │   Repo | Critical(red) | High(red) | Medium(yellow) | Low(dim)
    │
    └─ Exit code 1 if critical or high vulns found
```

**Windows Support:** npm and yarn are .cmd scripts on Windows. Use conditional compilation (`#[cfg(windows)]`) to wrap with `cmd.exe /C`.

**Error Handling:** Repos without package.json are skipped; audit errors print "✗ error" in table.

## Data Structures

### RepoInfo Struct (expanded for TUI Dashboard)
```rust
pub struct RepoInfo {
    // Base fields
    pub name: String,                    // "api-service"
    pub path: PathBuf,                   // /workspace/api-service
    pub branch: String,                  // "develop" or "?"
    pub last_commit_hash: String,        // "a12bc3" (7-char short hash)
    pub last_commit_msg: String,         // "fix auth bug"
    pub has_uncommitted_changes: bool,   // true if git status --porcelain not empty

    // TUI Dashboard fields (new)
    pub latest_tag: String,              // "v1.2.3" or "" (fallback)
    pub last_commit_time: SystemTime,    // Timestamp for relative display
    pub changed_file_count: usize,       // Count of unstaged/untracked files
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
    Checkout { branch: String, #[arg(short='b')] create: bool },
    Pull,
    Push,
    Commit { #[arg(short, long)] message: String },
    Status,
    Audit,  // offline git health check
    AuditDeps,  // npm/yarn vulnerability scan
    Git { #[arg(trailing_var_arg)] args: Vec<String> },  // pass-through git args
    Run { script: String, #[arg(short, long, default_value_t = 1)] jobs: usize },
    Ui,  // launches full-screen ratatui TUI
}
```

**Lifetime:** Scope of main(); parsed once.

### BatchOp Enum (TUI Dashboard)
```rust
pub enum BatchOp {
    Pull,
    Push,
    Fetch,
    Commit(String),                      // Message provided via input mode
    Checkout(String),                    // Branch name provided via input
    NpmAudit,                            // Phase 8.2: npm/yarn vulnerability scan
}
```

**Execution:** Async in tokio thread; UI notified of completion (✓ or ✗).

### NpmAuditResult Struct (Phase 8.2)
```rust
pub struct NpmAuditResult {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl NpmAuditResult {
    pub fn is_clean(&self) -> bool { /* all zeros */ }
    pub fn has_critical_or_high(&self) -> bool { /* critical > 0 || high > 0 */ }
}
```

**Sourced from:** `npm audit --json` or `yarn audit --json` JSON parsing via serde_json.

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

## Control Flow — Example: `repo ui` (TUI Dashboard)

1. **User:** `repo ui`
2. **cli.rs:** Parse → `Commands::Ui`
3. **main.rs:** Match Ui arm → `tui::run(repos)`
4. **repo_scanner.rs:** `scan_repos(cwd)` (same as above)
5. **tui/mod.rs:** Initialize terminal
   - Enter raw mode + alternate screen
   - Install panic hook (restores terminal on crash)
   - Spawn event loop (100ms polling)
6. **tui/app.rs:** Render sidebar + status panel
   - Display repos with checkboxes, branch, tag, relative time
   - Highlight selected row (blue background)
   - Parse git status porcelain for detailed file view
7. **tui/events.rs:** Keyboard dispatch (normal vs. input mode)
   - ↑↓/jk: navigate
   - Space/a: toggle selection
   - p/P/f/c/b: Queue batch operation
   - t: open input mode for text entry
   - q: quit
8. **tui/batch_ops.rs:** Execute operation async in tokio thread
   - User sees live progress (✓/✗) without blocking UI
   - Results logged per-repo
9. **tui/ui.rs:** Re-render with updated status
10. **Process exits:** Terminal restored, return 0

## Control Flow — Example: Graph Modal (Phase 3 — In Progress)

1. **User (in TUI):** Press `g` to open commit graph modal
2. **tui/events.rs:** Match input → signal modal open
3. **tui/app.rs:** Set modal state (open=true, page=0)
4. **git_runner.rs:** `get_commit_graph(&repo_path, limit=50)`
   - Execute: `git log --graph --all --oneline --decorate --color=never -n 50`
   - Return raw ASCII graph string
5. **commit_graph.rs:** `parse_git_log_graph(output) -> Result<CommitGraph>`
   - Parse ASCII graph lines
   - Extract commit hashes (7-40 hex chars)
   - Extract branch decorators: `(HEAD -> main, origin/main, tag: v1.0)`
   - Extract subject lines (first text after hash/decorators)
   - Return Vec<CommitNode> + raw lines for display
6. **tui/graph_modal.rs:** Render modal (phase 3)
   - Display graph lines in scrollable view
   - Support pagination: `get_page(graph, page, page_size)`
   - Show current page / total pages
   - Allow user: ↑↓/jk to scroll, j/k to paginate, p: checkout, c: cherry-pick, etc.
7. **Process exits modal:** Restore main TUI view, continue normal operation

## Graph Visualization Pipeline (Phase 2-3)

```
Repository
    │
    ├─ git log --graph --all --oneline --decorate
    │   (raw ASCII output with branch refs)
    │
    ▼ commit_graph::get_commit_graph()
    │
    ▼ (raw string)
    │
    ├─ commit_graph::parse_git_log_graph()
    │   │ (parse_git_log_graph)
    │   │
    │   ├─ extract_hash() → commit hash (7-40 hex)
    │   ├─ extract_decorators() → Vec<String> (HEAD, origin/*, tags)
    │   └─ extract_subject() → commit message
    │
    ▼
CommitGraph {
    nodes: Vec<CommitNode>,    // hash, subject, decorators
    graph_lines: Vec<String>,  // raw ASCII for display
}
    │
    ├─ commit_graph::get_page(page, page_size)
    │   │
    │   ▼
    │ Vec<(String, CommitNode)>  // graph_line + node data
    │
    ▼ tui/graph_modal.rs (phase 3)
    │
    ▼ Display in modal
    │
    ├─ Render graph lines as-is
    ├─ Highlight commits w/ decorators
    └─ Support operations: checkout, cherry-pick, rebase, merge
```

**Key Design Decisions:**
- Graph parser is decoupled from git operations (separate module)
- CommitNode is self-contained; suitable for UI rendering
- Pagination support built-in for large repos (1000+ commits)
- Raw graph lines preserved exactly as git outputs them (no re-rendering)

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
| `checkout(branch, create)` | `["checkout", branch]` or `["checkout", "-b", branch]` | success msg | error |
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

## TUI Dashboard Architecture

### Overview
The `repo ui` command launches an interactive full-screen dashboard with three core features:

1. **Enriched Sidebar** — Multi-select repo list with metadata badges (tag, commit time, changed file count)
2. **Status Panel** — Detailed git status porcelain display with color-coded staging
3. **Batch Operations** — Execute operations (Pull, Push, Fetch, Commit, Checkout) on selected repos async

### Input Modes
- **Normal Mode:** Navigation (↑↓/jk), select toggle (Space/a), batch op keys (p/P/f/c/b), quit (q)
- **Input Mode:** Text entry for branch names or commit messages with prompt overlay

### Batch Operations Execution
```
User presses 'p' (pull)
    │
    ▼
Event captured in events.rs
    │
    ▼
batch_ops::execute() spawned in tokio thread
    │
    ├─ for each selected repo:
    │   ├─ Call git_runner::pull()
    │   ├─ Mark ✓ on success
    │   └─ Mark ✗ on error
    │
    ▼
UI re-renders live with progress feedback
    │
    ▼
User sees operation status without blocking
```

### Sidebar Enrichment (New Fields)
Each repo row displays:
```
[X] repo-name                 # checkbox + name
    branch-name (+2/-1)       # branch with ahead/behind
    latest-tag                # tag badge
    2m ago, 3 files changed   # relative time + changed count
```

### Error Handling
- **Batch op errors:** Logged per-repo, marked ✗, continue to next
- **Input validation:** Simple bounds checking; git handles invalid branch/message
- **Terminal restore:** Panic hook guarantees shell safety even on crash

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
