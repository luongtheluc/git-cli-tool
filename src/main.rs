use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::env;
use std::path::Path;

mod cli;
mod git_runner;
mod repo_scanner;
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

        Commands::Checkout { branch } => {
            let repos = repo_scanner::scan_repos(&cwd)?;
            let selected = ui::select_repos(&repos)?;
            run_batch(&repos, &selected, |path| git_runner::checkout(path, &branch));
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
