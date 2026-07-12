mod app;
mod diffmodel;
mod git;
mod highlight;
mod ui;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;

use app::App;

fn main() -> Result<()> {
    let staged = std::env::args().any(|a| a == "--staged");

    let diff_text = git::diff(staged)?;
    let files = diffmodel::parse(&diff_text);

    if files.is_empty() {
        println!("No changes to show.");
        return Ok(());
    }

    let mut app = App::new(files);
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
