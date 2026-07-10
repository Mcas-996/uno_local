use crate::core::Color;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Screen};
use crate::i18n::Message;

pub const MIN_WIDTH: u16 = 70;
pub const MIN_HEIGHT: u16 = 22;

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(app.language.text(Message::TooSmall))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(app.language.text(Message::Title)),
                )
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    if app.game.is_some() {
        render_game(frame, app, area);
    } else {
        render_setup(frame, app, area);
    }

    match app.screen {
        Screen::Help => render_overlay(
            frame,
            area,
            62,
            17,
            app.language.text(Message::Help),
            app.language.text(Message::HelpBody),
        ),
        Screen::QuitConfirm => render_overlay(
            frame,
            area,
            42,
            7,
            app.language.text(Message::QuitTitle),
            app.language.text(Message::QuitBody),
        ),
        Screen::Result => {
            let state = app.game.as_ref().expect("result has game").public_state();
            let winner = state
                .winner
                .and_then(|id| {
                    state
                        .players
                        .into_iter()
                        .find(|player| player.id == id)
                        .map(|player| player.name)
                })
                .unwrap_or_default();
            render_overlay(
                frame,
                area,
                46,
                8,
                app.language.text(Message::Winner),
                &format!(
                    "🏆 {winner}\n\n{}",
                    app.language.text(Message::NewMatchHint)
                ),
            );
        }
        Screen::Setup | Screen::Game => {}
    }

    if app.pending_wild.is_some() && app.screen == Screen::Game {
        render_color_picker(frame, app, area);
    }
}

fn render_setup(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let outer = centered(area, 58, 16);
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(outer);
    frame.render_widget(
        Paragraph::new(app.language.text(Message::Title))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(TuiColor::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)),
        rows[0],
    );

    let values = [
        format!(
            "{}: {}",
            app.language.text(Message::PlayerName),
            app.setup.name
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Bots),
            app.setup.bot_count
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Difficulty),
            app.language.difficulty(app.setup.difficulty)
        ),
        app.language.text(Message::Start).to_owned(),
    ];
    let items = values.into_iter().enumerate().map(|(index, value)| {
        let prefix = if index == app.setup.selected {
            "▶ "
        } else {
            "  "
        };
        let style = if index == app.setup.selected {
            Style::default()
                .fg(TuiColor::Black)
                .bg(TuiColor::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{prefix}{value}")).style(style)
    });
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .title(app.language.text(Message::Setup)),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(app.language.text(Message::SetupHint))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)),
        rows[2],
    );
}

fn render_game(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let game = app.game.as_ref().expect("game view has game");
    let state = game.public_state();
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let current_name = state
        .players
        .iter()
        .find(|player| player.id == state.current_player)
        .map(|player| player.name.as_str())
        .unwrap_or("?");
    let header = format!(
        "{}  │  {}: {}  │  {}: {}",
        app.language.text(Message::Title),
        app.language.text(Message::Turn),
        current_name,
        app.language.text(Message::Direction),
        app.language.direction(state.direction)
    );
    frame.render_widget(
        Paragraph::new(header)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(TuiColor::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL)),
        rows[0],
    );

    let opponents = state
        .players
        .iter()
        .filter(|player| player.id != app.human_id)
        .map(|player| {
            format!(
                "{}: {} {}",
                player.name,
                player.hand_len,
                app.language.text(Message::Cards)
            )
        })
        .collect::<Vec<_>>()
        .join("   │   ");
    frame.render_widget(
        Paragraph::new(opponents)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.language.text(Message::Opponents)),
            ),
        rows[1],
    );

    let table = vec![
        Line::from(vec![
            Span::raw(format!("{}: ", app.language.text(Message::ActiveColor))),
            Span::styled(
                app.language.color(state.active_color),
                Style::default()
                    .fg(card_color(state.active_color))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(
                format!("[ {} ]", app.language.card(state.discard_top)),
                Style::default()
                    .fg(state
                        .discard_top
                        .color
                        .map_or(TuiColor::Magenta, card_color))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(table).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.language.text(Message::Table)),
        ),
        rows[2],
    );

    let hand_spans = app
        .human_hand()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .flat_map(|(index, card)| {
            let mut style = Style::default().fg(card.color.map_or(TuiColor::Magenta, card_color));
            if index == app.selected_card {
                style = style.bg(TuiColor::White).add_modifier(Modifier::BOLD);
            }
            [
                Span::styled(
                    format!(" {}:[{}] ", index + 1, app.language.card(*card)),
                    style,
                ),
                Span::raw(" "),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(hand_spans))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.language.text(Message::YourHand)),
            ),
        rows[3],
    );

    let log_items = app
        .logs
        .iter()
        .rev()
        .map(|line| ListItem::new(line.as_str()));
    frame.render_widget(
        List::new(log_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.language.text(Message::EventLog)),
        ),
        rows[4],
    );

    let footer = if app.command_mode {
        format!(":{}", app.command)
    } else {
        format!("{}\n{}", app.status, app.language.text(Message::GameHint))
    };
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .title(if app.command_mode {
                    app.language.text(Message::Command)
                } else {
                    ""
                }),
        ),
        rows[5],
    );
}

fn render_color_picker(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let popup = centered(area, 52, 7);
    frame.render_widget(Clear, popup);
    let spans = Color::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(index, color)| {
            let mut style = Style::default().fg(card_color(color));
            if index == app.selected_color {
                style = style.bg(TuiColor::White).add_modifier(Modifier::BOLD);
            }
            [
                Span::styled(format!(" {} ", app.language.color(color)), style),
                Span::raw("  "),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(spans),
            Line::from(app.language.text(Message::ColorHint)),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.language.text(Message::ChooseColor)),
        ),
        popup,
    );
}

fn render_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    width: u16,
    height: u16,
    title: &str,
    body: &str,
) {
    let popup = centered(area, width, height);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title)),
        popup,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn card_color(color: Color) -> TuiColor {
    match color {
        Color::Red => TuiColor::Red,
        Color::Yellow => TuiColor::Yellow,
        Color::Green => TuiColor::Green,
        Color::Blue => TuiColor::Blue,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::i18n::Language;

    fn contents(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn setup_renders_in_chinese() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::Chinese);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains('新'));
        assert!(screen.contains('电'));
    }

    #[test]
    fn game_renders_without_ai_hands() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains("Your hand"));
        assert!(screen.contains("AI 1: 7 cards"));
        assert!(!screen.contains("AI 1 hand"));
    }

    #[test]
    fn small_terminal_shows_resize_message() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(contents(&terminal).contains("Terminal too small"));
    }
}
