# repo — Polyrepo Manager

Manage multiple Git repositories from a workspace folder. Discover, inspect, and batch-operate across repos with an interactive terminal UI.

## Features

- Auto-discovers all Git repos in workspace (direct children only)
- Interactive multi-select UI for safe batch operations
- Shows branch, last commit, and dirty state per repo
- Parallel metadata scanning (50+ repos in <2 seconds)
- Batch operations: checkout, pull, push, commit, status
- **`repo audit`** — offline git health check across all repos (uncommitted, unpushed, behind); exit code 1 for CI
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

## Usage

Run `repo` from a workspace directory containing multiple git repo folders:

```
workspace/
├── api-service/       ← contains .git
├── auth-service/      ← contains .git
├── frontend/          ← contains .git
└── worker/            ← contains .git
```

### Commands

**List all repos:**

```
$ repo list

REPO              BRANCH        LAST COMMIT
──────────────────────────────────────────────────────────────
api-service       develop       a12bc3 fix auth bug
auth-service      main          7a91de update deps          *
frontend          feature/ui    0ab221 add layout
worker            develop       c2f991 fix queue
```

> `*` indicates repos with uncommitted changes.

---

**Checkout a branch across selected repos:**

```
$ repo checkout develop

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] auth-service     main         7a91de update deps
  [ ] frontend         feature/ui   0ab221 add layout
  [x] worker           develop      c2f991 fix queue

=== api-service ===
Already on 'develop'

=== auth-service ===
Switched to branch 'develop'

=== worker ===
Already on 'develop'
```

---

**Pull in selected repos:**

```
$ repo pull
```

**Push in selected repos:**

```
$ repo push
```

**Stage all + commit in selected repos:**

```
$ repo commit -m "update api"
```

**Show status in selected repos:**

```
$ repo status
```

**Audit git health across all repos (offline, no fetch):**

```
$ repo audit

  Repo            Branch    Uncommitted    Unpushed  Behind  Status
  ─────────────────────────────────────────────────────────────────────
  api-service     main      ✗ 3 files      ↑ 2       —       ⚠ warning
  frontend        feature   ✗ 1 file       —         —       ⚠ warning
  shared-lib      main      ✓              ✓         ↓ 1     ↓ behind
  infra           main      ✓              ✓         —       ✓ clean
  ─────────────────────────────────────────────────────────────────────
  4 repos | 2 warning(s) | 1 behind | 1 clean
```

Exit code 0 if all clean, exit code 1 if any issues (CI-friendly).

**Run any git command across selected repos:**

```
$ repo git -- log --oneline -5
$ repo git -- stash
$ repo git -- diff --stat
```

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
| `winreg` | 0.52 | Windows registry access (setup binary only) |

## Project Structure

```
src/
├── main.rs          — Entry point, command dispatch, run_batch(), print_audit_table()
├── cli.rs           — clap argument parsing (Cli, Commands enums)
├── repo_scanner.rs  — Repository discovery, parallel metadata collection
├── git_runner.rs    — System git wrapper; AuditResult + audit_repo()
├── ui.rs            — Terminal UI (dialoguer multiselect, table printing)
├── tui/             — Interactive TUI (ratatui): sidebar, audit view, batch ops
└── setup.rs         — Installer binary (PATH registration)
```

## Documentation

Key documentation files in `./docs/`:
- `project-overview-pdr.md` — Vision, requirements, acceptance criteria
- `code-standards.md` — Coding conventions, patterns, best practices
- `codebase-summary.md` — File-by-file breakdown, LOC, responsibilities
- `system-architecture.md` — Module architecture, data flow, design decisions
- `project-roadmap.md` — Current status, planned features, long-term vision
- `tutorial-vi.md` — Vietnamese usage tutorial
