# Tutorial: Using repo CLI

Complete guide to managing multiple Git repositories with the `repo` polyrepo manager.

## Workspace Structure

The `repo` command works on a workspace directory containing multiple Git repositories as direct children:

```
workspace/
├── api-service/       ← contains .git
├── auth-service/      ← contains .git
├── frontend/          ← contains .git
└── worker/            ← contains .git
```

Run `repo` commands from the workspace directory.

---

## Listing All Repositories

**Display all discovered repositories with branch, commit, and dirty status:**

```bash
$ repo list

REPO              BRANCH        LAST COMMIT
──────────────────────────────────────────────────────────────
api-service       develop       a12bc3 fix auth bug
auth-service      main          7a91de update deps          *
frontend          feature/ui    0ab221 add layout
worker            develop       c2f991 fix queue
```

Legend:
- `*` indicates repos with uncommitted changes.
- Hash is dimmed; message is normal; dirty indicator is yellow.

---

## Checking Out a Branch

**Switch selected repos to a specific branch:**

```bash
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

**Create a new branch if it doesn't exist:**

```bash
$ repo checkout feature/new-ui -b
```

---

## Pulling Latest Changes

**Fetch and merge from remote on selected repos:**

```bash
$ repo pull

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] auth-service     main         7a91de update deps
  [ ] frontend         feature/ui   0ab221 add layout
  [x] worker           develop      c2f991 fix queue

=== api-service ===
Already up to date.

=== auth-service ===
Fast-forward
 package.json | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)

=== worker ===
Already up to date.
```

---

## Pushing Changes

**Push commits to remote on selected repos:**

```bash
$ repo push

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] auth-service     main         7a91de update deps
  [x] worker           develop      c2f991 fix queue

=== api-service ===
Everything up-to-date

=== auth-service ===
Enumerating objects: 5, done.
...
 7a91de..4b2c1f  main -> main

=== worker ===
Everything up-to-date
```

---

## Committing Changes

**Stage all changes and commit with a message:**

```bash
$ repo commit -m "update dependencies"

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [ ] auth-service     main         7a91de update deps
  [x] worker           develop      c2f991 fix queue

=== api-service ===
[develop 5e4a2c] update dependencies
 2 files changed, 10 insertions(+)

=== worker ===
[develop 8f3b2a] update dependencies
 1 file changed, 5 insertions(+)
```

---

## Checking Repository Status

**Show git status (staged, unstaged, untracked files) for selected repos:**

```bash
$ repo status

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] auth-service     main         7a91de update deps
  [x] worker           develop      c2f991 fix queue

=== api-service ===
On branch develop
Your branch is up to date with 'origin/develop'.

Changes to be committed:
  (use "git reset HEAD <file>..." to unstage)
    modified:   src/main.rs

Untracked files:
  (use "git add <file>..." to include in what will be tracked)
    .env.local

=== auth-service ===
On branch main
Your branch is up to date with 'origin/main'.

nothing to commit, working tree clean

=== worker ===
On branch develop
Your branch is ahead of 'origin/develop' by 2 commits.
...
```

---

## Auditing Repository Health

**Check offline git health (uncommitted, unpushed, behind-remote status):**

```bash
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

**Exit codes:**
- `0` if all repos are clean
- `1` if any issues detected (uncommitted, unpushed, behind)

Perfect for CI/CD pipelines to fail fast if workspace is dirty.

---

## Running Custom Git Commands

**Execute arbitrary git commands across selected repos:**

```bash
$ repo git -- log --oneline -5

Select repositories (space to toggle, enter to confirm):
> [x] api-service      develop      a12bc3 fix auth bug
  [x] worker           develop      c2f991 fix queue

=== api-service ===
a12bc3 fix auth bug
b8a9f2 refactor login
c3d4e5 add oauth2
d4e5f6 update deps
e5f6g7 initial commit

=== worker ===
c2f991 fix queue
d3g0a2 add retry logic
...
```

**More examples:**

```bash
# Show git stash in all repos
$ repo git -- stash list

# Show file diffs
$ repo git -- diff --stat

# Show remote tracking branches
$ repo git -- branch -avv
```

---

## Interactive Terminal Dashboard

**Launch full-screen interactive TUI for batch operations and monitoring:**

```bash
$ repo ui
```

### Features
- **Sidebar:** Multi-select repos with checkboxes (Space to toggle, A to select all, Shift+A to deselect all)
- **Repo metadata:** Branch, tags, commit time, changed file count displayed per repo
- **Status panel:** Detailed git status (staged, modified, untracked) with color-coded output
- **Batch operations:** Execute operations via keyboard shortcuts:
  - `P` — Pull from remote
  - `U` — Push to remote
  - `F` — Fetch from remote
  - `C` — Commit (prompts for message)
  - `B` — Checkout (prompts for branch)
  - `A` — Toggle to audit view (health status)
  - `N` — Toggle npm/yarn audit view (vulnerability scanning)
- **Navigation:** ↑↓ or j/k to move up/down; Esc or q to quit
- **Live feedback:** ✓ (success) and ✗ (error) indicators for operations
- **Terminal-safe:** Panic hook ensures terminal restoration on crash

---

## Auditing NPM/Yarn Vulnerabilities

**Scan selected repos for npm/yarn package vulnerabilities:**

```bash
$ repo audit-deps

Select repositories (space to toggle, enter to confirm):
> [x] frontend          feature/ui   0ab221 add layout
  [x] api-service       develop      a12bc3 fix auth bug
  [ ] worker            develop      c2f991 fix queue

=== frontend ===
  Package             Critical  High   Medium  Low   Status
  ─────────────────────────────────────────────────────────
  lodash              1         3      2       5     ✗ VULNERABLE
  express             0         1      4       2     ⚠ needs review
  react               0         0      0       1     ✓ ok

=== api-service ===
  Package             Critical  High   Medium  Low   Status
  ─────────────────────────────────────────────────────────
  all packages        0         0      0       0     ✓ clean
```

**Exit codes:**
- `0` if all repos are clean
- `1` if critical or high-severity vulnerabilities found

### In TUI Dashboard
Press `N` to toggle npm/yarn audit view. Shows sidebar icons:
- `N✓` — clean (no vulnerabilities)
- `N✗` — has critical/high vulns
- `N⚠` — has medium/low vulns
- `N?` — error scanning (no package.json or audit failed)

---

## Tips & Workflows

### Safe Batch Operations
1. Always run `repo list` first to see all repos
2. Use `repo audit` before batch pull/push to spot issues
3. Review selected repos in selection UI before confirming

### Common Workflows

**Sync all to main branch, then pull:**
```bash
$ repo checkout main
$ repo pull
```

**Check health before releasing:**
```bash
$ repo audit
$ repo audit-deps
```

**Stash uncommitted work across all repos:**
```bash
$ repo git -- stash
```

**Create release tags everywhere:**
```bash
$ repo git -- tag -a v2.0.0 -m "Release v2.0.0"
```

---

## Troubleshooting

### Git command not found
Ensure git is installed and in your PATH. Test: `git --version`

### Repos not discovered
Check workspace directory structure. `repo list` must run from parent directory containing .git folders.

### Selection UI appears frozen
Press Escape to cancel or Space/Enter to confirm selection. If stuck, press Ctrl+C.

### Terminal colors look wrong
Colors use ANSI 16-color palette (Windows 10+ supports this). On older Windows, disable colors via config (future feature).

---

## See Also
- `repo --help` — Full command reference
- `docs/project-overview-pdr.md` — Design & requirements
- `docs/system-architecture.md` — Technical architecture
