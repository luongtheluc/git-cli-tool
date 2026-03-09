use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use super::app::{App, OperationProgress};

/// Render a centered modal dialog showing operation progress
pub fn render_progress_modal(f: &mut Frame, app: &App) {
    if app.operation_progress.is_none() {
        return;
    }

    let progress = app.operation_progress.as_ref().unwrap();

    // Calculate modal dimensions: width 50-80% of terminal, height 7-10 lines
    let terminal_width = f.size().width as usize;
    let terminal_height = f.size().height as usize;

    let modal_width = std::cmp::min(80, std::cmp::max(50, terminal_width / 2));
    let modal_height = 10;

    // Center position
    let x = (terminal_width as u16 - modal_width as u16) / 2;
    let y = (terminal_height as u16 - modal_height as u16) / 2;

    let modal_area = Rect {
        x,
        y,
        width: modal_width as u16,
        height: modal_height as u16,
    };

    // Create modal box with border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Working ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));

    // Render the modal background and border
    f.render_widget(block.clone(), modal_area);

    // Inner content area (accounting for borders: 1 char on each side)
    let inner = Rect {
        x: modal_area.x + 1,
        y: modal_area.y + 1,
        width: modal_area.width.saturating_sub(2),
        height: modal_area.height.saturating_sub(2),
    };

    // Split inner area: operation name, progress bar, spacer, hint text
    let [name_area, progress_area, _spacer_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    // 1. Operation name
    let op_line = Line::raw(format!("{}  ", progress.op_name()))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(Paragraph::new(op_line), name_area);

    // 2. Progress bar or indeterminate spinner
    match progress {
        OperationProgress::Batch {
            completed,
            total,
            ..
        } => {
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(*completed as f64 / *total as f64)
                .label(format!("{}/{}", completed, total))
                .style(Style::default());
            f.render_widget(gauge, progress_area);
        }
        OperationProgress::Single { .. } => {
            // Indeterminate progress - animated spinner (braille dots)
            let spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"; // Braille spinner
            let frame = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() / 80) as usize // ~80ms per frame for smooth animation
                % spinner.len();
            let char = spinner.chars().nth(frame).unwrap_or('⠋');
            let msg = format!("{} Processing...", char);
            let p = Paragraph::new(msg)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center);
            f.render_widget(p, progress_area);
        }
    }

    // 3. Hint text
    let hint = Line::raw("Press ESC to cancel")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(hint).alignment(Alignment::Center), hint_area);
}
