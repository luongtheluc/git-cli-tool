use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use rayon::prelude::*;
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

mod cli;
mod git_runner;
mod repo_scanner;
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
