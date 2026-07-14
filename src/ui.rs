//! * STAR CARNIVAL TABLE *
//!
//! GBK-safe text with bright terminal-native Holiday styling.

use crate::core::{Action, Color};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui_image::Image;

use crate::app::{App, Screen};
use crate::graphics::{GraphicsRuntime, PreviewSlot};
use crate::i18n::Message;

pub const MIN_WIDTH: u16 = 70;
pub const MIN_HEIGHT: u16 = 22;
pub const IMAGE_MIN_HEIGHT: u16 = 26;

const MIN_HAND_HEIGHT: u16 = 5;
const MIN_LOG_HEIGHT: u16 = 3;
const FIXED_GAME_HEIGHT: u16 = 14;

pub fn render(frame: &mut Frame<'_>, app: &App, graphics: &mut GraphicsRuntime) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        graphics.suspend();
        frame.render_widget(
            Paragraph::new(app.language.text(Message::TooSmall))
                .alignment(Alignment::Center)
                .block(carnival_block(app.language.text(Message::Title)))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    let images_visible = should_render_images(
        area,
        app.game.is_some(),
        app.screen,
        app.pending_wild.is_some(),
        graphics.effective_backend(app.setup.graphics),
    );
    if !images_visible {
        graphics.suspend();
    }

    if app.game.is_some() {
        render_game(frame, app, area, graphics, images_visible);
    } else {
        render_setup(frame, app, area, graphics);
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

fn should_render_images(
    area: Rect,
    has_game: bool,
    screen: Screen,
    has_pending_wild: bool,
    backend: crate::graphics::GraphicsBackend,
) -> bool {
    has_game
        && screen == Screen::Game
        && !has_pending_wild
        && area.width >= MIN_WIDTH
        && area.height >= IMAGE_MIN_HEIGHT
        && backend.supports_images()
}

fn render_setup(frame: &mut Frame<'_>, app: &App, area: Rect, graphics: &GraphicsRuntime) {
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
        format!(
            "{}: {}",
            app.language.text(Message::Graphics),
            app.language.graphics(
                app.setup.graphics,
                graphics.effective_backend(app.setup.graphics)
            )
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

fn render_game(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    graphics: &mut GraphicsRuntime,
    images_visible: bool,
) {
    let game = app.game.as_ref().expect("game view has game");
    let state = game.public_state();
    let (hand_lines, selected_hand_row) = hand_lines(
        app.language,
        app.human_hand().unwrap_or_default(),
        app.selected_card,
        area.width.saturating_sub(2) as usize,
    );
    let table_height = if images_visible { 9 } else { 5 };
    let hand_height = hand_height(hand_lines.len(), area.height, images_visible);
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(table_height),
            Constraint::Length(hand_height),
            Constraint::Min(MIN_LOG_HEIGHT),
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

    if images_visible {
        render_image_table(frame, app, &state, rows[2], graphics);
    } else {
        render_text_table(frame, app, &state, rows[2]);
    }

    let visible_hand_rows = rows[3].height.saturating_sub(2) as usize;
    let hand_scroll = hand_scroll(selected_hand_row, visible_hand_rows);
    frame.render_widget(
        Paragraph::new(hand_lines)
            .scroll((u16::try_from(hand_scroll).unwrap_or(u16::MAX), 0))
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

fn hand_lines(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
) -> (Vec<Line<'static>>, usize) {
    let layout = hand_layout(language, hand, selected_card, width);
    (layout.lines, layout.selected_row)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HandCardPosition {
    index: usize,
    row: usize,
    center_twice: usize,
}

struct HandLayout {
    lines: Vec<Line<'static>>,
    positions: Vec<HandCardPosition>,
    selected_row: usize,
}

fn hand_layout(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
) -> HandLayout {
    let mut lines = Vec::new();
    let mut positions = Vec::with_capacity(hand.len());
    let mut current_line = Vec::new();
    let mut current_width = 0;
    let mut selected_row = 0;

    for (index, card) in hand.iter().enumerate() {
        let selected = index == selected_card;
        let mut entry = vec![Span::styled(
            format!(" {}:[", index + 1),
            selected_style(Style::default().fg(TuiColor::Gray), selected),
        )];
        entry.extend(styled_card(language, *card, selected));
        entry.push(Span::styled(
            "]  ",
            selected_style(Style::default().fg(TuiColor::Gray), selected),
        ));
        let entry_width = entry.iter().map(Span::width).sum::<usize>();

        if !current_line.is_empty() && current_width + entry_width > width {
            lines.push(Line::from(current_line));
            current_line = Vec::new();
            current_width = 0;
        }
        let row = lines.len();
        positions.push(HandCardPosition {
            index,
            row,
            center_twice: current_width.saturating_mul(2).saturating_add(entry_width),
        });
        if selected {
            selected_row = row;
        }
        current_line.extend(entry);
        current_width += entry_width;
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    HandLayout {
        lines,
        positions,
        selected_row,
    }
}

pub(crate) fn adjacent_hand_card(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
    row_delta: isize,
) -> usize {
    let layout = hand_layout(language, hand, selected_card, width);
    let Some(current) = layout
        .positions
        .iter()
        .find(|position| position.index == selected_card)
    else {
        return selected_card;
    };
    let Some(target_row) = current.row.checked_add_signed(row_delta) else {
        return selected_card;
    };

    layout
        .positions
        .iter()
        .filter(|position| position.row == target_row)
        .min_by_key(|position| {
            (
                position.center_twice.abs_diff(current.center_twice),
                position.index,
            )
        })
        .map_or(selected_card, |position| position.index)
}

fn hand_height(line_count: usize, area_height: u16, images_visible: bool) -> u16 {
    let desired_height = u16::try_from(line_count)
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .max(MIN_HAND_HEIGHT);
    let fixed_height = FIXED_GAME_HEIGHT + if images_visible { 4 } else { 0 };
    let max_height = area_height
        .saturating_sub(fixed_height + MIN_LOG_HEIGHT)
        .max(MIN_HAND_HEIGHT);
    desired_height.min(max_height)
}

fn render_text_table(
    frame: &mut Frame<'_>,
    app: &App,
    state: &crate::core::PublicGameState,
    area: Rect,
) {
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
        area,
    );
}

fn render_image_table(
    frame: &mut Frame<'_>,
    app: &App,
    state: &crate::core::PublicGameState,
    area: Rect,
    graphics: &mut GraphicsRuntime,
) {
    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let selected_card = app
        .human_hand()
        .and_then(|hand| hand.get(app.selected_card))
        .copied();

    let selected_title = selected_card.map_or_else(
        || app.language.text(Message::SelectedCard).to_owned(),
        |card| {
            format!(
                "{}: {}",
                app.language.text(Message::SelectedCard),
                app.language.card(card)
            )
        },
    );
    let selected_block = carnival_block(&selected_title);
    let selected_inner = selected_block.inner(columns[0]);
    frame.render_widget(selected_block, columns[0]);
    if let Some(card) = selected_card {
        render_card_preview(
            frame,
            graphics,
            PreviewSlot::Selected,
            card,
            selected_inner,
            app,
        );
    } else {
        graphics.clear_slot(PreviewSlot::Selected);
        frame.render_widget(
            Paragraph::new(app.language.text(Message::NoSelectedCard)).alignment(Alignment::Center),
            selected_inner,
        );
    }

    let discard_title = format!(
        "{}: {} · {}: {}",
        app.language.text(Message::DiscardTop),
        app.language.card(state.discard_top),
        app.language.text(Message::ActiveColor),
        app.language.color(state.active_color)
    );
    let discard_block = carnival_block(&discard_title);
    let discard_inner = discard_block.inner(columns[1]);
    frame.render_widget(discard_block, columns[1]);
    render_card_preview(
        frame,
        graphics,
        PreviewSlot::Discard,
        state.discard_top,
        discard_inner,
        app,
    );
}

fn render_card_preview(
    frame: &mut Frame<'_>,
    graphics: &mut GraphicsRuntime,
    slot: PreviewSlot,
    card: crate::core::Card,
    area: Rect,
    app: &App,
) {
    let size = ratatui::layout::Size::new(area.width, area.height);
    if let Some(protocol) = graphics.protocol(slot, card, size) {
        let image_area = centered_image_area(area, protocol.size());
        frame.render_widget(Image::new(protocol), image_area);
    } else {
        frame.render_widget(
            Paragraph::new(app.language.card(card)).alignment(Alignment::Center),
            area,
        );
    }
}

fn centered_image_area(area: Rect, image_size: ratatui::layout::Size) -> Rect {
    let width = image_size.width.min(area.width);
    let height = image_size.height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn hand_scroll(selected_row: usize, visible_rows: usize) -> usize {
    selected_row.saturating_add(1).saturating_sub(visible_rows)
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
    use crate::core::{Action, Card, Rank};
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

    fn draw_text(terminal: &mut Terminal<TestBackend>, app: &App) {
        let mut graphics = GraphicsRuntime::text_for_tests();
        terminal
            .draw(|frame| render(frame, app, &mut graphics))
            .unwrap();
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
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(9, 3, 62, 18));

        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(0, 0, 100, 3));

        app.screen = Screen::Help;
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(19, 3, 62, 21));
    }

    #[test]
    fn setup_renders_in_chinese() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::Chinese);
        draw_text(&mut terminal, &app);
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

        draw_text(&mut terminal, &app);

        assert!(contents(&terminal).contains("Language: English"));
        assert!(contents(&terminal).contains("Graphics: Auto (Text: unsupported)"));
    }

    #[test]
    fn image_mode_is_responsive_and_suspends_for_overlays_or_text() {
        use crate::graphics::{FallbackReason, GraphicsBackend};

        assert!(!should_render_images(
            Rect::new(0, 0, 70, 25),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Iterm2,
        ));
        assert!(should_render_images(
            Rect::new(0, 0, 70, 26),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Sixel,
        ));
        assert!(!should_render_images(
            Rect::new(0, 0, 100, 30),
            true,
            Screen::Help,
            false,
            GraphicsBackend::Kitty,
        ));
        assert!(!should_render_images(
            Rect::new(0, 0, 100, 30),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Text(FallbackReason::Ssh),
        ));
    }

    #[test]
    fn fitted_images_are_centered_inside_their_panels() {
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), ratatui::layout::Size::new(6, 4)),
            Rect::new(17, 8, 6, 4)
        );
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), ratatui::layout::Size::new(30, 20)),
            Rect::new(10, 5, 20, 10)
        );
    }

    #[test]
    fn image_previews_are_created_only_on_large_game_and_cleared_for_overlay() {
        use ratatui_image::picker::ProtocolType;

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let mut graphics = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);

        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Table"));

        terminal.backend_mut().resize(100, 28);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 2);
        let screen = contents(&terminal);
        assert!(screen.contains("Selected"));
        assert!(screen.contains("Discard"));

        terminal.backend_mut().resize(50, 10);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Terminal too small"));

        terminal.backend_mut().resize(100, 28);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 2);

        app.screen = Screen::Help;
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Help"));
    }

    #[test]
    fn game_renders_without_ai_hands() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("Your hand"));
        assert!(screen.contains("AI 1: 7 cards"));
        assert!(!screen.contains("AI 1 hand"));
    }

    #[test]
    fn large_hand_wraps_every_card_and_tracks_the_selected_row() {
        let cards = (0..40)
            .map(|number| Card::new(Color::Red, Rank::Number(number % 10)))
            .collect::<Vec<_>>();

        let (lines, selected_row) = hand_lines(Language::English, &cards, 39, 68);
        let text = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(lines.len() > 3);
        assert_eq!(selected_row, lines.len() - 1);
        assert!(text.contains(" 1:["));
        assert!(text.contains(" 33:["));
        assert!(text.contains(" 40:["));
    }

    #[test]
    fn large_hand_scroll_keeps_the_selected_row_visible() {
        let visible_rows = 3_usize;
        let selected_row = 5_usize;
        let scroll = hand_scroll(selected_row, visible_rows);

        assert_eq!(scroll, 3);
        assert!(selected_row >= scroll);
        assert!(selected_row < scroll + visible_rows);
    }

    #[test]
    fn vertical_hand_navigation_chooses_the_nearest_horizontal_card() {
        let cards = (0..5)
            .map(|number| Card::new(Color::Red, Rank::Number(number)))
            .collect::<Vec<_>>();

        assert_eq!(adjacent_hand_card(Language::English, &cards, 2, 37, 1), 4);
        assert_eq!(adjacent_hand_card(Language::English, &cards, 4, 37, -1), 1);
    }

    #[test]
    fn vertical_hand_navigation_stops_at_the_first_and_last_rows() {
        let cards = (0..5)
            .map(|number| Card::new(Color::Blue, Rank::Number(number)))
            .collect::<Vec<_>>();

        assert_eq!(adjacent_hand_card(Language::English, &cards, 0, 25, -1), 0);
        assert_eq!(adjacent_hand_card(Language::English, &cards, 4, 25, 1), 4);
    }

    #[test]
    fn vertical_hand_navigation_uses_localized_card_widths() {
        let cards = (0..6)
            .map(|number| Card::new(Color::Yellow, Rank::Number(number)))
            .collect::<Vec<_>>();

        let target = adjacent_hand_card(Language::Chinese, &cards, 1, 24, 1);

        assert!(target > 1);
        let layout = hand_layout(Language::Chinese, &cards, target, 24);
        assert_eq!(layout.positions[target].row, 1);
    }

    #[test]
    fn hand_height_expands_without_hiding_the_event_log() {
        assert_eq!(hand_height(1, MIN_HEIGHT, false), MIN_HAND_HEIGHT);
        assert_eq!(hand_height(6, 28, false), 8);
        assert_eq!(hand_height(20, 28, false), 11);
        assert_eq!(hand_height(20, 28, true), 7);
    }

    #[test]
    fn game_hint_changes_after_drawing() {
        let backend = TestBackend::new(140, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();

        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("D draw"));
        assert!(!screen.contains("P pass"));

        app.game
            .as_mut()
            .unwrap()
            .apply_action(&app.human_id, Action::Draw)
            .unwrap();
        draw_text(&mut terminal, &app);
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

        draw_text(&mut terminal, &app);
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

        draw_text(&mut terminal, &app);
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
        draw_text(&mut terminal, &app);
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
        draw_text(&mut terminal, &app);
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
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("STAR CARNIVAL"));
        assert!(screen.contains("WILD +16 changes color"));

        app.start_match().unwrap();
        app.pending_wild = Some(Card::wild(Rank::WildDrawSixteen));
        draw_text(&mut terminal, &app);
        assert!(contents(&terminal).contains("Choose a color"));

        app.pending_wild = None;
        app.screen = Screen::Result;
        draw_text(&mut terminal, &app);
        assert!(contents(&terminal).contains("[WIN]"));
    }
}
