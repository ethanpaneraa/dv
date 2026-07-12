use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as RLine, Span};

use crate::diffmodel::{FileDiff, LineKind};
use crate::highlight::Highlighter;
use crate::project::Project;

const ADDED_BG: Color = Color::Rgb(20, 40, 22);
const REMOVED_BG: Color = Color::Rgb(45, 20, 22);

pub struct ProjectView {
    pub name: String,
    pub files: Vec<FileDiff>,
    pub rendered: Vec<Vec<RLine<'static>>>,
}

pub struct App {
    pub projects: Vec<ProjectView>,
    pub selected_project: usize,
    pub selected_file: usize,
    pub scroll: u16,
    pub should_quit: bool,
}

impl App {
    pub fn new(projects: Vec<Project>) -> Self {
        let highlighter = Highlighter::new();
        let projects = projects
            .into_iter()
            .map(|p| {
                let rendered = p
                    .files
                    .iter()
                    .map(|f| render_file(f, &highlighter))
                    .collect();
                ProjectView {
                    name: p.name,
                    files: p.files,
                    rendered,
                }
            })
            .collect();

        Self {
            projects,
            selected_project: 0,
            selected_file: 0,
            scroll: 0,
            should_quit: false,
        }
    }

    pub fn current_project(&self) -> Option<&ProjectView> {
        self.projects.get(self.selected_project)
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.current_project()?.files.get(self.selected_file)
    }

    pub fn current_rendered(&self) -> &[RLine<'static>] {
        self.current_project()
            .and_then(|p| p.rendered.get(self.selected_file))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn next_file(&mut self) {
        let Some(project) = self.current_project() else {
            return;
        };
        if self.selected_file + 1 < project.files.len() {
            self.selected_file += 1;
            self.scroll = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if self.selected_file > 0 {
            self.selected_file -= 1;
            self.scroll = 0;
        }
    }

    pub fn next_project(&mut self) {
        if self.selected_project + 1 < self.projects.len() {
            self.selected_project += 1;
            self.selected_file = 0;
            self.scroll = 0;
        }
    }

    pub fn prev_project(&mut self) {
        if self.selected_project > 0 {
            self.selected_project -= 1;
            self.selected_file = 0;
            self.scroll = 0;
        }
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

fn render_file(file: &FileDiff, highlighter: &Highlighter) -> Vec<RLine<'static>> {
    let syntax = highlighter.syntax_for_path(&file.path);
    let mut hl = highlighter.line_highlighter(syntax);
    let mut lines = Vec::new();

    for hunk in &file.hunks {
        lines.push(RLine::from(Span::styled(
            hunk.header.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        for line in &hunk.lines {
            let (marker, marker_style, bg) = match line.kind {
                LineKind::Added => ("+", Style::default().fg(Color::Green), Some(ADDED_BG)),
                LineKind::Removed => ("-", Style::default().fg(Color::Red), Some(REMOVED_BG)),
                LineKind::Context => (" ", Style::default(), None),
            };

            let gutter = format!(
                "{:>4} {:>4} ",
                line.old_lineno.map(|n| n.to_string()).unwrap_or_default(),
                line.new_lineno.map(|n| n.to_string()).unwrap_or_default(),
            );

            let mut spans = vec![
                Span::styled(gutter, Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{marker} "), marker_style),
            ];

            for span in highlighter.highlight(&mut hl, &line.content) {
                let mut style = span.style;
                if let Some(bg) = bg {
                    style = style.bg(bg);
                }
                spans.push(Span::styled(span.content.into_owned(), style));
            }

            lines.push(RLine::from(spans));
        }
    }

    lines
}
