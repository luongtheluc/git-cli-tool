use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use rayon::prelude::*;
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

mod cli;
mod commit_graph;
mod git_runner;
mod repo_scanner;
mod text_utils;
mod tui;
mod ui;

use cli::{Cli, Commands};
use repo_scanner::RepoInfo;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;

    match cli.command {
        Commands::List => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            ui::print_repo_table(&repos);
        }

        Commands::Checkout { branch, create } => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            run_batch(&repos, &selected, |path| git_runner::checkout(path, &branch, create));
        }

        Commands::Pull => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            run_batch(&repos, &selected, git_runner::pull);
        }

        Commands::Push => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            run_batch(&repos, &selected, git_runner::push);
        }

        Commands::Commit { message } => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            run_batch(&repos, &selected, |path| git_runner::commit(path, &message));
        }

        Commands::Git { args } => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_batch(&repos, &selected, |path| {
                git_runner::run_git(path, &str_args)
            });
        }

        Commands::Run { script, jobs } => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;

            if selected.is_empty() {
                println!("{}", "  No repositories selected.".dimmed());
                return Ok(());
            }

            let parallelism = resolve_jobs(jobs);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(parallelism)
                .build()
                .context("failed to build thread pool")?;

            let print_lock = Arc::new(Mutex::new(()));
            let ok_count = Arc::new(AtomicUsize::new(0));
            let fail_count = Arc::new(AtomicUsize::new(0));
            let selected_repos: Vec<&RepoInfo> = selected.iter().map(|&i| &repos[i]).collect();
            let total = selected_repos.len();

            println!(
                "  Running {} on {} repo(s) with {} job(s)...\n",
                script.bold(),
                total,
                parallelism
            );

            pool.install(|| {
                selected_repos.par_iter().for_each(|repo| {
                    let tool = git_runner::detect_build_tool(&repo.path);
                    let resolved_cmd = git_runner::resolve_script(&script, &tool);
                    let prefix = format!("[{}] ", repo.name.cyan());
                    {
                        let _g = print_lock.lock().unwrap();
                        println!("{}{}", prefix, format!("> {resolved_cmd}").dimmed());
                    }
                    match git_runner::run_shell(&repo.path, &resolved_cmd, &prefix, &print_lock) {
                        Ok(true) => {
                            ok_count.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(false) => {
                            let _g = print_lock.lock().unwrap();
                            eprintln!("{}{}", prefix, "✗ exited with non-zero status".red());
                            fail_count.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            let _g = print_lock.lock().unwrap();
                            eprintln!("{}{}", prefix, format!("✗ {e}").red());
                            fail_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            });

            let ok = ok_count.load(Ordering::Relaxed);
            let fail = fail_count.load(Ordering::Relaxed);
            println!();
            if fail == 0 {
                println!(
                    "{}",
                    format!("  {} of {} completed successfully.", ok, total)
                        .green()
                        .bold()
                );
            } else {
                println!(
                    "{}",
                    format!("  {} ok, {} failed (of {}).", ok, fail, total)
                        .yellow()
                        .bold()
                );
            }
        }

        Commands::Audit => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            if repos.is_empty() {
                println!("{}", "  No repositories found.".dimmed());
                return Ok(());
            }
            // Parallel audit using rayon — offline, no fetch
            let results: Vec<(&RepoInfo, git_runner::AuditResult)> = repos
                .par_iter()
                .map(|r| (r, git_runner::audit_repo(&r.path)))
                .collect();

            let has_issues = results.iter().any(|(_, a)| a.has_issues());
            print_audit_table(&results);

            if has_issues {
                std::process::exit(1);
            }
        }

        Commands::AuditDeps => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            if repos.is_empty() {
                println!("{}", "  No repositories found.".dimmed());
                return Ok(());
            }
            // Parallel npm/yarn audit using rayon
            let results: Vec<(&RepoInfo, Option<git_runner::NpmAuditResult>)> = repos
                .par_iter()
                .map(|r| (r, git_runner::run_npm_audit(&r.path)))
                .collect();

            let has_vulnerable = results.iter().any(|(_, r)| {
                r.as_ref().map_or(false, |a| a.is_vulnerable())
            });
            print_npm_audit_table(&results);

            if has_vulnerable {
                std::process::exit(1);
            }
        }

        Commands::Ui => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            if repos.is_empty() {
                println!("{}", "  No repositories found in current directory.".dimmed());
            } else {
                tui::run_tui(repos)?;
            }
        }

        Commands::Status => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            if selected.is_empty() {
                println!("{}", "  No repositories selected.".dimmed());
            } else {
                for &idx in &selected {
                    let repo = &repos[idx];
                    let files = git_runner::status_files(&repo.path).unwrap_or_default();
                    ui::print_status_block(repo, &files);
                }
                println!(); // trailing newline
            }
        }
    }

    Ok(())
}

/// Print color-coded npm/yarn vulnerability audit table to stdout.
fn print_npm_audit_table(results: &[(&RepoInfo, Option<git_runner::NpmAuditResult>)]) {
    let name_w = results.iter().map(|(r, _)| r.name.len()).max().unwrap_or(4).max(4);

    println!(
        "\n  {:<nw$}  {:<5}  {:>8}  {:>5}  {:>6}  {:>4}  {}",
        "Repo", "Tool", "Critical", "High", "Medium", "Low", "Status",
        nw = name_w
    );
    let sep_len = name_w + 5 + 8 + 5 + 6 + 4 + 12 + 14;
    println!("  {}", "─".repeat(sep_len));

    let (mut vuln_count, mut clean_count, mut skip_count) = (0usize, 0usize, 0usize);

    for (repo, audit_opt) in results {
        match audit_opt {
            None => {
                skip_count += 1;
                let row = format!(
                    "  {:<nw$}  {:<5}  {}",
                    repo.name, "—", "(skipped — no package.json)",
                    nw = name_w
                );
                println!("{}", row.dimmed());
            }
            Some(a) if a.error.is_some() => {
                skip_count += 1;
                let tool = if a.has_yarn { "yarn" } else { "npm" };
                let row = format!(
                    "  {:<nw$}  {:<5}  {}",
                    repo.name, tool, a.error.as_deref().unwrap_or("unknown error"),
                    nw = name_w
                );
                println!("{}", row.red());
            }
            Some(a) => {
                let tool = if a.has_yarn { "yarn" } else { "npm" };
                let crit_s = if a.critical > 0 { format!("✗ {}", a.critical) } else { "—".into() };
                let high_s = if a.high > 0     { format!("✗ {}", a.high) }     else { "—".into() };
                let med_s  = if a.moderate > 0  { a.moderate.to_string() }      else { "—".into() };
                let low_s  = if a.low > 0       { a.low.to_string() }           else { "—".into() };
                let status = if a.is_vulnerable() { "✗ vulnerable" }
                             else if a.is_clean() { "✓ clean" }
                             else { "⚠ low risk" };

                let row = format!(
                    "  {:<nw$}  {:<5}  {:>8}  {:>5}  {:>6}  {:>4}  {}",
                    repo.name, tool, crit_s, high_s, med_s, low_s, status,
                    nw = name_w
                );
                let colored = if a.is_vulnerable() {
                    row.red().to_string()
                } else if a.is_clean() {
                    row.green().to_string()
                } else {
                    row.yellow().to_string()
                };
                println!("{}", colored);

                if a.is_vulnerable() { vuln_count += 1; } else { clean_count += 1; }
            }
        }
    }

    let npm_total = vuln_count + clean_count;
    println!("  {}", "─".repeat(sep_len));
    println!(
        "  {} npm/yarn repos | {} vulnerable | {} clean",
        npm_total, vuln_count, clean_count
    );
    if skip_count > 0 {
        println!("  {} repos skipped (no package.json)", skip_count);
    }
    println!();
}

/// Print color-coded audit table to stdout. Returns after printing summary footer.
fn print_audit_table(results: &[(&RepoInfo, git_runner::AuditResult)]) {
    let name_w = results.iter().map(|(r, _)| r.name.len()).max().unwrap_or(4).max(4);
    let branch_w = results.iter().map(|(_, a)| a.branch.len()).max().unwrap_or(6).max(6);

    // Header
    println!(
        "  {:<nw$}  {:<bw$}  {:<13}  {:<8}  {:<6}  {}",
        "Repo", "Branch", "Uncommitted", "Unpushed", "Behind", "Status",
        nw = name_w, bw = branch_w
    );
    let sep_len = name_w + branch_w + 13 + 8 + 6 + 10 + 12;
    println!("  {}", "─".repeat(sep_len));

    let (mut warnings, mut behind_count, mut clean_count) = (0usize, 0usize, 0usize);

    for (repo, audit) in results {
        let uncommitted = if audit.uncommitted > 0 {
            format!("✗ {} file{}", audit.uncommitted, if audit.uncommitted == 1 { "" } else { "s" })
        } else {
            "✓".to_string()
        };
        let unpushed = if audit.unpushed > 0 { format!("↑ {}", audit.unpushed) } else { "—".to_string() };
        let behind   = if audit.behind   > 0 { format!("↓ {}", audit.behind)   } else { "—".to_string() };
        let status_str = match audit.severity() {
            "critical" => "✗ critical",
            "warning"  => "⚠ warning",
            "behind"   => "↓ behind",
            _          => "✓ clean",
        };

        let row = format!(
            "  {:<nw$}  {:<bw$}  {:<13}  {:<8}  {:<6}  {}",
            repo.name, audit.branch, uncommitted, unpushed, behind, status_str,
            nw = name_w, bw = branch_w
        );
        let colored = match audit.severity() {
            "critical" => row.red().to_string(),
            "warning"  => row.yellow().to_string(),
            "behind"   => row.cyan().to_string(),
            _          => row.green().to_string(),
        };
        println!("{}", colored);

        match audit.severity() {
            "warning" | "critical" => warnings += 1,
            "behind" => behind_count += 1,
            _ => clean_count += 1,
        }
    }

    println!("  {}", "─".repeat(sep_len));
    println!(
        "  {} repos | {} warning(s) | {} behind | {} clean",
        results.len(), warnings, behind_count, clean_count
    );
}

/// Resolve --jobs value: 0 → half of logical CPU count (min 1), else use as-is
fn resolve_jobs(jobs: usize) -> usize {
    if jobs == 0 {
        (std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            / 2)
        .max(1)
    } else {
        jobs
    }
}

/// Execute `operation` on each selected repo sequentially, printing a header and result per repo.
/// A failure in one repo prints an error in red but continues processing the rest.
/// Prints a summary line (ok/fail counts) at the end.
fn run_batch<F>(repos: &[RepoInfo], selected_indices: &[usize], operation: F)
where
    F: Fn(&Path) -> Result<String>,
{
    if selected_indices.is_empty() {
        println!("{}", "  No repositories selected.".dimmed());
        return;
    }

    let total = selected_indices.len();
    let mut ok = 0usize;
    let mut fail = 0usize;

    for &idx in selected_indices {
        let repo = &repos[idx];
        // Separator: ── repo-name ──────────────
        let label = format!(" {} ", repo.name);
        let line_len = 60usize.saturating_sub(label.len() + 2);
        println!(
            "\n{}{}{}",
            "──".dimmed(),
            label.bold().green(),
            "─".repeat(line_len).dimmed()
        );
        match operation(&repo.path) {
            Ok(output) => {
                ok += 1;
                if output.is_empty() {
                    println!("{}", "  ✓ Done.".green());
                } else {
                    // Indent each output line slightly
                    for line in output.lines() {
                        println!("  {line}");
                    }
                }
            }
            Err(e) => {
                fail += 1;
                eprintln!("{}", format!("  ✗ {e}").red());
            }
        }
    }

    // Summary footer
    println!();
    let summary = if fail == 0 {
        format!("  {} of {} completed successfully.", ok, total)
            .green()
            .bold()
            .to_string()
    } else {
        format!("  {} ok, {} failed (of {}).", ok, fail, total)
            .yellow()
            .bold()
            .to_string()
    };
    println!("{}", summary);
}
