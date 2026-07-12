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
    if roots.is_empty() {
        roots.push(std::env::current_dir()?);
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

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
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

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
