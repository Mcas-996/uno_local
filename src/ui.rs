//! * STAR CARNIVAL TABLE *
//!
//! GBK-safe text with bright terminal-native Holiday styling.

use crate::core::{Action, Color};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};

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
                .block(carnival_block(app.language.text(Message::Title)))
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
            21,
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
                    "[WIN] * {winner} *\n\n{}",
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
    let outer = centered(area, 62, 18);
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(outer);
    frame.render_widget(
        Paragraph::new(app.language.text(Message::Title))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(TuiColor::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(carnival_border()),
            ),
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
        format!(
            "{}: {}",
            app.language.text(Message::Deck),
            app.language.deck_variant(app.setup.deck_variant)
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Language),
            app.language.name()
        ),
        app.language.text(Message::Start).to_owned(),
    ];
    let items = values.into_iter().enumerate().map(|(index, value)| {
        let prefix = if index == app.setup.selected {
            "> "
        } else {
            "  "
        };
        let style = if index == app.setup.selected {
            Style::default()
                .fg(TuiColor::Black)
                .bg(TuiColor::LightYellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{prefix}{value}")).style(style)
    });
    frame.render_widget(
        List::new(items).block(
            carnival_block(app.language.text(Message::Setup))
                .borders(Borders::LEFT | Borders::RIGHT),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(app.language.text(Message::SetupHint))
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(carnival_border()),
            ),
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
        "{}  |  {}  |  {}: {}  |  {}: {}",
        app.language.text(Message::Title),
        app.language.deck_variant(game.deck_variant()),
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
                    .fg(TuiColor::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(carnival_block("* STAR TABLE *")),
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
        .join("   |   ");
    frame.render_widget(
        Paragraph::new(opponents)
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(carnival_block(app.language.text(Message::Opponents))),
        rows[1],
    );

    let mut discard_line = vec![Span::raw("      [ ")];
    discard_line.extend(styled_card(app.language, state.discard_top, false));
    discard_line.push(Span::raw(" ]"));
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
        Line::from(discard_line),
    ];
    frame.render_widget(
        Paragraph::new(table)
            .alignment(Alignment::Center)
            .block(carnival_block(app.language.text(Message::Table))),
        rows[2],
    );

    let hand_spans = app
        .human_hand()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .flat_map(|(index, card)| {
            let selected = index == app.selected_card;
            let mut spans = vec![Span::styled(
                format!(" {}:[", index + 1),
                selected_style(Style::default().fg(TuiColor::Gray), selected),
            )];
            spans.extend(styled_card(app.language, *card, selected));
            spans.push(Span::styled(
                "] ",
                selected_style(Style::default().fg(TuiColor::Gray), selected),
            ));
            spans.push(Span::raw(" "));
            spans
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(hand_spans))
            .wrap(Wrap { trim: false })
            .block(carnival_block(app.language.text(Message::YourHand))),
        rows[3],
    );

    let log_items = app
        .logs
        .iter()
        .rev()
        .map(|line| ListItem::new(line.as_str()));
    frame.render_widget(
        List::new(log_items).block(carnival_block(app.language.text(Message::EventLog))),
        rows[4],
    );

    let footer = if app.command_mode {
        format!(":{}", app.command)
    } else {
        let hint = game_hint(app);
        if app.status.is_empty() {
            hint
        } else {
            format!("{}  │  {hint}", app.status)
        }
    };
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(carnival_block(if app.command_mode {
                app.language.text(Message::Command)
            } else {
                ""
            })),
        rows[5],
    );
}

fn game_hint(app: &App) -> String {
    let game = app.game.as_ref().expect("game hint has game");
    let mut hints = Vec::new();

    if game.current_player() == &app.human_id
        && let Ok(actions) = game.legal_actions(&app.human_id)
    {
        if actions
            .iter()
            .any(|action| matches!(action, Action::Play { .. }))
        {
            hints.push(app.language.text(Message::PlayHint));
        }
        if actions.iter().any(|action| matches!(action, Action::Draw)) {
            hints.push(app.language.text(Message::DrawHint));
        }
        if actions.iter().any(|action| matches!(action, Action::Pass)) {
            hints.push(app.language.text(Message::PassHint));
        }
    }

    hints.push(app.language.text(Message::GameUtilitiesHint));
    hints.join(" · ")
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
        .block(carnival_block(app.language.text(Message::ChooseColor))),
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
            .block(carnival_block(title)),
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

// ===== * CARD LIGHTS * =====

fn styled_card(
    language: crate::i18n::Language,
    card: crate::core::Card,
    selected: bool,
) -> Vec<Span<'static>> {
    use crate::core::Rank;

    if matches!(card.rank, Rank::WildDrawSixteen) {
        let wild = match language {
            crate::i18n::Language::English => "WILD",
            crate::i18n::Language::Chinese => "变色",
        };
        return vec![
            themed_span("< ", TuiColor::LightYellow, selected),
            themed_span(wild, TuiColor::LightRed, selected),
            themed_span(" +", TuiColor::LightYellow, selected),
            themed_span("1", TuiColor::LightGreen, selected),
            themed_span("6", TuiColor::LightBlue, selected),
            themed_span(" >", TuiColor::LightYellow, selected),
        ];
    }

    let color = match card.rank {
        Rank::DrawEight => card.color.map_or(TuiColor::LightYellow, card_color),
        _ => card.color.map_or(TuiColor::LightMagenta, card_color),
    };
    vec![themed_span(language.card(card), color, selected)]
}

fn themed_span(
    content: impl Into<std::borrow::Cow<'static, str>>,
    color: TuiColor,
    selected: bool,
) -> Span<'static> {
    Span::styled(
        content,
        selected_style(
            Style::default().fg(color).add_modifier(Modifier::BOLD),
            selected,
        ),
    )
}

fn selected_style(style: Style, selected: bool) -> Style {
    if selected {
        style.bg(TuiColor::White).add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn carnival_border() -> Style {
    Style::default()
        .fg(TuiColor::LightYellow)
        .add_modifier(Modifier::BOLD)
}

fn carnival_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(carnival_border())
        .title(title)
        .title_style(
            Style::default()
                .fg(TuiColor::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::core::Action;
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

    fn assert_rounded_corners(terminal: &Terminal<TestBackend>, area: Rect) {
        let buffer = terminal.backend().buffer();
        let right = area.x + area.width - 1;
        let bottom = area.y + area.height - 1;

        assert_eq!(buffer[(area.x, area.y)].symbol(), "╭");
        assert_eq!(buffer[(right, area.y)].symbol(), "╮");
        assert_eq!(buffer[(area.x, bottom)].symbol(), "╰");
        assert_eq!(buffer[(right, bottom)].symbol(), "╯");
    }

    #[test]
    fn setup_game_and_overlay_use_rounded_borders() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert_rounded_corners(&terminal, Rect::new(9, 3, 62, 18));

        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert_rounded_corners(&terminal, Rect::new(0, 0, 100, 3));

        app.screen = Screen::Help;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert_rounded_corners(&terminal, Rect::new(19, 3, 62, 21));
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
        assert!(screen.contains('节'));
        assert!(screen.contains('日'));
        assert!(screen.contains("118"));
    }

    #[test]
    fn setup_renders_language_setting() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);

        terminal.draw(|frame| render(frame, &app)).unwrap();

        assert!(contents(&terminal).contains("Language: English"));
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
    fn game_hint_changes_after_drawing() {
        let backend = TestBackend::new(140, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains("D draw"));
        assert!(!screen.contains("P pass"));

        app.game
            .as_mut()
            .unwrap()
            .apply_action(&app.human_id, Action::Draw)
            .unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(!screen.contains("D draw"));
        assert!(screen.contains("P pass"));
    }

    #[test]
    fn ai_turn_hides_human_action_hints() {
        let backend = TestBackend::new(140, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let game = app.game.as_mut().unwrap();
        game.apply_action(&app.human_id, Action::Draw).unwrap();
        game.apply_action(&app.human_id, Action::Pass).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(!screen.contains("Enter play"));
        assert!(!screen.contains("D draw"));
        assert!(!screen.contains("P pass"));
        assert!(screen.contains("? help · Q quit"));
    }

    #[test]
    fn chinese_game_renders_localized_action_hint() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::Chinese);
        app.setup.bot_count = 1;
        app.start_match().unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains('摸'));
        assert!(screen.contains('牌'));
        assert!(screen.contains('帮'));
        assert!(screen.contains('退'));
    }

    #[test]
    fn small_terminal_shows_resize_message() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(contents(&terminal).contains("Terminal too small"));
    }

    #[test]
    fn holiday_card_styles_keep_penalty_text_and_four_colors() {
        use crate::core::{Card, Rank};

        let draw_eight = styled_card(
            Language::English,
            Card::new(Color::Red, Rank::DrawEight),
            false,
        );
        assert_eq!(draw_eight.len(), 1);
        assert!(draw_eight[0].content.contains("+8"));
        assert_eq!(draw_eight[0].style.fg, Some(TuiColor::Red));

        let wild = styled_card(Language::English, Card::wild(Rank::WildDrawSixteen), false);
        let label = wild
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let colors = wild
            .iter()
            .filter_map(|span| span.style.fg)
            .collect::<Vec<_>>();
        assert!(label.contains("WILD +16"));
        assert!(colors.contains(&TuiColor::LightRed));
        assert!(colors.contains(&TuiColor::LightYellow));
        assert!(colors.contains(&TuiColor::LightGreen));
        assert!(colors.contains(&TuiColor::LightBlue));
    }

    #[test]
    fn standard_setup_variant_is_visible() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.deck_variant = crate::core::DeckVariant::Standard;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains("Standard 108"));
        assert!(screen.contains("STAR CARNIVAL"));
    }

    #[test]
    fn holiday_help_color_picker_and_result_use_themed_copy() {
        use crate::core::{Card, Rank};

        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.screen = Screen::Help;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = contents(&terminal);
        assert!(screen.contains("STAR CARNIVAL"));
        assert!(screen.contains("WILD +16 changes color"));

        app.start_match().unwrap();
        app.pending_wild = Some(Card::wild(Rank::WildDrawSixteen));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(contents(&terminal).contains("Choose a color"));

        app.pending_wild = None;
        app.screen = Screen::Result;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(contents(&terminal).contains("[WIN]"));
    }
}
