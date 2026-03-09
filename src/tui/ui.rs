use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::app::{App, InputMode, Panel};

/// Top-level render function — called every frame.
/// Splits the terminal into three zones: sidebar | main panel | footer.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.size();

    // Vertical split: content area + fixed-height footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Horizontal split: wider sidebar (45%) + main panel (55%)
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[0]);

    render_sidebar(f, app, top[0]);
    render_main(f, app, top[1]);
    render_footer(f, app, outer[1]);

    // Render modal dialog overlay if operation in progress
    super::modal::render_progress_modal(f, app);
}

/// Left sidebar: scrollable list of repos with enriched info.
/// Format: [x] ● 3  repo-name  main ↑2↓0  v1.0  2h ago
fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let active = app.focused_panel == Panel::Sidebar;
    let border_style = panel_border_style(active);

    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut spans = Vec::new();

            // Checkbox
            let (cb, cb_color) = if app.checked[i] {
                ("[x]", Color::Cyan)
            } else {
                ("[ ]", Color::DarkGray)
            };
            spans.push(Span::styled(
                format!("{} ", cb),
                Style::default().fg(cb_color),
            ));

            // Dirty indicator + change count
            if r.has_uncommitted_changes {
                spans.push(Span::styled("● ", Style::default().fg(Color::Yellow)));
                if r.changed_file_count > 0 {
                    spans.push(Span::styled(
                        format!("{} ", r.changed_file_count),
                        Style::default().fg(Color::Yellow),
                    ));
                }
            } else {
                spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
            }

            // Repo name
            spans.push(Span::styled(
                r.name.clone(),
                Style::default().fg(Color::White),
            ));

            // Branch + ahead/behind
            let mut branch_info = format!("  {}", r.branch);
            if r.ahead > 0 || r.behind > 0 {
                branch_info.push_str(&format!(" ↑{}↓{}", r.ahead, r.behind));
            }
            spans.push(Span::styled(branch_info, Style::default().fg(Color::Cyan)));

            // Latest tag
            if r.latest_tag != "—" {
                spans.push(Span::styled(
                    format!("  {}", r.latest_tag),
                    Style::default().fg(Color::Magenta),
                ));
            }

            // Relative commit time
            if r.last_commit_time != "—" {
                spans.push(Span::styled(
                    format!("  {}", r.last_commit_time),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Count checked repos for title
    let checked_count = app.checked.iter().filter(|&&c| c).count();
    let title = if checked_count > 0 {
        format!(" REPOS ({} selected) ", checked_count)
    } else {
        " REPOS ".to_string()
    };

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}

/// Right main panel: git status of the selected repo.
fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let active = app.focused_panel == Panel::Main;
    let border_style = panel_border_style(active);

    let title = app
        .repos
        .get(app.selected)
        .map(|r| format!(" {} [{}] ", r.name, r.branch))
        .unwrap_or_else(|| " STATUS ".into());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let entries = app.current_status();
    if entries.is_empty() {
        let msg = if app.repos.get(app.selected).is_some() {
            "  ✓  Nothing to commit, working tree clean"
        } else {
            "  No repository selected"
        };
        let p = Paragraph::new(msg)
            .block(block)
            .style(Style::default().fg(Color::Green));
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let color = e.status_color();
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", e.code),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", e.label()),
                    Style::default().fg(color),
                ),
                Span::styled(e.path.clone(), Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// Footer: keyboard hints, input field, or progress bar depending on mode.
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    // Progress bar takes priority
    if let Some(ref progress) = app.operation_progress {
        let pct = progress.progress_pct();
        let filled = (pct * 20.0) as usize;
        let bar = "█".repeat(filled) + &"░".repeat(20 - filled);
        let text = if let Some((completed, total)) = progress.batch_progress() {
            format!(
                " {} {}/{} {}",
                progress.op_name(),
                completed,
                total,
                bar
            )
        } else {
            format!(" {} {}", progress.op_name(), bar)
        };
        let p = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(p, area);
        return;
    }

    // Input mode: show prompt + buffer + cursor
    if app.input_mode == InputMode::Input {
        let prompt = match app.input_purpose {
            super::app::InputPurpose::CommitMessage => "COMMIT MSG: ",
            super::app::InputPurpose::BranchName => "BRANCH: ",
            super::app::InputPurpose::GitCommand => "GIT ARGS: ",
            super::app::InputPurpose::None => "> ",
        };
        let line = Line::from(vec![
            Span::styled(
                prompt,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.input_buffer, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]);
        let p = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(p, area);
        return;
    }

    // Normal mode: keyboard hints + status message
    let msg = app.message.as_deref().unwrap_or("Ready");
    let line = Line::from(vec![
        hint(" ↑↓ "), Span::raw("Nav "),
        hint("Spc"), Span::raw("Sel "),
        hint(" a "), Span::raw("All "),
        hint(" p "), Span::raw("Pull "),
        hint(" P "), Span::raw("Push "),
        hint(" f "), Span::raw("Fetch "),
        hint(" c "), Span::raw("Commit "),
        hint(" b "), Span::raw("Branch "),
        hint(" g "), Span::raw("Git args "),
        hint(" r "), Span::raw("Refresh "),
        hint(" q "), Span::raw("Quit"),
        Span::styled(
            format!("  │ {}", msg),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let p = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(p, area);
}

/// Cyan border when panel is active, dim when inactive.
fn panel_border_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Styled keyboard hint badge.
fn hint(label: &str) -> Span<'_> {
    Span::styled(
        label,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}
