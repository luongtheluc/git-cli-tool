pub mod app;
pub mod batch_ops;
pub mod events;
pub mod ui;
pub mod modal;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, panic, time::Duration};

use app::App;
use crate::repo_scanner::RepoInfo;

/// Entry point: set up the terminal, run the event loop, restore terminal on exit.
pub fn run_tui(repos: Vec<RepoInfo>) -> Result<()> {
    // Install a panic hook that restores the terminal before printing the panic message,
    // so the user's shell is not left in raw mode on an unexpected crash.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load status for the first repo before the first frame is drawn
    let mut app = App::new(repos);
    app.refresh_status();

    let result = event_loop(&mut terminal, &mut app);

    // Always restore terminal, even if the loop returned an error
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Parse a pipe-delimited audit message back into an AuditResult.
/// Format: "branch|uncommitted|unpushed|behind|no_upstream"
fn parse_audit_result(msg: &str) -> Option<crate::git_runner::AuditResult> {
    let mut parts = msg.splitn(5, '|');
    let branch = parts.next()?.to_string();
    let uncommitted = parts.next()?.parse().ok()?;
    let unpushed = parts.next()?.parse().ok()?;
    let behind = parts.next()?.parse().ok()?;
    let no_upstream = parts.next()? == "true";
    Some(crate::git_runner::AuditResult { branch, uncommitted, unpushed, behind, no_upstream })
}

/// Parse a pipe-encoded npm audit message back into an NpmAuditResult.
/// Formats:
///   "skip"               → None (not an npm/yarn repo)
///   "e|<1|0>|<message>"  → error result
///   "v|<1|0>|c|h|m|l|i|t" → vulnerability counts
fn parse_npm_audit_result(msg: &str) -> Option<crate::git_runner::NpmAuditResult> {
    if msg == "skip" { return None; }
    let mut parts = msg.splitn(3, '|');
    let kind = parts.next()?;
    let has_yarn = parts.next()? == "1";
    match kind {
        "e" => {
            let err = parts.next().unwrap_or("error");
            Some(crate::git_runner::NpmAuditResult {
                critical: 0, high: 0, moderate: 0, low: 0, info: 0, total: 0,
                has_yarn, error: Some(err.to_string()),
            })
        }
        "v" => {
            let rest = parts.next()?;
            let mut nums = rest.split('|');
            let critical = nums.next()?.parse().ok()?;
            let high     = nums.next()?.parse().ok()?;
            let moderate = nums.next()?.parse().ok()?;
            let low      = nums.next()?.parse().ok()?;
            let info     = nums.next()?.parse().ok()?;
            let total    = nums.next()?.parse().ok()?;
            Some(crate::git_runner::NpmAuditResult {
                critical, high, moderate, low, info, total, has_yarn, error: None,
            })
        }
        _ => None,
    }
}

/// Main loop: draw one frame, then block up to 100 ms for a key event.
/// 100 ms tick keeps CPU near zero while still feeling instantaneous to users.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;


        // Poll batch operation results (non-blocking, incremental updates)
        if let Some(_) = app.batch_receiver.as_ref() {
            let batch_complete = app.poll_batch_results();

            if batch_complete {
                // Batch finished - show completion message and keep modal open for review.
                app.animation_start = None;
                if let Some(app::OperationProgress::Batch { total, completed: _, op_name, results })
                    = app.operation_progress.as_ref()
                {
                    let success = results.iter().filter(|r| r.success).count();
                    let failed: Vec<String> = results
                        .iter()
                        .filter(|r| !r.success)
                        .map(|r| format!("{}: {}", r.repo_name, r.message))
                        .collect();

                    let is_audit = op_name == "Auditing";
                    let is_npm_audit = op_name == "Npm Audit";
                    // Collect result data before leaving borrow scope (cloned to owned)
                    let audit_data: Vec<(String, String)> = if is_audit || is_npm_audit {
                        results
                            .iter()
                            .filter(|r| r.success)
                            .map(|r| (r.repo_name.clone(), r.message.clone()))
                            .collect()
                    } else {
                        vec![]
                    };

                    if failed.is_empty() {
                        app.message = Some(format!("{} — {}/{} succeeded", op_name, success, total));
                    } else {
                        app.message = Some(format!(
                            "{} — {}/{} succeeded | Failed: {}",
                            op_name,
                            success,
                            total,
                            failed.join(", ")
                        ));
                    }

                    // Populate audit cache and switch to audit view
                    if is_audit {
                        for (repo_name, msg) in &audit_data {
                            if let Some(idx) = app.repos.iter().position(|r| &r.name == repo_name) {
                                if let Some(audit) = parse_audit_result(msg) {
                                    app.audit_cache.insert(idx, audit);
                                }
                            }
                        }
                        app.main_view = app::MainView::Audit;
                    }
                    // Populate npm audit cache and switch to npm audit view
                    if is_npm_audit {
                        for (repo_name, msg) in &audit_data {
                            if let Some(idx) = app.repos.iter().position(|r| &r.name == repo_name) {
                                if let Some(result) = parse_npm_audit_result(msg) {
                                    app.npm_audit_cache.insert(idx, result);
                                }
                                // "skip" → repo has no package.json, don't insert
                            }
                        }
                        app.main_view = app::MainView::NpmAudit;
                    }
                }
            }
        }

        // Dynamic poll interval: 50ms during operations for smooth animation, 100ms idle
        let poll_duration = if app.operation_progress.is_some() {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(100)
        };

        if event::poll(poll_duration)? {
            events::handle(app)?;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
