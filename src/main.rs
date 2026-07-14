//! * STAR CARNIVAL LAUNCHER *
//!
//! Cross-platform terminal entry and reliable cleanup.

mod ai;
mod app;
mod card_art;
mod core;
mod graphics;
mod i18n;
mod ui;

use std::env;
use std::io::{self, stdout};
use std::time::Duration;

use app::App;
use crossterm::cursor::Show;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use graphics::GraphicsRuntime;
use i18n::Language;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [] => run_tui().map_err(|error| error.to_string()),
        [argument] if matches!(argument.as_str(), "--help" | "-h" | "help") => {
            print_help();
            Ok(())
        }
        [argument] => Err(format!("unknown argument '{argument}'; run uno --help")),
        _ => Err("uno does not accept positional arguments; run uno --help".to_owned()),
    }
}

fn print_help() {
    println!("uno - local terminal UNO against AI");
    println!();
    println!("Usage: uno");
    println!("       uno --help");
    println!();
    println!("The game runs fully offline. Configure 1-4 AI opponents in the TUI.");
}

fn run_tui() -> io::Result<()> {
    install_panic_restore();
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut graphics = GraphicsRuntime::detect();
    terminal.clear()?;
    let mut app = App::new(Language::detect());

    while !app.should_exit {
        terminal.draw(|frame| ui::render(frame, &app, &mut graphics))?;
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key, terminal.size()?.width);
        }
        app.tick();
    }
    graphics.suspend();
    terminal.clear()?;
    terminal.show_cursor()?;
    Ok(())
}

fn install_panic_restore() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), Clear(ClearType::All), LeaveAlternateScreen, Show);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_unknown_arguments_are_safe() {
        assert!(run(vec!["--help".to_owned()]).is_ok());
        assert!(
            run(vec!["host".to_owned()])
                .unwrap_err()
                .contains("unknown argument")
        );
        assert!(run(vec!["join".to_owned(), "share".to_owned()]).is_err());
    }
}
