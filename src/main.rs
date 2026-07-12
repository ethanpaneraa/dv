mod app;
mod diffmodel;
mod git;
mod highlight;
mod project;
mod ui;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::{App, Screen};

fn main() -> Result<()> {
    let mut staged = false;
    let mut scan_dir: Option<PathBuf> = None;
    let mut explicit_path: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--staged" => staged = true,
            "--scan" => {
                let dir = args
                    .next()
                    .ok_or_else(|| anyhow!("--scan requires a directory argument"))?;
                scan_dir = Some(PathBuf::from(dir));
            }
            other => explicit_path = Some(PathBuf::from(other)),
        }
    }

    // An explicit path stays direct and non-interactive-friendly: no discovery, no
    // Home screen. Useful for scripting or a future git-pager integration.
    if let Some(path) = explicit_path {
        let Some(project) = project::load(&path, staged)? else {
            println!("No changes to show.");
            return Ok(());
        };
        let mut app = App::new_direct(project);
        run_tui(&mut app)?;
        return Ok(());
    }

    // No explicit target: auto-discover. `dv` alone finds whatever there is to review
    // without needing --scan spelled out first.
    let discovery_root = match scan_dir {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };

    let mut roots = Vec::new();
    if git::is_repo(&discovery_root) {
        roots.push(discovery_root.clone());
    }
    roots.extend(project::discover(&discovery_root)?);

    let mut projects = Vec::new();
    for root in &roots {
        if let Some(p) = project::load(root, staged)? {
            projects.push(p);
        }
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

            // Raw mode disables SIGINT generation, so without this Ctrl+C would just
            // be swallowed as ordinary input (e.g. typed into the Home filter).
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.quit();
            } else {
                match app.screen {
                    Screen::Home => match key.code {
                        KeyCode::Char('q') if app.query.is_empty() => app.quit(),
                        KeyCode::Enter => app.home_confirm(),
                        KeyCode::Backspace => app.home_backspace(),
                        KeyCode::Up => app.home_move(-1),
                        KeyCode::Down => app.home_move(1),
                        KeyCode::Esc => app.home_clear_query(),
                        KeyCode::Char(c) => app.home_type(c),
                        _ => {}
                    },
                    Screen::Diff => {
                        if key.code == KeyCode::Char(' ') {
                            let now = Instant::now();
                            let is_double_tap = last_space
                                .is_some_and(|prev| now.duration_since(prev) <= DOUBLE_TAP_WINDOW);
                            if is_double_tap {
                                app.go_home();
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
                                KeyCode::Char('n') | KeyCode::Tab | KeyCode::Right => {
                                    app.next_file()
                                }
                                KeyCode::Char('p') | KeyCode::BackTab | KeyCode::Left => {
                                    app.prev_file()
                                }
                                KeyCode::Char('}') => app.next_project(),
                                KeyCode::Char('{') => app.prev_project(),
                                KeyCode::Char('g') => app.scroll = 0,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
