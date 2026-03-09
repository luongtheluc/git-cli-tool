use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use super::app::{App, InputMode, InputPurpose, Panel};
use super::batch_ops::BatchOp;

/// Read one key event and mutate app state accordingly.
/// Routes to input_handle() or normal_handle() based on current mode.
pub fn handle(app: &mut App) -> Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match app.input_mode {
            InputMode::Input => handle_input_mode(app, key.code),
            InputMode::Normal => handle_normal_mode(app, key.code, key.modifiers),
        }
    }
    Ok(())
}

/// Handle keys in text input mode (commit message, branch name)
fn handle_input_mode(app: &mut App, code: KeyCode) {
    match code {
        // Confirm input → execute the batch operation
        KeyCode::Enter => {
            let text = app.input_buffer.clone();
            if text.is_empty() {
                app.message = Some("Input cancelled (empty).".into());
                app.input_mode = InputMode::Normal;
                return;
            }
            let op = match app.input_purpose {
                InputPurpose::CommitMessage => BatchOp::Commit(text),
                InputPurpose::BranchName => BatchOp::Checkout(text),
                InputPurpose::None => {
                    app.input_mode = InputMode::Normal;
                    return;
                }
            };
            app.input_mode = InputMode::Normal;
            start_batch(app, op);
        }
        // Cancel input
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
            app.message = Some("Input cancelled.".into());
        }
        // Delete last character
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        // Append character to buffer
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

/// Handle keys in normal navigation mode
fn handle_normal_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Block all keys except quit while a batch operation is running
    if app.batch_progress.is_some() {
        if code == KeyCode::Char('q') || code == KeyCode::Esc {
            app.message = Some("Batch operation in progress...".into());
        }
        return;
    }

    match code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Navigate sidebar
        KeyCode::Up | KeyCode::Char('k') if app.focused_panel == Panel::Sidebar => {
            app.move_up();
            app.ensure_status_loaded();
        }
        KeyCode::Down | KeyCode::Char('j') if app.focused_panel == Panel::Sidebar => {
            app.move_down();
            app.ensure_status_loaded();
        }

        // Multi-select: Space toggles current, 'a' toggles all
        KeyCode::Char(' ') if app.focused_panel == Panel::Sidebar => {
            app.toggle_selected();
        }
        KeyCode::Char('a') if app.focused_panel == Panel::Sidebar => {
            app.toggle_all();
        }

        // Batch operations (no input needed)
        KeyCode::Char('p') => start_batch(app, BatchOp::Pull),
        KeyCode::Char('P') => start_batch(app, BatchOp::Push),
        KeyCode::Char('f') => start_batch(app, BatchOp::Fetch),

        // Batch operations (input needed)
        KeyCode::Char('c') => app.enter_input(InputPurpose::CommitMessage),
        KeyCode::Char('b') => app.enter_input(InputPurpose::BranchName),

        // Refresh status of selected repo
        KeyCode::Char('r') | KeyCode::F(5) => {
            app.refresh_status();
            app.message = Some("Refreshed.".into());
        }

        // Toggle panel focus
        KeyCode::Tab => {
            app.focused_panel = match app.focused_panel {
                Panel::Sidebar => Panel::Main,
                Panel::Main => Panel::Sidebar,
            };
        }

        _ => {}
    }
}

/// Execute a batch operation on selected repos with async progress tracking
fn start_batch(app: &mut App, op: BatchOp) {
    let indices = app.selected_indices();
    if indices.is_empty() {
        app.message = Some("No repositories selected.".into());
        return;
    }

    let op_name = op.display_name().to_string();
    let total = indices.len();

    // Use async execution with channel for live progress
    let rx = super::batch_ops::execute_batch_async(&app.repos, &indices, &op);

    // Store progress state — channel is polled in the event loop (mod.rs)
    app.batch_progress = Some(super::app::BatchProgress {
        total,
        completed: 0,
        op_name,
        results: Vec::new(),
    });

    // Invalidate status cache for affected repos
    app.invalidate_status(&indices);

    // Store receiver in a thread-local or process inline
    // Since we can't store rx in App easily, drain synchronously for now
    // and show results. Phase 4 upgrade: store rx, poll in event_loop.
    drain_batch_results(app, rx);
}

/// Drain all results from the batch channel and finalize progress
fn drain_batch_results(
    app: &mut App,
    rx: std::sync::mpsc::Receiver<super::app::BatchResult>,
) {
    let total = app.batch_progress.as_ref().map(|p| p.total).unwrap_or(0);
    let mut results = Vec::new();

    for result in rx.iter() {
        results.push(result);
        if results.len() >= total {
            break;
        }
    }

    let success = results.iter().filter(|r| r.success).count();
    let failed: Vec<String> = results
        .iter()
        .filter(|r| !r.success)
        .map(|r| format!("{}: {}", r.repo_name, r.message))
        .collect();

    let op_name = app
        .batch_progress
        .as_ref()
        .map(|p| p.op_name.clone())
        .unwrap_or_default();

    app.batch_progress = None;

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
}
