use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
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

    if app.palette_open {
        draw_palette(frame, app);
    }
}

fn draw_files_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .current_project()
        .map(|p| {
            p.files
                .iter()
                .map(|f| ListItem::new(Line::from(f.path.clone())))
                .collect()
        })
        .unwrap_or_default();

    let mut state = ListState::default();
    state.select(Some(app.selected_file));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
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

    let paragraph = Paragraph::new(app.current_rendered().to_vec())
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((app.scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut hint = String::new();
    if app.projects.len() > 1 {
        if let Some(project) = app.current_project() {
            hint.push_str(&project.name);
            hint.push_str("  \u{2022}  ");
        }
        hint.push_str("space space: switch project  \u{2022}  ");
    }
    hint.push_str("q: quit");

    let footer = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

fn draw_palette(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let input = Paragraph::new(format!("> {}", app.palette_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Switch project"),
    );
    frame.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .palette_matches
        .iter()
        .map(|&idx| ListItem::new(app.projects[idx].name.clone()))
        .collect();

    let mut state = ListState::default();
    if !app.palette_matches.is_empty() {
        state.select(Some(app.palette_selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, chunks[1], &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
