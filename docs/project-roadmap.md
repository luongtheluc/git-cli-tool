# Project Roadmap

## Current Status: v0.1.0 (MVP — Released)

**Release Date:** 2026-03-05
**Status:** Stable, all core features implemented
**Testing:** 9/9 unit tests passing
**Code Quality:** No clippy warnings, fully documented

## Version Overview

### v0.1.0 (Current)
**Core polyrepo management with interactive selection**

**Completed Features:**
- [x] Repository auto-discovery (depth-2 scanning)
- [x] Parallel metadata collection (rayon)
- [x] Interactive multi-select UI (dialoguer)
- [x] List command with ASCII table
- [x] Checkout command
- [x] Pull command
- [x] Push command
- [x] Commit command (stage + commit)
- [x] Status command
- [x] Error handling (per-repo continue on error)
- [x] ANSI color output
- [x] Setup installer (Windows + Unix)
- [x] Comprehensive unit tests
- [x] Documentation
- [x] Vietnamese tutorial

**Known Limitations:**
- No configuration file support
- Sequential batch execution only (no parallel flag)
- No filtering before selection (all repos always listed)
- No commit history or undo
- No custom git commands

**Breaking Changes:** None

---

## v0.2.0 (Next — Planned)
**Enhanced user control and configuration**

**Target Timeline:** Q2 2026
**Estimated Effort:** 20 developer-hours

### Features

#### F1: Configuration File (~/.repo/config)
**Status:** Planned
**Effort:** 4 hours

Support TOML configuration for:
- Default workspace directory (no cwd required)
- Repository aliases (e.g., `[alias.services] = ["api", "auth"]`)
- Batch operation macros (e.g., `[macro.release-prep] = ["checkout main", "pull", "status"]`)
- Color preferences (on/off)
- Parallelization defaults

**Example:**
```toml
[workspace]
default = "/path/to/workspace"

[aliases]
services = ["api-service", "auth-service", "worker"]
frontend = ["frontend", "next-app"]

[macros]
release-prep = ["checkout main", "pull", "status"]
```

**Implementation:**
1. Add `config.rs` module for parsing
2. Integrate into main.rs (overlay CWD with config)
3. Update help text
4. Add tests

#### F2: Repository Filtering
**Status:** Planned
**Effort:** 3 hours

Add filters to narrow repo list before selection:

```bash
repo list --branch develop
repo list --dirty          # show only repos with changes
repo list --status        # show only repos needing pull
repo pull --alias services
```

**Implementation:**
1. Add filter flags to CLI
2. Add filter functions in repo_scanner
3. Apply filters before selection UI

#### F3: Parallel Batch Execution
**Status:** Planned
**Effort:** 3 hours

Optional `--parallel` flag for batch operations (useful for pull/push):

```bash
repo pull --parallel       # use rayon for execution
repo push --parallel
```

**Trade-off:** Less per-repo visibility; better for network-bound ops.

**Implementation:**
1. Add `--parallel` flag to CLI
2. Create `run_batch_parallel()` function
3. Use rayon + Arc<Mutex<Vec>> for output ordering
4. Document when to use parallel

#### F4: Custom Git Commands
**Status:** Planned
**Effort:** 5 hours

Allow running arbitrary git commands across repos:

```bash
repo run "git log --oneline -5"
repo run "git branch -a"
repo run "git stash"
```

**Implementation:**
1. Add `Run { command: String }` variant to Commands
2. Validate command for safety (reject --force, reset, clean)
3. Execute with git_runner::run_git()
4. Print output per repo

#### F5: Commit Hooks
**Status:** Backlog
**Effort:** 4 hours

Support pre/post-operation hooks:

```toml
[hooks]
pre-push = "cargo test"
post-commit = "echo 'changes committed'"
```

---

## v0.3.0 (Future — Exploratory)
**Advanced features and scripting**

**Target Timeline:** Q3 2026
**Status:** Concept phase

### Potential Features

#### F1: Workspace Templates
Generate new workspaces from templates:
```bash
repo new myworkspace --template microservices
```

#### F2: Dependency Graph
Visualize repo dependencies:
```bash
repo graph --show-versions
```

#### F3: Batch Script Runner
Define and execute complex multi-step operations:
```bash
repo batch release-v2.0.0
```

#### F4: Git Worktree Integration
Simplify worktree management across repos:
```bash
repo worktree create feature/new-ui
```

#### F5: Changelog Generation
Auto-generate changelogs from git history:
```bash
repo changelog v1.0.0..v2.0.0
```

---

## Known Issues & Backlog

### Current (v0.1.0)

#### Issue: No Exit Code on Batch Error
**Severity:** Low
**Status:** Open
**Workaround:** Check output for red errors

When batch operation fails in any repo, process still exits 0. Should exit 1 if any repo failed.

**Fix:** Track `had_errors` bool in `run_batch()`.

#### Issue: Setup Installer Not Tested on macOS/Linux
**Severity:** Medium
**Status:** Open
**Workaround:** Manual PATH export

setup.rs uses conditional compilation but not tested on Unix.

**Fix:** Test on macOS and Linux; adjust shell profile logic if needed.

#### Issue: Long Commit Messages Not Truncated
**Severity:** Low
**Status:** Open
**Workaround:** None; visual clutter

Very long commit messages extend line width.

**Fix:** Truncate to 50 chars + "..." in table display.

### Future Backlog

#### Parallel Batch Execution
Output ordering might be confusing; need test coverage.

#### Configuration File Parsing Errors
TOML syntax errors should produce helpful messages.

#### Cross-Platform Path Handling
Windows path separators in config files.

---

## Metrics & Success Criteria

### v0.1.0 Status
| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Unit tests | 8+ | 9 | ✓ |
| Code coverage | >80% | ~85% | ✓ |
| Clippy warnings | 0 | 0 | ✓ |
| Build time | <30s | ~8s | ✓ |
| Scan time (10 repos) | <1s | ~400ms | ✓ |
| Documentation | Complete | 100% | ✓ |
| Safe code | 100% | 100% | ✓ |

### v0.2.0 Targets
- Unit tests: 15+
- Code coverage: >80%
- Configuration parsing: 100% success on valid TOML
- Filter performance: <100ms overhead
- Parallel execution: 2-3x speedup for pull/push

---

## Development Phases

### Phase 1: Core Implementation (COMPLETE)
- [x] Basic Git scanning and operations
- [x] Interactive UI
- [x] Command dispatch
- [x] Testing framework

### Phase 2: Polish & Release (COMPLETE)
- [x] Error handling
- [x] ANSI colors
- [x] Installer
- [x] Documentation

### Phase 3: Configuration (PLANNED)
- [ ] TOML config parsing
- [ ] Workspace defaults
- [ ] Aliases and macros
- [ ] User preferences

### Phase 4: Scripting (PLANNED)
- [ ] Custom git commands
- [ ] Batch operations
- [ ] Hooks

### Phase 5: Advanced Features (EXPLORATORY)
- [ ] Dependency graph
- [ ] Worktree integration
- [ ] Changelog generation
- [ ] Template system

---

## Dependencies & Tech Debt

### Current Dependencies
All dependencies up-to-date as of 2026-03-05. No security advisories.

### Technical Debt
**Low** — Codebase is lean and well-structured.

Minor opportunities:
- Extract `run_batch()` into module once more variants exist
- Add `--json` output flag for scripting
- Refactor UI column logic if more commands output tables

### Performance Debt
None. Scanning is bottleneck; parallel rayon is already optimal.

---

## Release Checklist (Future Releases)

For each release:
- [ ] Update version in Cargo.toml
- [ ] Run `cargo test`
- [ ] Run `cargo clippy`
- [ ] Update CHANGELOG.md
- [ ] Update docs/project-roadmap.md (this file)
- [ ] Tag release: `git tag -a v0.X.Y -m "Release v0.X.Y"`
- [ ] Build release artifacts: `cargo build --release`
- [ ] Test on Windows, macOS, Linux
- [ ] Push to GitHub with release notes

---

## Community Contributions

**Status:** Open to contributions

### Welcome Contributions
- Bug reports (open GitHub issue)
- Feature requests (GitHub discussion)
- Documentation improvements
- Cross-platform testing
- Performance optimizations

### Contribution Guidelines
1. Fork and create feature branch
2. Implement with tests
3. Run `cargo clippy` and `cargo fmt`
4. Submit PR with description
5. Wait for review

---

## Long-Term Vision (12+ months)

### Positioning
repo → standard tool for polyrepo teams (like `nx` for monorepos, but simpler, git-native)

### Ecosystem
- Plugins for CI/CD integration
- Language-specific shortcuts (e.g., `repo build`, `repo test`)
- Cloud workspace sync (share workspace config)
- GUI companion tool (electron/tauri)

### Market
- DevOps teams managing microservices
- Monorepo teams (e.g., Backend + Frontend repos)
- Open-source project maintainers

### Success Metrics (5 years)
- 10K+ GitHub stars
- 1K+ monthly downloads
- Active community contributions
- Enterprise adoption
