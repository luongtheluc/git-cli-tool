use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table},
    Frame,
};

use super::app::{App, InputMode, MainView, Panel};
use crate::text_utils;

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

            // Repo name - dynamically sized based on available sidebar width.
            // Reserve space for: checkbox(4) + dirty(4) + branch/tag/time(~30) + borders(2)
            let reserved = 40;
            let max_name_width = if area.width as usize > reserved {
                area.width as usize - reserved
            } else {
                12 // fallback for very narrow terminals
            };
            let display_name = if text_utils::display_width(&r.name) > max_name_width {
                text_utils::truncate_to_width(&r.name, max_name_width - 1).to_string() + "…"
            } else {
                r.name.clone()
            };
            spans.push(Span::styled(
                display_name,
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

            // Git audit severity icon (shown after 'A' audit runs)
            if let Some(audit) = app.audit_cache.get(&i) {
                let (icon, color) = match audit.severity() {
                    "clean"    => ("✓", Color::Green),
                    "warning"  => ("⚠", Color::Yellow),
                    "critical" => ("✗", Color::Red),
                    "behind"   => ("↓", Color::Cyan),
                    _          => ("?", Color::DarkGray),
                };
                spans.push(Span::styled(format!(" {}", icon), Style::default().fg(color)));
            }

            // Npm audit icon (shown after 'N' audit runs); N✓/N✗/N⚠
            if let Some(npm) = app.npm_audit_cache.get(&i) {
                let (icon, color) = if npm.error.is_some() {
                    ("N?", Color::DarkGray)
                } else if npm.is_vulnerable() {
                    ("N✗", Color::Red)
                } else if !npm.is_clean() {
                    ("N⚠", Color::Yellow)
                } else {
                    ("N✓", Color::Green)
                };
                spans.push(Span::styled(format!(" {}", icon), Style::default().fg(color)));
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

/// Right main panel: routes to status, git audit, or npm audit view.
fn render_main(f: &mut Frame, app: &App, area: Rect) {
    match app.main_view {
        MainView::Status   => render_status_panel(f, app, area),
        MainView::Audit    => render_audit_view(f, app, area),
        MainView::NpmAudit => render_npm_audit_view(f, app, area),
    }
}

/// Show full audit table for all repos (populated after 'A' audit).
fn render_audit_view(f: &mut Frame, app: &App, area: Rect) {
    let border_style = panel_border_style(app.focused_panel == Panel::Main);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " AUDIT ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    if app.audit_cache.is_empty() {
        let p = Paragraph::new("  No audit data — press A to run")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    let mut sorted_indices: Vec<usize> = app.audit_cache.keys().cloned().collect();
    sorted_indices.sort();

    let rows: Vec<Row> = sorted_indices
        .iter()
        .filter_map(|&idx| {
            let audit = app.audit_cache.get(&idx)?;
            let repo_name = app.repos.get(idx).map(|r| r.name.as_str()).unwrap_or("?");
            let uncommitted = if audit.uncommitted > 0 {
                format!("✗ {}", audit.uncommitted)
            } else {
                "✓".to_string()
            };
            let unpushed = if audit.unpushed > 0 { format!("↑ {}", audit.unpushed) } else { "—".to_string() };
            let behind   = if audit.behind   > 0 { format!("↓ {}", audit.behind)   } else { "—".to_string() };
            let status   = match audit.severity() {
                "critical" => "✗ critical",
                "warning"  => "⚠ warning",
                "behind"   => "↓ behind",
                _          => "✓ clean",
            };
            let color = match audit.severity() {
                "critical" => Color::Red,
                "warning"  => Color::Yellow,
                "behind"   => Color::Cyan,
                _          => Color::Green,
            };
            let style = Style::default().fg(color);
            Some(Row::new(vec![
                Cell::from(repo_name.to_string()).style(style),
                Cell::from(audit.branch.clone()).style(style),
                Cell::from(uncommitted).style(style),
                Cell::from(unpushed).style(style),
                Cell::from(behind).style(style),
                Cell::from(status).style(style),
            ]))
        })
        .collect();

    let header = Row::new(vec!["Repo", "Branch", "Uncommitted", "Unpushed", "Behind", "Status"])
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(22),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(13),
            Constraint::Percentage(13),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

/// npm/yarn vulnerability audit table (populated after 'N' audit).
/// Shows ALL repos: npm/yarn ones with vuln data, non-npm ones as dimmed "skipped" rows.
fn render_npm_audit_view(f: &mut Frame, app: &App, area: Rect) {
    let border_style = panel_border_style(app.focused_panel == Panel::Main);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " NPM AUDIT ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    // Audit was run but no npm/yarn repos exist in workspace
    if app.npm_audit_cache.is_empty() {
        let p = Paragraph::new("  No npm/yarn projects found in workspace")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    // Show ALL repos: npm/yarn ones with data, others as skipped
    let rows: Vec<Row> = app
        .repos
        .iter()
        .enumerate()
        .map(|(idx, repo)| {
            if let Some(result) = app.npm_audit_cache.get(&idx) {
                let tool = if result.has_yarn { "yarn" } else { "npm" };

                // Error row: tool not found or parse failure
                if let Some(ref e) = result.error {
                    let style = Style::default().fg(Color::Red);
                    return Row::new(vec![
                        Cell::from(repo.name.clone()).style(style),
                        Cell::from(tool).style(style),
                        Cell::from("").style(style),
                        Cell::from("").style(style),
                        Cell::from("").style(style),
                        Cell::from("").style(style),
                        Cell::from(format!("⚠ {}", e)).style(style),
                    ]);
                }

                let crit_s = if result.critical > 0 { format!("✗ {}", result.critical) } else { "—".to_string() };
                let high_s = if result.high > 0     { format!("✗ {}", result.high) }     else { "—".to_string() };
                let med_s  = if result.moderate > 0 { result.moderate.to_string() }      else { "—".to_string() };
                let low_s  = if result.low > 0      { result.low.to_string() }           else { "—".to_string() };
                let status = if result.is_vulnerable() { "✗ vulnerable" }
                             else if result.is_clean()  { "✓ clean" }
                             else                        { "⚠ low risk" };
                let color  = if result.is_vulnerable() { Color::Red }
                             else if result.is_clean()  { Color::Green }
                             else                        { Color::Yellow };
                let style = Style::default().fg(color);
                Row::new(vec![
                    Cell::from(repo.name.clone()).style(style),
                    Cell::from(tool).style(style),
                    Cell::from(crit_s).style(style),
                    Cell::from(high_s).style(style),
                    Cell::from(med_s).style(style),
                    Cell::from(low_s).style(style),
                    Cell::from(status).style(style),
                ])
            } else {
                // Not an npm/yarn repo — show dimmed skipped row
                let style = Style::default().fg(Color::DarkGray);
                Row::new(vec![
                    Cell::from(repo.name.clone()).style(style),
                    Cell::from("—").style(style),
                    Cell::from("").style(style),
                    Cell::from("").style(style),
                    Cell::from("").style(style),
                    Cell::from("").style(style),
                    Cell::from("skipped").style(style),
                ])
            }
        })
        .collect();

    let header = Row::new(vec!["Repo", "Tool", "Critical", "High", "Medium", "Low", "Status"])
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(8),
            Constraint::Percentage(12),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

/// Git status of the selected repo.
fn render_status_panel(f: &mut Frame, app: &App, area: Rect) {
    let active = app.focused_panel == Panel::Main;
    let border_style = panel_border_style(active);

    let title = app
        .repos
        .get(app.selected)
        .map(|r| {
            // Format title, dynamically sized to available panel width
            let title_text = format!(" {} [{}] ", r.name, r.branch);
            let max_title_width = (area.width as usize).saturating_sub(4).max(10);
            if text_utils::display_width(&title_text) > max_title_width {
                text_utils::truncate_to_width(&title_text, max_title_width - 1).to_string() + "…"
            } else {
                title_text
            }
        })
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
            // Dynamically size file path based on panel width.
            // Reserve for: border(2) + status code(4) + label(~12)
            let reserved_path = 18;
            let max_path_width = if area.width as usize > reserved_path {
                area.width as usize - reserved_path
            } else {
                28
            };
            let display_path = if text_utils::display_width(&e.path) > max_path_width {
                text_utils::truncate_to_width(&e.path, max_path_width - 1).to_string() + "…"
            } else {
                e.path.clone()
            };
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", e.code),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", e.label()),
                    Style::default().fg(color),
                ),
                Span::styled(display_path, Style::default().fg(Color::White)),
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
        hint(" A "), Span::raw("Audit "),
        hint(" N "), Span::raw("Npm audit "),
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
