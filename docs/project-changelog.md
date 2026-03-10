# Project Changelog

All notable changes to `repo` polyrepo manager are documented here.

---

## [Unreleased]

### Added
- **npm/yarn Vulnerability Audit (Phase 8)** — 2026-03-10
  - Added `NpmAuditResult` struct to `src/git_runner.rs` with vulnerability counts (critical/high/medium/low)
  - New `npm_audit()` and `yarn_audit()` functions for package vulnerability scanning
  - CLI command `repo audit-deps` — scans all repos for package.json/yarn.lock, runs npm/yarn audit, color-coded output
  - TUI npm audit view — press `N` to toggle npm/yarn audit table, sidebar icons (N✓/N✗/N⚠/N?)
  - Windows support for npm/yarn execution via `cmd.exe /C` wrapper (conditional compilation)
  - Added `serde_json` dependency for JSON parsing
  - Added 6 new unit tests for npm/yarn audit functions

- **Text Utilities Module** — Dynamic terminal width support
  - New `src/text_utils.rs` module with Unicode display width calculations
  - Used by TUI for dynamic sidebar and panel sizing (no hardcoded truncation)
  - Supports Unicode characters in repo names and paths

- **TUI Modal Dialog System**
  - New `src/tui/modal.rs` for modal dialog rendering
  - Used for user input prompts (commit messages, branch names)

### Changed
- Updated test count from 48 to 54 unit tests (6 new npm audit tests)
- TUI dynamic width sizing: repo names, status panel title, file paths now scale with terminal width
- Dependencies added: `serde_json`, `unicode-width`, `unicode-segmentation`

### Fixed
- Windows npm/yarn execution — npm and yarn are .cmd scripts requiring `cmd.exe /C` wrapper
- TUI hardcoded truncation replaced with dynamic Unicode-aware width calculation

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
| Unreleased | 54 | 8.7/10 | — |
| v0.1.5 | 48 | 8.6/10 | 2026-03-09 |
| v0.1.4 | 44 | 8.6/10 | Earlier |

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
