use anyhow::Result;
use colored::Colorize;
use dialoguer::MultiSelect;

use crate::repo_scanner::RepoInfo;

/// Max characters for commit message in multi-select list (keeps lines < 100 chars)
const MSG_W_SELECT: usize = 38;
/// Max characters for commit message in `repo list` table
const MSG_W_TABLE: usize = 48;
/// Fixed width for the status badge column (e.g. "✗ ⇡10" = 5 chars + padding)
const STATUS_W: usize = 8;

// ─── Public API ──────────────────────────────────────────────────────────────

/// Show interactive multi-select TUI for repository selection.
/// Returns the indices of selected repos from the input slice.
pub fn select_repos(repos: &[RepoInfo]) -> Result<Vec<usize>> {
    if repos.is_empty() {
        println!("No git repositories found in current directory.");
        return Ok(vec![]);
    }

    let name_w = col_width(repos.iter().map(|r| r.name.len()), 10);
    let branch_w = col_width(repos.iter().map(|r| r.branch.len()), 8);

    // Plain text (no ANSI) — dialoguer applies its own highlighting.
    // Unicode symbols (✓ ✗ ⇡ ⇣) are fine; only ANSI escape codes are avoided.
    let items: Vec<String> = repos
        .iter()
        .map(|r| format_select_line(r, name_w, branch_w))
        .collect();

    // Pre-select all repos; user can deselect with Space, or press 'a' to toggle all
    let defaults: Vec<bool> = vec![true; items.len()];
    let selections = MultiSelect::new()
        .with_prompt("Select repositories (space=toggle, a=select/deselect all, enter=confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    Ok(selections)
}

/// Print an aligned, colorized table of all repos to stdout (used by `repo list`).
/// Includes a summary footer with clean / dirty / sync counts.
pub fn print_repo_table(repos: &[RepoInfo]) {
    if repos.is_empty() {
        println!("No git repositories found in current directory.");
        return;
    }

    let name_w = col_width(repos.iter().map(|r| r.name.len()), 16);
    let branch_w = col_width(repos.iter().map(|r| r.branch.len()), 12);
    let total_w = name_w + branch_w + STATUS_W + 12 + MSG_W_TABLE;

    // Header
    println!(
        "  {}  {}  {}  {}  {}",
        format!("{:<name_w$}", "REPO").bold(),
        format!("{:<branch_w$}", "BRANCH").bold(),
        format!("{:<8}", "HASH").bold(),
        format!("{:<STATUS_W$}", "STATUS").bold(),
        "COMMIT MESSAGE".bold(),
    );
    println!("  {}", "─".repeat(total_w));

    let mut n_clean = 0usize;
    let mut n_dirty = 0usize;
    let mut n_diverged = 0usize;

    for repo in repos {
        // Pad raw strings BEFORE colorizing to preserve column alignment with ANSI codes
        let branch_col = format!("{:<branch_w$}", repo.branch).cyan().to_string();
        let hash_col = format!("{:<8}", repo.last_commit_hash).dimmed().to_string();
        let msg = truncate_str(&repo.last_commit_msg, MSG_W_TABLE);
        let (status_raw, status_col) = status_badge_colored(repo);

        // Tally summary counts
        if repo.has_uncommitted_changes {
            n_dirty += 1;
        } else {
            n_clean += 1;
        }
        if repo.ahead > 0 || repo.behind > 0 {
            n_diverged += 1;
        }

        // Pad the raw (no-ANSI) status to STATUS_W before the colorized version
        let status_pad = STATUS_W.saturating_sub(status_raw.chars().count());
        let status_col = format!("{}{}", status_col, " ".repeat(status_pad));

        println!(
            "  {:<name_w$}  {}  {}  {}  {}",
            repo.name, branch_col, hash_col, status_col, msg,
        );
    }

    println!("  {}", "─".repeat(total_w));

    // Summary footer
    let total = repos.len();
    let clean_s = format!("{} clean", n_clean).green().to_string();
    let dirty_s = if n_dirty > 0 {
        format!("{} dirty", n_dirty).red().to_string()
    } else {
        format!("{} dirty", n_dirty).dimmed().to_string()
    };
    let sync_s = if n_diverged > 0 {
        format!("{} out of sync", n_diverged).yellow().to_string()
    } else {
        format!("all synced").green().to_string()
    };
    println!(
        "  {} repo(s) — {} · {} · {}",
        total.to_string().bold(),
        clean_s,
        dirty_s,
        sync_s,
    );
}

/// Print a formatted status block for a single repo.
/// Shows the status badge, branch, and per-file change list.
pub fn print_status_block(repo: &RepoInfo, files: &[(String, String)]) {
    let (status_raw, status_col) = status_badge_colored(repo);
    let _ = status_raw; // used for width in table; not needed here

    // Header line: ── name ── branch  STATUS  ahead/behind
    let label = format!(" {} ", repo.name);
    let line_len = 58usize.saturating_sub(label.len() + 2);
    let sync = match (repo.ahead, repo.behind) {
        (0, 0) => String::new(),
        (a, 0) => format!("  {}", format!("⇡{a}").yellow()),
        (0, b) => format!("  {}", format!("⇣{b}").yellow()),
        (a, b) => format!("  {} {}", format!("⇡{a}").yellow(), format!("⇣{b}").yellow()),
    };
    println!(
        "\n{}{}{}  {}  {}{}",
        "──".dimmed(),
        label.bold().green(),
        "─".repeat(line_len).dimmed(),
        repo.branch.cyan(),
        status_col,
        sync,
    );

    if files.is_empty() {
        println!("  {}", "✓ Nothing to commit, working tree clean".green());
        return;
    }

    // Per-file lines — color by staging area vs working tree
    for (code, path) in files {
        let staged_char = code.chars().next().unwrap_or(' ');
        let wt_char = code.chars().nth(1).unwrap_or(' ');

        let code_col = if code == "??" {
            code.dimmed().to_string()
        } else if staged_char != ' ' && wt_char == ' ' {
            code.green().to_string()          // fully staged
        } else if staged_char == ' ' {
            code.yellow().to_string()         // unstaged only
        } else {
            code.cyan().to_string()           // partially staged
        };

        println!("  {}  {}", code_col, path);
    }

    println!(
        "  {} file(s) changed",
        files.len().to_string().bold()
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a plain-text repo line for the multi-select prompt.
/// Uses Unicode symbols but no ANSI escape codes.
fn format_select_line(repo: &RepoInfo, name_w: usize, branch_w: usize) -> String {
    let status = status_badge_plain(repo);
    let msg = truncate_str(&repo.last_commit_msg, MSG_W_SELECT);
    format!(
        "{:<name_w$}  {:<branch_w$}  {:8}  {:<STATUS_W$}  {}",
        repo.name, repo.branch, repo.last_commit_hash, status, msg,
    )
}

/// Returns a colorized status badge `(raw_text, colored_text)`.
/// `raw_text` is used to measure display width; `colored_text` is printed.
fn status_badge_colored(repo: &RepoInfo) -> (String, String) {
    let dirty = repo.has_uncommitted_changes;
    let ahead = repo.ahead;
    let behind = repo.behind;

    let mut parts: Vec<String> = Vec::new();
    if dirty {
        parts.push("✗".red().to_string());
    } else {
        parts.push("✓".green().to_string());
    }
    if ahead > 0 {
        parts.push(format!("⇡{}", ahead).yellow().to_string());
    }
    if behind > 0 {
        parts.push(format!("⇣{}", behind).yellow().to_string());
    }

    // Raw (no ANSI) version for width calculation
    let mut raw_parts: Vec<String> = Vec::new();
    if dirty { raw_parts.push("✗".into()); } else { raw_parts.push("✓".into()); }
    if ahead > 0 { raw_parts.push(format!("⇡{}", ahead)); }
    if behind > 0 { raw_parts.push(format!("⇣{}", behind)); }

    (raw_parts.join(" "), parts.join(" "))
}

/// Plain-text status badge (no ANSI) for the multi-select prompt.
fn status_badge_plain(repo: &RepoInfo) -> String {
    let mut parts: Vec<String> = Vec::new();
    if repo.has_uncommitted_changes {
        parts.push("✗".into());
    } else {
        parts.push("✓".into());
    }
    if repo.ahead > 0 {
        parts.push(format!("⇡{}", repo.ahead));
    }
    if repo.behind > 0 {
        parts.push(format!("⇣{}", repo.behind));
    }
    parts.join(" ")
}

/// Truncate a string to `max` chars, appending `…` if cut.
fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Compute display column width: max of all values and a minimum floor.
fn col_width(lengths: impl Iterator<Item = usize>, min: usize) -> usize {
    lengths.max().unwrap_or(min).max(min)
}
