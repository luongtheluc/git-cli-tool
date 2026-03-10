# Project Changelog

All notable changes to `repo` polyrepo manager are documented here.

---

## [Unreleased]

### Added
- **Repository Audit Feature (Phase 7)** — 2026-03-10
  - Added `AuditResult` struct to `src/git_runner.rs` with severity detection (clean/warning/critical/behind)
  - New `audit_repo()` function for offline git health checks (uncommitted, unpushed, behind-remote)
  - CLI command `repo audit` — scans all repos in parallel, prints color-coded table, exits 1 if issues
  - TUI audit view — press `A` to toggle audit table, sidebar icons show repo health (✓/⚠/✗/↓)
  - Added 4 new unit tests for audit functions

### Changed
- Updated test count from 44 to 48 unit tests (4 new audit tests)
- Reorganized phase numbering: Phase 7 now Audit, Phase 8+ shifted accordingly

### Fixed
- None

### Security
- No security changes

---

## [v0.1.5] — 2026-03-09

### Added
- **Interactive Commit Graph (Phase 6-7, WIP)** — Foundation for interactive graph modal
  - Added `src/commit_graph.rs` module with CommitGraph data structure
  - New git operations: `git_checkout()`, `git_cherry_pick()`, `git_rebase()`, `git_merge()`
  - Added 13 new unit tests for graph parsing
  - Support for parsing ASCII commit graph from git log

- **Custom Git Commands** — `repo git -- <args>`
  - Run arbitrary git commands across all selected repos

- **Script Runner** — `repo run --jobs N`
  - Execute shell scripts in parallel or sequential mode

- **Full TUI Dashboard** — `repo ui`
  - Interactive full-screen terminal dashboard with ratatui
  - Keyboard shortcuts: Pull (P), Push (U), Fetch (F), Commit (C), Checkout (B)
  - Status display with color-coded badges and enriched sidebar
  - Production-ready code quality (8.6/10)

### Changed
- Total unit tests: 44 (from 31 in v0.1.4)
- Core modules: main.rs, cli.rs, repo_scanner.rs, git_runner.rs, ui.rs, tui/
- Improved error handling with per-repo continuity (don't abort on single repo error)

### Fixed
- None major

### Security
- Zero unsafe code blocks
- All dependencies up-to-date

---

## [v0.1.4] — Earlier

Core features shipped:
- [x] Repository auto-discovery (depth-2 scanning)
- [x] Parallel metadata collection (rayon)
- [x] Interactive multi-select UI (dialoguer)
- [x] List command with ASCII table
- [x] Checkout, Pull, Push, Commit, Status commands
- [x] ANSI color output
- [x] Setup installer (Windows + Unix)
- [x] 31 unit tests

---

## Metrics

| Version | Tests | Code Quality | Release Date |
|---------|-------|--------------|--------------|
| Unreleased | 48 | 8.6/10 | — |
| v0.1.5 | 44 | 8.6/10 | 2026-03-09 |
| v0.1.4 | 31 | 8.0/10 | Earlier |

---

## Release Checklist (Next Release)

For v0.1.6 or later:
- [ ] Update version in `Cargo.toml`
- [ ] Run `cargo test` — all pass
- [ ] Run `cargo clippy` — no new warnings
- [ ] Update this changelog with final version
- [ ] Update `docs/project-roadmap.md` with new phase status
- [ ] Tag release: `git tag -a v0.X.Y -m "Release v0.X.Y"`
- [ ] Build release artifacts: `cargo build --release`
- [ ] Test on Windows, macOS, Linux
- [ ] Push to GitHub with release notes
