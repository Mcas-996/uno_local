//! * STAR CARNIVAL LAUNCHER *
//!
//! Cross-platform terminal entry and reliable cleanup.

mod ai;
mod app;
mod card_art;
mod core;
mod environment;
mod frontend;
mod i18n;
mod screen;
mod termwez;
mod uninstall;
mod universal;
mod view;

use std::env;
use std::io::{self, stdout};

use app::App;
use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType, LeaveAlternateScreen, disable_raw_mode};
use environment::{FrontendKind, TerminalEnvironment};
use frontend::GraphicsChoice;
use i18n::Language;

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
        [argument] if matches!(argument.as_str(), "--version" | "-v") => {
            print_version();
            Ok(())
        }
        [argument] if argument == "--uninstall" => uninstall::run(false),
        [argument, confirmation]
            if argument == "--uninstall" && matches!(confirmation.as_str(), "-y" | "--yes") =>
        {
            uninstall::run(true)
        }
        [argument] => Err(format!("unknown argument '{argument}'; run uno --help")),
        _ => Err("uno does not accept positional arguments; run uno --help".to_owned()),
    }
}

fn print_help() {
    println!("uno - local terminal UNO against AI");
    println!();
    println!("Usage: uno [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -h, --help       Print help");
    println!("  -v, --version    Print version");
    println!("      --uninstall  Uninstall a managed UNO installation (-y, --yes to confirm)");
    println!();
    println!("The game runs fully offline. Configure 1-4 AI opponents in the TUI.");
}

fn print_version() {
    println!(
        "uno {} (commit {})",
        env!("CARGO_PKG_VERSION"),
        env!("UNO_GIT_COMMIT")
    );
}

/// 初始化终端、图形运行时和应用状态，并驱动逐帧事件循环。
fn run_tui() -> io::Result<()> {
    install_panic_restore();
    let environment = TerminalEnvironment::detect();
    match environment.frontend() {
        FrontendKind::UniversalText => {
            let mut app = App::with_graphics(Language::detect(), GraphicsChoice::Text);
            let reason = if environment.is_ssh {
                frontend::FallbackReason::Ssh
            } else {
                frontend::FallbackReason::Tmux
            };
            universal::run(&mut app, Some(reason))
        }
        FrontendKind::Universal => {
            let mut app = App::with_graphics(Language::detect(), GraphicsChoice::Text);
            universal::run(&mut app, None)
        }
        FrontendKind::Termwiz => {
            let mut app = App::with_graphics(Language::detect(), GraphicsChoice::GraphicsBeta);
            match termwez::run(&mut app) {
                Ok(()) => Ok(()),
                Err(_) => {
                    app.should_exit = false;
                    app.setup.graphics = GraphicsChoice::Text;
                    universal::run(&mut app, Some(frontend::FallbackReason::Unsupported))
                }
            }
        }
    }
}

fn install_panic_restore() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
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
        assert!(run(vec!["--version".to_owned()]).is_ok());
        assert!(run(vec!["-v".to_owned()]).is_ok());
        assert!(
            run(vec!["host".to_owned()])
                .unwrap_err()
                .contains("unknown argument")
        );
        assert!(run(vec!["join".to_owned(), "share".to_owned()]).is_err());
    }
}
