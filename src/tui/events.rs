use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use super::app::{App, InputMode, InputPurpose, Panel};
use super::batch_ops::BatchOp;

/// Read one key event and mutate app state accordingly.
/// Routes to input_handle() or normal_handle() based on current mode.
pub fn handle(app: &mut App) -> Result<()> {
    match event::read()? {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match app.input_mode {
                InputMode::Input => handle_input_mode(app, key.code),
                InputMode::Normal => handle_normal_mode(app, key.code, key.modifiers),
            }
        }
        Event::Mouse(mouse) => {
            // Wheel scrolling navigates repo selection when not blocked by modal/input mode.
            if app.input_mode == InputMode::Normal
                && app.operation_progress.is_none()
                && app.focused_panel == Panel::Sidebar
            {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.move_up();
                        app.ensure_status_loaded();
                    }
                    MouseEventKind::ScrollDown => {
                        app.move_down();
                        app.ensure_status_loaded();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
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
                InputPurpose::GitCommand => {
                    let mut args = match parse_git_args(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            app.message = Some(format!("Invalid git args: {}", e));
                            app.input_mode = InputMode::Normal;
                            return;
                        }
                    };

                    // Allow users to type either "pull --rebase" or "git pull --rebase"
                    if args.first().map(|s| s.as_str()) == Some("git") {
                        args.remove(0);
                    }

                    if args.is_empty() {
                        app.message = Some("Git command cannot be empty".into());
                        app.input_mode = InputMode::Normal;
                        return;
                    }

                    // Safety rail: block destructive commands across multiple repos.
                    if app.selected_indices().len() > 1 && is_risky_git_args(&args) {
                        app.message = Some(
                            "Blocked risky git command for multi-repo selection".into(),
                        );
                        app.input_mode = InputMode::Normal;
                        return;
                    }

                    BatchOp::Git(args)
                }
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

fn parse_git_args(input: &str) -> std::result::Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut token_started = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            token_started = true;
            continue;
        }

        match quote {
            Some(q) => {
                if ch == '\\' {
                    // Inside quotes, allow escaping quote and backslash.
                    escaped = true;
                    continue;
                }
                if ch == q {
                    quote = None;
                    token_started = true;
                } else {
                    current.push(ch);
                    token_started = true;
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                    token_started = true;
                } else if ch.is_whitespace() {
                    if token_started {
                        args.push(std::mem::take(&mut current));
                        token_started = false;
                    }
                } else {
                    current.push(ch);
                    token_started = true;
                }
            }
        }
    }

    if escaped {
        return Err("trailing escape character".into());
    }
    if quote.is_some() {
        return Err("unclosed quote".into());
    }
    if token_started {
        args.push(current);
    }

    Ok(args)
}

fn is_risky_git_args(args: &[String]) -> bool {
    let Some(idx) = find_git_subcommand_index(args) else {
        return false;
    };

    let sub = args[idx].as_str();
    if sub == "reset" || sub == "clean" {
        return true;
    }

    if sub == "push"
        && args
            .iter()
            .skip(idx + 1)
            .any(|a| a == "--force" || a == "-f" || a == "--force-with-lease")
    {
        return true;
    }

    false
}

fn find_git_subcommand_index(args: &[String]) -> Option<usize> {
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            return if i + 1 < args.len() { Some(i + 1) } else { None };
        }

        if !a.starts_with('-') {
            return Some(i);
        }

        // Global options that consume the next value when not using '=' form.
        if a == "-c" || a == "-C" || a == "--git-dir" || a == "--work-tree" || a == "--namespace" || a == "--config-env" {
            i += 1;
        }
        i += 1;
    }
    None
}

/// Handle keys in normal navigation mode
fn handle_normal_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // While modal is open, limit keys to quit/close/scroll controls.
    if app.operation_progress.is_some() {
        match code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => app.scroll_batch_results_up(1),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_batch_results_down(1),
            KeyCode::PageUp => app.scroll_batch_results_up(5),
            KeyCode::PageDown => app.scroll_batch_results_down(5),
            KeyCode::Home => app.reset_batch_result_scroll(),
            KeyCode::Esc => {
                match &app.operation_progress {
                    Some(super::app::OperationProgress::Single { .. }) => {
                        // Allow dismissal for single operations (quick, synchronous)
                        app.operation_progress = None;
                        app.reset_batch_result_scroll();
                        app.message = Some("Operation dismissed".into());
                    }
                    Some(super::app::OperationProgress::Batch { .. }) => {
                        if app.batch_receiver.is_some() {
                            // Batch is still running; don't allow closing modal.
                            app.message = Some("Cannot close while batch is running".into());
                        } else {
                            // Completed: allow explicit close after user reviews results.
                            app.operation_progress = None;
                            app.reset_batch_result_scroll();
                            app.message = Some("Closed batch result dialog".into());
                        }
                    }
                    None => {}
                }
            }
            // Provide feedback for blocked keys
            KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Char('f') 
            | KeyCode::Char('c') | KeyCode::Char('b') | KeyCode::Char('g') => {
                app.message = Some("Operation already in progress".into());
            }
            _ => {} // Ignore other keys during operation
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
        KeyCode::Char('g') => app.enter_input(InputPurpose::GitCommand),

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

    // Batch actions always use Batch progress because results arrive via async channel.
    app.operation_progress = Some(super::app::OperationProgress::Batch {
        total,
        completed: 0,
        op_name,
        results: Vec::new(),
    });
    app.reset_batch_result_scroll();

    // Invalidate status cache for affected repos
    app.invalidate_status(&indices);
    // Start animation timer for smooth spinner
    app.animation_start = Some(std::time::Instant::now());


    // Store receiver for incremental polling in the event loop
    app.batch_receiver = Some(rx);
}
