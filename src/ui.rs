use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(20)])
        .split(frame.area());

    draw_sidebar(frame, app, chunks[0]);
    draw_diff_pane(frame, app, chunks[1]);
}

fn draw_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|f| ListItem::new(Line::from(f.path.clone())))
        .collect();

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
    let title = app
        .current_file()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "no changes".to_string());

    let paragraph = Paragraph::new(app.current_rendered().to_vec())
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((app.scroll, 0));

    frame.render_widget(paragraph, area);
}
