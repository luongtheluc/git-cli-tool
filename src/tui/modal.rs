use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use super::app::{App, OperationProgress};
use crate::text_utils;

/// Render a centered modal dialog showing operation progress
pub fn render_progress_modal(f: &mut Frame, app: &App) {
    // C2 Fix: Use if let Some pattern instead of unwrap
    let Some(progress) = app.operation_progress.as_ref() else {
        return;
    };

    // H1 Fix: Terminal resize handling with minimum size checks and saturating arithmetic
    let terminal_width = f.size().width as usize;
    let terminal_height = f.size().height as usize;

    // Require minimum terminal size to avoid underflow/overflow
    if terminal_width < 40 || terminal_height < 8 {
        // Terminal too small - skip modal rendering
        return;
    }

    // Calculate modal dimensions with safe arithmetic
    let modal_width = std::cmp::min(
        terminal_width.saturating_sub(4), // Leave 2 char margins
        std::cmp::max(30, terminal_width / 2),
    );
    let modal_height = std::cmp::min(14, terminal_height.saturating_sub(2));

    // Center position using saturating math to prevent underflow
    let x = (terminal_width.saturating_sub(modal_width) / 2) as u16;
    let y = (terminal_height.saturating_sub(modal_height) / 2) as u16;

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

    // Split inner area: operation name, progress, live results, hint text
    let [name_area, progress_area, results_area, hint_area] = Layout::vertical([
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
            results,
            ..
        } => {
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(*completed as f64 / *total as f64)
                .label(format!("{}/{}", completed, total))
                .style(Style::default());
            f.render_widget(gauge, progress_area);

            if results_area.height > 0 {
                let line_budget = results_area.height as usize;
                let mut lines = Vec::new();

                // Summary/status line
                let header = if completed < total {
                    format!(
                        "Completed: {}/{} | Remaining: {}",
                        completed,
                        total,
                        total.saturating_sub(*completed)
                    )
                } else {
                    format!("Completed: {}/{} | Finished", completed, total)
                };
                lines.push(Line::raw(truncate_line(&header, results_area.width as usize))
                    .style(Style::default().fg(Color::DarkGray)));

                let visible_rows = line_budget.saturating_sub(1);
                if visible_rows > 0 {
                    let max_scroll = results.len().saturating_sub(visible_rows);
                    let scroll = app.batch_result_scroll.min(max_scroll);

                    if results.is_empty() {
                        lines.push(Line::raw("Waiting for first result...")
                            .style(Style::default().fg(Color::DarkGray)));
                    } else {
                        let end = (scroll + visible_rows).min(results.len());
                        for result in results.iter().skip(scroll).take(end.saturating_sub(scroll)) {
                            let status = if result.success { "[OK]" } else { "[ERR]" };
                            let color = if result.success { Color::Green } else { Color::Red };
                            let text = format!("{} {} - {}", status, result.repo_name, result.message);
                            lines.push(Line::raw(truncate_line(&text, results_area.width as usize))
                                .style(Style::default().fg(color)));
                        }
                    }
                }

                f.render_widget(Paragraph::new(lines), results_area);
            }
        }
        OperationProgress::Single { .. } => {
            // Indeterminate progress - animated spinner (braille dots)
            let spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"; // Braille spinner
            let frame = if let Some(start) = app.animation_start {
                (start.elapsed().as_millis() / 80) as usize % spinner.len()
            } else {
                0
            };
            let char = spinner.chars().nth(frame).unwrap_or('⠋');
            let msg = format!("{} Processing...", char);
            let p = Paragraph::new(msg)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center);
            f.render_widget(p, progress_area);

            let single_hint = Line::raw("Running operation...")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(Paragraph::new(single_hint).alignment(Alignment::Center), results_area);
        }
    }

    // 3. Hint text
    let hint_text = match progress {
        OperationProgress::Batch { .. } => {
            if app.batch_receiver.is_some() {
                "j/k or PgUp/PgDn scroll | ESC locked while running"
            } else {
                "j/k or PgUp/PgDn scroll | ESC close"
            }
        }
        OperationProgress::Single { .. } => "Press ESC to cancel",
    };
    let hint = Line::raw(hint_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(hint).alignment(Alignment::Center), hint_area);
}

fn truncate_line(input: &str, max_width: usize) -> String {
    // Use grapheme-safe truncation to handle CJK and combining marks correctly
    if text_utils::display_width(input) <= max_width {
        return input.to_string();
    }

    if max_width <= 3 {
        // For very small spaces, just take as much as displays in max_width columns
        return text_utils::truncate_to_width(input, max_width).to_string();
    }

    // Truncate to make room for ellipsis (...) which is 3 columns wide
    let max_with_ellipsis = max_width.saturating_sub(3);
    let truncated = text_utils::truncate_to_width(input, max_with_ellipsis);
    format!("{}...", truncated)
}
