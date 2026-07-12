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
    rendered: Option<Vec<Vec<RLine<'static>>>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Diff,
}

pub struct App {
    pub projects: Vec<ProjectView>,
    pub selected_project: usize,
    pub selected_file: usize,
    pub scroll: u16,
    pub should_quit: bool,
    pub screen: Screen,
    pub query: String,
    pub matches: Vec<usize>,
    pub matched_selected: usize,
    // Lazily built on first use -- loading syntect's default syntax/theme sets isn't
    // free, and Home never needs it at all.
    highlighter: Option<Highlighter>,
}

impl App {
    /// Lands on the Home screen so the caller picks a project. Used for the default
    /// (no explicit path) discovery flow, even when only one or zero projects load --
    /// consistent app-like behavior beats skipping straight in when there's exactly one.
    /// Nothing is syntax-highlighted here; Home only needs project names.
    pub fn new(projects: Vec<Project>) -> Self {
        let mut app = Self::from_projects(projects);
        app.recompute_matches();
        app
    }

    /// Skips Home entirely. Used when a single project was named explicitly on the
    /// command line (scripting / git-pager use), where an interactive picker would
    /// just be in the way.
    pub fn new_direct(project: Project) -> Self {
        let mut app = Self::from_projects(vec![project]);
        app.screen = Screen::Diff;
        app.ensure_rendered(0);
        app
    }

    fn from_projects(projects: Vec<Project>) -> Self {
        let projects = projects
            .into_iter()
            .map(|p| ProjectView {
                name: p.name,
                files: p.files,
                rendered: None,
            })
            .collect();

        Self {
            projects,
            selected_project: 0,
            selected_file: 0,
            scroll: 0,
            should_quit: false,
            screen: Screen::Home,
            query: String::new(),
            matches: Vec::new(),
            matched_selected: 0,
            highlighter: None,
        }
    }

    /// Syntax-highlights a project's files on first visit and caches the result.
    /// No-op if already rendered.
    fn ensure_rendered(&mut self, idx: usize) {
        let Some(project) = self.projects.get(idx) else {
            return;
        };
        if project.rendered.is_some() {
            return;
        }

        let highlighter = &*self.highlighter.get_or_insert_with(Highlighter::new);
        let rendered = self.projects[idx]
            .files
            .iter()
            .map(|f| render_file(f, highlighter))
            .collect();
        self.projects[idx].rendered = Some(rendered);
    }

    pub fn current_project(&self) -> Option<&ProjectView> {
        self.projects.get(self.selected_project)
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.current_project()?.files.get(self.selected_file)
    }

    pub fn current_rendered(&self) -> &[RLine<'static>] {
        self.current_project()
            .and_then(|p| p.rendered.as_ref())
            .and_then(|r| r.get(self.selected_file))
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
            self.ensure_rendered(self.selected_project);
        }
    }

    pub fn prev_project(&mut self) {
        if self.selected_project > 0 {
            self.selected_project -= 1;
            self.selected_file = 0;
            self.scroll = 0;
            self.ensure_rendered(self.selected_project);
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

    /// Returns to the full-page project picker, e.g. from a double-tap of Space while
    /// reviewing a diff.
    pub fn go_home(&mut self) {
        self.screen = Screen::Home;
        self.query.clear();
        self.matched_selected = 0;
        self.recompute_matches();
    }

    pub fn home_type(&mut self, c: char) {
        self.query.push(c);
        self.matched_selected = 0;
        self.recompute_matches();
    }

    pub fn home_backspace(&mut self) {
        self.query.pop();
        self.matched_selected = 0;
        self.recompute_matches();
    }

    pub fn home_clear_query(&mut self) {
        self.query.clear();
        self.matched_selected = 0;
        self.recompute_matches();
    }

    pub fn home_move(&mut self, delta: i32) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as i32;
        let idx = (self.matched_selected as i32 + delta).clamp(0, len - 1);
        self.matched_selected = idx as usize;
    }

    pub fn home_confirm(&mut self) {
        if let Some(&project_idx) = self.matches.get(self.matched_selected) {
            self.selected_project = project_idx;
            self.selected_file = 0;
            self.scroll = 0;
            self.screen = Screen::Diff;
            self.ensure_rendered(project_idx);
        }
    }

    fn recompute_matches(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = self
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| query.is_empty() || p.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
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
