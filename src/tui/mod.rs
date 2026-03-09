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

/// Main loop: draw one frame, then block up to 100 ms for a key event.
/// 100 ms tick keeps CPU near zero while still feeling instantaneous to users.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            events::handle(app)?;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
