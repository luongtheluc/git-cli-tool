# Project Overview & Product Development Requirements (PDR)

## Project Summary

**Name:** repo
**Version:** 0.1.0
**Type:** Rust CLI Application
**Platform:** Windows (primary), cross-platform (macOS/Linux support)
**License:** MIT (implied)

repo is a polyrepo manager — a command-line tool for discovering, managing, and batch-operating across multiple Git repositories in a workspace directory.

## Vision & Purpose

Reduce cognitive load and operational friction when managing many related Git repositories. Enable developers to:
- Quickly discover all repos in a workspace
- Run common Git operations (pull, push, checkout, commit, status) across multiple repos with interactive selection
- See unified view of branch, commit status, and dirty state across all repos
- Execute batch operations without scripting or manual repo-by-repo iteration

## Functional Requirements

### F1: Repository Discovery
- Auto-scan workspace directory for Git repositories
- Discover only direct children (depth 2) — avoid nested/monorepo confusion
- Parallelize metadata collection for performance with 50+ repos
- Sort results alphabetically by repo name
- Handle errors gracefully (missing branches, no commits) with placeholder fallbacks

### F2: Repository Information Display
- Show repo name, current branch, last commit (short hash + message)
- Highlight repos with uncommitted changes with visual indicator
- Display in aligned ASCII table format with clean column padding
- Support ANSI color output (branch in cyan, hash dimmed, dirty state in yellow)

### F3: Interactive Repository Selection
- Present multi-select TUI using dialoguer crate
- Support space-toggle for selection, enter to confirm
- Show same info columns (name, branch, hash, message) in selection prompt
- Return indices of selected repos (not references, to avoid lifetime issues)
- Handle empty selection gracefully

### F4: Batch Git Operations
- Support six main operations: list, checkout, pull, push, commit, status
- Execute operations sequentially per-repo in single session
- Print operation header per repo (name in green, bold)
- Print success output or error message per repo
- Continue on error in one repo (don't abort batch)
- Print errors in red for visibility

### F5: Command-Line Interface
- Single binary entry point: `repo`
- Subcommands: list, checkout, pull, push, commit, status
- Argument: branch name for checkout; message string for commit
- Version and help text via clap derive macros
- Proper error messages for missing git, invalid paths, etc.

## Non-Functional Requirements

### NF1: Performance
- Scan 50+ repos in <2 seconds
- Parallelize discovery phase with rayon work-stealing
- Use rayon par_iter for metadata collection
- Avoid repeated Git calls for same metadata

### NF2: Reliability
- Handle missing/corrupt Git repos without crashing
- Fallback to "?" for branch, "no commits" for message on error
- Use proper error handling (anyhow Result types)
- Exit with meaningful error messages

### NF3: Portability
- Windows: built-in; uses ANSI colors (Windows 10+)
- macOS/Linux: support via cross-compilation or native build
- Use conditional compilation for Windows-specific setup installer
- Shell-agnostic commands (no bash-only scripts)

### NF4: User Experience
- Clear column alignment in table output
- ANSI padding must preserve column width even with color codes
- Interactive selection before batch operations (avoid accidental mass changes)
- Red error output for critical issues
- Progress headers per repo (no silent operations)

### NF5: Code Quality
- All public functions documented with doc comments
- Unit tests for scanning, Git operations, error cases
- 100% error case coverage in batch execution
- Modular design: cli, scanner, git_runner, ui modules
- No unsafe code

### NF6: Dependencies
- Minimize external crates; prefer stdlib where feasible
- clap 4 (derive) for CLI parsing
- dialoguer 0.11 for interactive TUI
- colored 2 for terminal colors
- walkdir 2 for recursive scanning
- anyhow 1 for error handling
- rayon 1 for parallelization
- console 0.15 (dialoguer dependency)
- winreg 0.52 (Windows-only, for installer)

## Acceptance Criteria

### AC1: Repository Discovery
- [ ] Scanning workspace with 3 repos returns all 3 in alphabetical order
- [ ] Scanning empty workspace returns empty list
- [ ] Scanning non-git directories excludes them
- [ ] Parallel metadata collection completes without race conditions
- [ ] Error repos (missing HEAD, no commits) show fallback values

### AC2: Interactive Selection
- [ ] MultiSelect prompt appears with all repos listed
- [ ] Space toggles selection state
- [ ] Enter confirms and returns indices
- [ ] Empty selection shows message, returns empty vec
- [ ] Column alignment preserved in prompt (no jagged lines)

### AC3: Batch Operations
- [ ] Checkout, pull, push, commit, status execute on all selected repos
- [ ] Error in one repo prints in red, continues to next
- [ ] Success output printed with green header
- [ ] Each repo shows clear separator header with name
- [ ] No repos selected shows informative message

### AC4: Table Output
- [ ] `repo list` displays all repos with aligned columns
- [ ] Dirty indicator (*) shown in yellow
- [ ] Commit hash in dimmed color
- [ ] Column widths adjust to longest entry + minimum width
- [ ] Header row bold and underlined

### AC5: Error Handling
- [ ] Missing git binary produces clear error
- [ ] Invalid workspace path caught
- [ ] Unopenable directories logged
- [ ] All Result types properly propagated
- [ ] No panics on user input

## Success Metrics

- **Usability:** User can list, select, and run operation on 10 repos in <10 seconds
- **Performance:** Scan time linear with repo count; 50 repos < 3 seconds
- **Reliability:** 9/9 unit tests pass; no crashes on edge cases
- **Code:** Zero unsafe code; all functions have doc comments
- **Cross-platform:** Builds on Windows, macOS, Linux without modification

## Architecture Overview

```
main.rs
├── Cli parse (clap)
├── Commands dispatch
└── run_batch()
    ├── ui::select_repos() → Vec<usize>
    ├── Iterate selected indices
    └── git_runner::{operation}()

repo_scanner.rs
├── find_git_repos() → Vec<PathBuf>
│   └── WalkDir depth 2
├── build_repo_info() → RepoInfo
│   ├── get_branch()
│   ├── get_last_commit()
│   └── has_changes()
└── parallel par_iter

ui.rs
├── select_repos() → Vec<usize>
│   └── dialoguer MultiSelect
└── print_repo_table()
    └── Aligned column formatting

git_runner.rs
└── run_git() wrapper
    └── Executes `git -C <path>` via Command
```

## Key Technical Decisions

1. **Depth-2 scanning:** Only direct children, prevents nested repo confusion
2. **Indices over references:** ui::select_repos returns Vec<usize> to avoid Rust lifetime complexity
3. **Padding before color:** Column padding applied to raw strings BEFORE ANSI codes, preserves width
4. **Sequential batch execution:** Simpler error handling than parallel; errors visible immediately
5. **Fallback metadata:** "?" branch, "no commits" message on error — avoids crashes, improves UX
6. **anyhow Result:** Consistent error handling across all modules

## Version History

- **0.1.0** (2026-03-05) — Initial release; 6 commands, parallel scanning, interactive selection
