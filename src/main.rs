mod app;
mod diffmodel;
mod git;
mod highlight;
mod project;
mod ui;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::App;

fn main() -> Result<()> {
    let mut staged = false;
    let mut scan_dirs: Vec<PathBuf> = Vec::new();
    let mut explicit_dirs: Vec<PathBuf> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--staged" => staged = true,
            "--scan" => {
                let dir = args
                    .next()
                    .ok_or_else(|| anyhow!("--scan requires a directory argument"))?;
                scan_dirs.push(PathBuf::from(dir));
            }
            other => explicit_dirs.push(PathBuf::from(other)),
        }
    }

    let mut roots = Vec::new();
    for scan_dir in &scan_dirs {
        roots.extend(project::discover(scan_dir)?);
    }
    roots.extend(explicit_dirs);

    let defaulted_to_cwd = roots.is_empty();
    if defaulted_to_cwd {
        let cwd = std::env::current_dir()?;
        if !git::is_repo(&cwd) {
            println!("'{}' is not a git repository.", cwd.display());
            println!("If it contains multiple projects, try: dv --scan .");
            return Ok(());
        }
        roots.push(cwd);
    }

    let mut projects = Vec::new();
    for root in &roots {
        if let Some(p) = project::load(root, staged)? {
            projects.push(p);
        }
    }

    if projects.is_empty() {
        println!("No changes to show.");
        return Ok(());
    }

    let mut app = App::new(projects);
    run_tui(&mut app)?;
    Ok(())
}

fn run_tui(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;

    let restore = || -> Result<()> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    };

    std::panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        eprintln!("{info}");
    }));

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, app);

    restore()?;
    result
}

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_space: Option<Instant> = None;

    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.palette_open {
                match key.code {
                    KeyCode::Esc => app.close_palette(),
                    KeyCode::Enter => app.palette_confirm(),
                    KeyCode::Backspace => app.palette_backspace(),
                    KeyCode::Up => app.palette_move(-1),
                    KeyCode::Down => app.palette_move(1),
                    KeyCode::Char(c) => app.palette_type(c),
                    _ => {}
                }
            } else if key.code == KeyCode::Char(' ') {
                let now = Instant::now();
                let is_double_tap =
                    last_space.is_some_and(|prev| now.duration_since(prev) <= DOUBLE_TAP_WINDOW);
                if is_double_tap {
                    app.open_palette();
                    last_space = None;
                } else {
                    last_space = Some(now);
                }
            } else {
                last_space = None;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
                    KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
                    KeyCode::Char('d') => app.scroll_down(15),
                    KeyCode::Char('u') => app.scroll_up(15),
                    KeyCode::Char('n') | KeyCode::Tab | KeyCode::Right => app.next_file(),
                    KeyCode::Char('p') | KeyCode::BackTab | KeyCode::Left => app.prev_file(),
                    KeyCode::Char('}') => app.next_project(),
                    KeyCode::Char('{') => app.prev_project(),
                    KeyCode::Char('g') => app.scroll = 0,
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
