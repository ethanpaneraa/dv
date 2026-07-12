use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as RLine, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, ProjectView, Screen};
use crate::diffmodel;

const LOGO: &str = "\
██████╗ ██╗   ██╗
██╔══██╗██║   ██║
██║  ██║██║   ██║
██║  ██║╚██╗ ██╔╝
██████╔╝ ╚████╔╝
╚═════╝   ╚═══╝  ";

// The one accent color tying selection, the always-primary Diff pane, and key hints
// together, instead of every border/highlight picking its own color.
const ACCENT: Color = Color::Rgb(97, 175, 239);
const SELECTED_BG: Color = Color::Rgb(40, 40, 60);
const ADDED_FG: Color = Color::Green;
const REMOVED_FG: Color = Color::Red;
const DIM: Color = Color::DarkGray;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Home => draw_home(frame, app),
        Screen::Diff => draw_diff_screen(frame, app),
    }
}

fn draw_home(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let logo = Paragraph::new(LOGO).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(logo, chunks[0]);

    draw_stat_line(frame, app, chunks[1]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(20)])
        .split(chunks[2]);

    draw_project_list(frame, app, main[0]);
    draw_preview(frame, app, main[1]);
    draw_home_footer(frame, chunks[3]);
}

fn draw_stat_line(frame: &mut Frame, app: &App, area: Rect) {
    if app.projects.is_empty() {
        let line = Paragraph::new("No projects with changes found here.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(DIM));
        frame.render_widget(line, area);
        return;
    }

    let (files, added, removed) = app
        .projects
        .iter()
        .fold((0usize, 0usize, 0usize), |(f, a, r), p| {
            (f + p.files.len(), a + p.added, r + p.removed)
        });
    let project_noun = if app.projects.len() == 1 {
        "project"
    } else {
        "projects"
    };
    let file_noun = if files == 1 { "file" } else { "files" };

    let line = RLine::from(vec![
        Span::styled(
            format!("{} {project_noun} with changes", app.projects.len()),
            Style::default().fg(DIM),
        ),
        Span::styled("  \u{2022}  ", Style::default().fg(DIM)),
        Span::styled(format!("{files} {file_noun}"), Style::default().fg(DIM)),
        Span::styled("  \u{2022}  ", Style::default().fg(DIM)),
        Span::styled(format!("+{added} "), Style::default().fg(ADDED_FG)),
        Span::styled(format!("-{removed}"), Style::default().fg(REMOVED_FG)),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn draw_project_list(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title("Projects");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let input = Paragraph::new(format!("> {}", app.query)).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input, inner_chunks[0]);

    if app.projects.is_empty() {
        return;
    }

    let width = inner_chunks[1].width;
    let items: Vec<ListItem> = app
        .matches
        .iter()
        .map(|&idx| {
            let p = &app.projects[idx];
            ListItem::new(stat_row(&p.name, p.added, p.removed, width, 2))
        })
        .collect();

    let mut state = ListState::default();
    if !app.matches.is_empty() {
        state.select(Some(app.matched_selected));
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(SELECTED_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, inner_chunks[1], &mut state);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    let selected: Option<&ProjectView> = app
        .matches
        .get(app.matched_selected)
        .map(|&idx| &app.projects[idx]);

    let title = selected.map(|p| p.name.as_str()).unwrap_or("Preview");
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(project) = selected else {
        let empty = Paragraph::new("No project selected.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(DIM));
        frame.render_widget(empty, inner);
        return;
    };

    let width = inner.width;
    let items: Vec<ListItem> = project
        .files
        .iter()
        .map(|f| {
            let (added, removed) = diffmodel::file_stats(f);
            ListItem::new(stat_row(&f.path, added, removed, width, 0))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

/// Builds a `name ... +N -M` row, right-aligning the stats to `width` columns.
/// `reserve` accounts for a highlight_symbol prefix on lists that use one.
fn stat_row(
    name: &str,
    added: usize,
    removed: usize,
    width: u16,
    reserve: usize,
) -> RLine<'static> {
    let stats = format!("+{added} -{removed}");
    let used = name.len() + stats.len() + reserve;
    let pad = " ".repeat((width as usize).saturating_sub(used).max(1));

    RLine::from(vec![
        Span::raw(name.to_string()),
        Span::raw(pad),
        Span::styled(format!("+{added} "), Style::default().fg(ADDED_FG)),
        Span::styled(format!("-{removed}"), Style::default().fg(REMOVED_FG)),
    ])
}

fn draw_home_footer(frame: &mut Frame, area: Rect) {
    let line = key_hint_line(&[("enter/\u{2192}", "open"), ("q", "quit")]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn draw_diff_screen(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(20)])
        .split(chunks[0]);

    draw_files_sidebar(frame, app, content[0]);
    draw_diff_pane(frame, app, content[1]);
    draw_footer(frame, app, chunks[1]);
}

fn draw_files_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title("Files");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width;
    let items: Vec<ListItem> = app
        .current_project()
        .map(|p| {
            p.files
                .iter()
                .map(|f| {
                    let (added, removed) = diffmodel::file_stats(f);
                    ListItem::new(stat_row(&f.path, added, removed, width, 2))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut state = ListState::default();
    state.select(Some(app.selected_file));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(SELECTED_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, inner, &mut state);
}

fn draw_diff_pane(frame: &mut Frame, app: &App, area: Rect) {
    let file_title = app
        .current_file()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "no changes".to_string());

    let title = if app.projects.len() > 1 {
        let project = app
            .current_project()
            .map(|p| p.name.as_str())
            .unwrap_or("?");
        format!("{project} \u{2014} {file_title}")
    } else {
        file_title
    };

    // The Diff pane is always the primary, always-live content (scroll keys act on it
    // unconditionally), so it carries the accent color; Files is a subordinate nav rail.
    let paragraph = Paragraph::new(app.current_rendered().to_vec())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(title),
        )
        .scroll((app.scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut hints = Vec::new();
    if app.projects.len() > 1 {
        if let Some(project) = app.current_project() {
            hints.push(Span::styled(project.name.clone(), Style::default().fg(DIM)));
            hints.push(Span::styled("  \u{2022}  ", Style::default().fg(DIM)));
        }
        hints.extend(key_hint_spans(&[
            ("{ }", "switch"),
            ("space space", "all projects"),
        ]));
        hints.push(Span::styled("  \u{2022}  ", Style::default().fg(DIM)));
    }
    hints.extend(key_hint_spans(&[("\u{2190}", "home"), ("q", "quit")]));

    frame.render_widget(Paragraph::new(RLine::from(hints)), area);
}

fn key_hint_spans(pairs: &[(&str, &str)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{2022}  ", Style::default().fg(DIM)));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {desc}"), Style::default().fg(DIM)));
    }
    spans
}

fn key_hint_line(pairs: &[(&str, &str)]) -> RLine<'static> {
    RLine::from(key_hint_spans(pairs))
}
