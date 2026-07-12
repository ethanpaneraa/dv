use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, Screen};

const LOGO: &str = "\
██████╗ ██╗   ██╗
██╔══██╗██║   ██║
██║  ██║██║   ██║
██║  ██║╚██╗ ██╔╝
██████╔╝ ╚████╔╝
╚═════╝   ╚═══╝  ";

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
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let logo = Paragraph::new(LOGO).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(logo, chunks[0]);

    let outer = Block::default().borders(Borders::ALL).title("Projects");
    let inner = outer.inner(chunks[1]);
    frame.render_widget(outer, chunks[1]);

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
        let empty = Paragraph::new("No projects with changes found here.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner_chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .matches
            .iter()
            .map(|&idx| ListItem::new(app.projects[idx].name.clone()))
            .collect();

        let mut state = ListState::default();
        if !app.matches.is_empty() {
            state.select(Some(app.matched_selected));
        }

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, inner_chunks[1], &mut state);
    }

    let footer = Paragraph::new("enter: open  \u{2022}  q: quit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
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
        hint.push_str("{ }: switch  \u{2022}  space space: all projects  \u{2022}  ");
    }
    hint.push_str("q: quit");

    let footer = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}
