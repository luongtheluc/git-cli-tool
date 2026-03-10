# repo — Polyrepo Manager

Manage multiple Git repositories from a workspace folder. Discover, inspect, and batch-operate across repos with an interactive terminal UI.

## Features

- Auto-discovers all Git repos in workspace (direct children only)
- Interactive multi-select UI for safe batch operations
- Shows branch, last commit, and dirty state per repo
- Parallel metadata scanning (50+ repos in <2 seconds)
- Batch operations: checkout, pull, push, commit, status
- **`repo audit`** — offline git health check across all repos (uncommitted, unpushed, behind); exit code 1 for CI
- **`repo audit-deps`** — npm/yarn vulnerability scanning across repos; shows critical/high/medium/low counts; exit code 1 on vulnerabilities
- **`repo ui`** — full-screen interactive dashboard with keyboard shortcuts for batch ops, status view, and npm audit integration
- Per-repo error handling (one failure doesn't stop the batch)
- ANSI color output with aligned columns
- Works cross-platform (Windows, macOS, Linux)

## Quick Start

### Build from Source

```bash
cargo build --release
# Binary: ./target/release/repo.exe (Windows) or ./target/release/repo (Linux/macOS)
```

### Install to PATH

**Option 1: Using setup installer**

```bash
./target/release/setup.exe  # Windows (creates ~/.repo/bin)
./target/release/setup       # macOS/Linux (creates ~/.repo/bin)
```

Then restart your terminal and run: `repo --help`

**Option 2: Install via cargo**

```bash
cargo install --path .
```

**Option 3: Manual copy**

```bash
# Windows PowerShell
Copy-Item target\release\repo.exe $env:USERPROFILE\.cargo\bin\

# macOS/Linux
cp target/release/repo ~/.cargo/bin/
```

## Quick Examples

Run `repo` from a workspace directory containing multiple git repo folders.

**List all repos:**
```bash
$ repo list
```

**Checkout a branch on selected repos:**
```bash
$ repo checkout develop
```

**Pull, push, commit, or check status:**
```bash
$ repo pull
$ repo push
$ repo commit -m "update dependencies"
$ repo status
```

**Offline audit for issues:**
```bash
$ repo audit
```

**Scan for npm/yarn vulnerabilities:**
```bash
$ repo audit-deps
```

**Interactive TUI dashboard:**
```bash
$ repo ui
```

**Run custom git commands:**
```bash
$ repo git -- log --oneline -5
$ repo git -- stash
```

For detailed usage examples and workflows, see [`docs/tutorial-en.md`](./docs/tutorial-en.md).

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4 | CLI argument parsing (derive macros) |
| `dialoguer` | 0.11 | Interactive multi-select TUI |
| `colored` | 2 | Terminal color output |
| `walkdir` | 2 | Recursive directory traversal |
| `anyhow` | 1 | Error handling with context |
| `rayon` | 1 | Parallel metadata collection |
| `console` | 0.15 | Terminal utilities (dialoguer dependency) |
| `serde_json` | 1 | npm/yarn audit JSON parsing |
| `unicode-width` | 0.1 | Display width calculations for dynamic sizing |
| `unicode-segmentation` | 1.10 | Unicode segmentation for text layout |
| `ratatui` | 0.26 | Terminal UI framework (interactive dashboard) |
| `crossterm` | 0.27 | Terminal manipulation (ratatui dependency) |
| `winreg` | 0.52 | Windows registry access (setup binary only) |

## Project Structure

```
src/
├── main.rs          — Entry point, command dispatch, run_batch()
├── cli.rs           — clap argument parsing (Cli, Commands enums)
├── repo_scanner.rs  — Repository discovery, parallel metadata collection
├── git_runner.rs    — System git wrapper; git & npm/yarn audit operations
├── ui.rs            — Terminal UI (dialoguer multiselect, table printing)
├── text_utils.rs    — Unicode display width utilities for dynamic sizing
├── commit_graph.rs  — Git commit graph parsing and pagination
├── tui/             — Interactive TUI (ratatui): sidebar, status, audit, batch ops
│   ├── mod.rs       — Terminal lifecycle and event loop
│   ├── app.rs       — Application state machine
│   ├── ui.rs        — Render sidebar and panels
│   ├── events.rs    — Keyboard input dispatch
│   ├── batch_ops.rs — Async batch operation execution
│   └── modal.rs     — Modal dialog rendering for user input
└── setup.rs         — Installer binary (PATH registration)
```

## Documentation

Key documentation files in `./docs/`:
- **[`tutorial-en.md`](./docs/tutorial-en.md)** — Detailed usage guide with examples for all commands
- `project-overview-pdr.md` — Vision, requirements, acceptance criteria
- `code-standards.md` — Coding conventions, patterns, best practices
- `codebase-summary.md` — File-by-file breakdown, LOC, responsibilities
- `system-architecture.md` — Module architecture, data flow, design decisions
- `project-roadmap.md` — Current status, planned features, long-term vision
- `tutorial-vi.md` — Vietnamese usage tutorial
- `project-changelog.md` — Version history and release notes
