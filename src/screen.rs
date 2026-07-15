//! Small retained cell surface shared by the two renderers.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, PlayMode, Screen};
use crate::core::{Card, Color};
use crate::frontend::{GraphicsBackend, GraphicsChoice, Viewport};
use crate::i18n::Message;
use crate::view::{AppView, MIN_COLUMNS, MIN_ROWS, wrap_hand};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiColor {
    #[default]
    Default,
    Black,
    Red,
    Yellow,
    Green,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub fg: UiColor,
    pub bg: UiColor,
    pub bold: bool,
}

impl Style {
    pub const fn fg(fg: UiColor) -> Self {
        Self {
            fg,
            bg: UiColor::Default,
            bold: false,
        }
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        if selected {
            self.bg = UiColor::White;
            self.fg = UiColor::Black;
            self.bold = true;
        }
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cell {
    pub symbol: char,
    pub style: Style,
    pub continuation: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImageSlot {
    Selected,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ImagePlacement {
    pub slot: ImageSlot,
    pub card: Card,
    pub rect: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Canvas {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
    pub images: Vec<ImagePlacement>,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![
                Cell {
                    symbol: ' ',
                    ..Cell::default()
                };
                usize::from(width) * usize::from(height)
            ],
            images: Vec::new(),
        }
    }

    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        (x < self.width && y < self.height)
            .then(|| &self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)])
    }

    fn put(&mut self, x: u16, y: u16, symbol: char, style: Style, continuation: bool) {
        if x < self.width && y < self.height {
            self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)] = Cell {
                symbol,
                style,
                continuation,
            };
        }
    }

    pub fn text(&mut self, mut x: u16, y: u16, text: &str, style: Style) {
        for symbol in text.chars() {
            let width = UnicodeWidthChar::width(symbol).unwrap_or(0);
            if width == 0 {
                continue;
            }
            self.put(x, y, symbol, style, false);
            if width == 2 {
                self.put(x.saturating_add(1), y, ' ', style, true);
            }
            x = x.saturating_add(width as u16);
            if x >= self.width {
                break;
            }
        }
    }

    pub fn centered_text(&mut self, y: u16, text: &str, style: Style) {
        let width = UnicodeWidthStr::width(text) as u16;
        self.text(self.width.saturating_sub(width) / 2, y, text, style);
    }

    fn clear(&mut self, rect: Rect) {
        let max_x = rect.x.saturating_add(rect.width).min(self.width);
        let max_y = rect.y.saturating_add(rect.height).min(self.height);
        for y in rect.y..max_y {
            for x in rect.x..max_x {
                self.put(x, y, ' ', Style::default(), false);
            }
        }
    }

    pub fn border(&mut self, rect: Rect, title: &str) {
        if rect.width < 2 || rect.height < 2 {
            return;
        }
        let style = Style {
            fg: UiColor::Yellow,
            bold: true,
            ..Style::default()
        };
        for x in rect.x..rect.x.saturating_add(rect.width) {
            self.put(
                x,
                rect.y,
                if x == rect.x || x + 1 == rect.x + rect.width {
                    '+'
                } else {
                    '-'
                },
                style,
                false,
            );
            self.put(
                x,
                rect.y + rect.height - 1,
                if x == rect.x || x + 1 == rect.x + rect.width {
                    '+'
                } else {
                    '-'
                },
                style,
                false,
            );
        }
        for y in rect.y + 1..rect.y + rect.height - 1 {
            self.put(rect.x, y, '|', style, false);
            self.put(rect.x + rect.width - 1, y, '|', style, false);
        }
        if !title.is_empty() {
            self.text(
                rect.x + 2,
                rect.y,
                title,
                Style {
                    fg: UiColor::Magenta,
                    bold: true,
                    ..Style::default()
                },
            );
        }
    }

    #[cfg(test)]
    pub fn plain_text(&self) -> String {
        let mut output = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell(x, y).unwrap();
                if !cell.continuation {
                    output.push(cell.symbol);
                }
            }
            output.push('\n');
        }
        output
    }
}

fn color(color: Color) -> UiColor {
    match color {
        Color::Red => UiColor::Red,
        Color::Yellow => UiColor::Yellow,
        Color::Green => UiColor::Green,
        Color::Blue => UiColor::Blue,
    }
}

pub fn render(app: &App, backend: GraphicsBackend, viewport: Viewport) -> Canvas {
    let Viewport {
        columns: width,
        rows: height,
    } = viewport;
    let view = AppView::new(
        app,
        app.setup.graphics == GraphicsChoice::GraphicsBeta && backend.supports_images(),
    );
    let mut canvas = Canvas::new(width, height);
    if width < MIN_COLUMNS || height < MIN_ROWS {
        canvas.centered_text(
            height / 2,
            app.language.text(Message::TooSmall),
            Style::fg(UiColor::Yellow),
        );
        return canvas;
    }
    match view.screen {
        Screen::Setup => render_setup(&mut canvas, app, backend),
        Screen::Game => render_game(&mut canvas, app, view.images_allowed),
        Screen::Help => render_overlay(
            &mut canvas,
            app.language.text(Message::Help),
            app.language.help_body(app.setup.mode),
        ),
        Screen::Result => render_result(&mut canvas, app),
        Screen::QuitConfirm => render_overlay(
            &mut canvas,
            app.language.text(Message::QuitTitle),
            app.language.text(Message::QuitBody),
        ),
    }
    if app.pending_wild.is_some() {
        let colors = Color::ALL
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                if index == app.selected_color {
                    format!("[{}]", app.language.color(value))
                } else {
                    app.language.color(value).to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        let player_index = app.pending_wild.map_or(0, |pending| pending.player_index);
        render_compact_overlay(
            &mut canvas,
            app.language.text(Message::ChooseColor),
            &format!(
                "{colors}\n{}",
                app.language.color_hint(app.setup.mode, player_index)
            ),
        );
    }
    canvas
}

fn render_setup(canvas: &mut Canvas, app: &App, backend: GraphicsBackend) {
    let rect = Rect {
        x: (canvas.width - 62) / 2,
        y: 3,
        width: 62,
        height: 19,
    };
    canvas.border(rect, app.language.text(Message::Setup));
    canvas.centered_text(
        5,
        app.language.text(Message::Title),
        Style {
            fg: UiColor::Yellow,
            bold: true,
            ..Style::default()
        },
    );
    let player_two = if app.setup.mode == PlayMode::Dual {
        app.setup.names[1].clone()
    } else {
        "—".to_owned()
    };
    let values = [
        format!(
            "{}: {}",
            app.language.text(Message::Mode),
            app.language.play_mode(app.setup.mode)
        ),
        format!(
            "{}: {}",
            app.language.text(Message::PlayerOne),
            app.setup.names[0]
        ),
        format!("{}: {}", app.language.text(Message::PlayerTwo), player_two),
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
            app.language.graphics(app.setup.graphics, backend)
        ),
        app.language.text(Message::Start).to_owned(),
    ];
    for (index, value) in values.iter().enumerate() {
        let prefix = if index == app.setup.selected {
            "> "
        } else {
            "  "
        };
        canvas.text(
            rect.x + 3,
            7 + index as u16,
            &format!("{prefix}{value}"),
            Style::default().selected(index == app.setup.selected),
        );
    }
    canvas.centered_text(
        20,
        app.language.text(Message::SetupHint),
        Style::fg(UiColor::Cyan),
    );
}

fn render_game(canvas: &mut Canvas, app: &App, images: bool) {
    let game = app.game.as_ref().expect("game screen has game");
    let state = game.public_state();
    let current = state
        .players
        .iter()
        .find(|p| p.id == state.current_player)
        .map_or("?", |p| p.name.as_str());
    canvas.border(
        Rect {
            x: 0,
            y: 0,
            width: canvas.width,
            height: 3,
        },
        "* STAR TABLE *",
    );
    canvas.centered_text(
        1,
        &format!(
            "{} | {}: {} | {}: {}",
            app.language.deck_variant(game.deck_variant()),
            app.language.text(Message::Turn),
            current,
            app.language.text(Message::Direction),
            app.language.direction(state.direction)
        ),
        Style::fg(UiColor::Yellow),
    );
    let opponents = state
        .players
        .iter()
        .filter(|p| !app.is_human(&p.id))
        .map(|p| {
            format!(
                "{}: {} {}",
                p.name,
                p.hand_len,
                app.language.text(Message::Cards)
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    canvas.centered_text(3, &opponents, Style::fg(UiColor::Cyan));
    if images {
        let half = canvas.width / 2;
        let card_width = 12.min(half.saturating_sub(4));
        let selected = app.selected_human_card();
        let selected_title = app
            .current_human_index()
            .and_then(|index| {
                state
                    .players
                    .iter()
                    .find(|player| player.id == app.human_ids[index])
            })
            .map_or_else(
                || app.language.text(Message::SelectedCard).to_owned(),
                |player| {
                    format!(
                        "{}: {}",
                        app.language.text(Message::SelectedCard),
                        player.name
                    )
                },
            );
        canvas.border(
            Rect {
                x: 1,
                y: 4,
                width: half.saturating_sub(2),
                height: 9,
            },
            &selected_title,
        );
        canvas.border(
            Rect {
                x: half + 1,
                y: 4,
                width: canvas.width - half - 2,
                height: 9,
            },
            app.language.text(Message::DiscardTop),
        );
        if let Some(card) = selected {
            canvas.images.push(ImagePlacement {
                slot: ImageSlot::Selected,
                card,
                rect: Rect {
                    x: (half - card_width) / 2,
                    y: 5,
                    width: card_width,
                    height: 7,
                },
            });
        }
        canvas.images.push(ImagePlacement {
            slot: ImageSlot::Discard,
            card: state.discard_top,
            rect: Rect {
                x: half + (half - card_width) / 2,
                y: 5,
                width: card_width,
                height: 7,
            },
        });
    } else {
        canvas.border(
            Rect {
                x: 1,
                y: 4,
                width: canvas.width - 2,
                height: 5,
            },
            app.language.text(Message::Table),
        );
        canvas.centered_text(
            5,
            &format!(
                "{}: {}",
                app.language.text(Message::ActiveColor),
                app.language.color(state.active_color)
            ),
            Style {
                fg: color(state.active_color),
                bold: true,
                ..Style::default()
            },
        );
        canvas.centered_text(
            7,
            &format!("[ {} ]", app.language.card(state.discard_top)),
            Style::fg(state.discard_top.color.map_or(UiColor::Magenta, color)),
        );
    }
    let hand_y = if images { 13 } else { 9 };
    if app.setup.mode == PlayMode::Single {
        render_hand_panel(
            canvas,
            app,
            0,
            Rect {
                x: 1,
                y: hand_y,
                width: canvas.width - 2,
                height: 6,
            },
            app.language.text(Message::YourHand),
        );
    } else {
        let half = canvas.width / 2;
        for (player_index, rect, controls) in [
            (
                0,
                Rect {
                    x: 1,
                    y: hand_y,
                    width: half.saturating_sub(1),
                    height: 6,
                },
                "WASD",
            ),
            (
                1,
                Rect {
                    x: half,
                    y: hand_y,
                    width: canvas.width.saturating_sub(half + 1),
                    height: 6,
                },
                "hjkl/Arrows",
            ),
        ] {
            let name = state
                .players
                .iter()
                .find(|player| player.id == app.human_ids[player_index])
                .map_or("?", |player| player.name.as_str());
            let marker = if state.current_player == app.human_ids[player_index] {
                "*"
            } else {
                ""
            };
            render_hand_panel(
                canvas,
                app,
                player_index,
                rect,
                &format!("{marker}{name} [{controls}]"),
            );
        }
    }
    let log_y = hand_y + 6;
    if log_y + 3 < canvas.height {
        canvas.border(
            Rect {
                x: 1,
                y: log_y,
                width: canvas.width - 2,
                height: canvas.height - log_y - 3,
            },
            app.language.text(Message::EventLog),
        );
        for (index, line) in app
            .logs
            .iter()
            .rev()
            .take((canvas.height - log_y - 5) as usize)
            .enumerate()
        {
            canvas.text(3, log_y + 1 + index as u16, line, Style::default());
        }
    }
    let footer = if app.command_mode {
        format!(":{}", app.command)
    } else {
        app.status.clone()
    };
    canvas.centered_text(canvas.height - 2, &footer, Style::fg(UiColor::Cyan));
    canvas.centered_text(
        canvas.height - 1,
        app.language.game_hint(app.setup.mode),
        Style::fg(UiColor::Gray),
    );
}

fn render_hand_panel(canvas: &mut Canvas, app: &App, player_index: usize, rect: Rect, title: &str) {
    canvas.border(rect, title);
    let hand = app.human_hand(player_index).unwrap_or_default();
    let rows = wrap_hand(
        app.language,
        hand,
        rect.width.saturating_sub(2).max(1) as usize,
    );
    let selected_row = rows
        .iter()
        .position(|row| {
            row.iter()
                .any(|(index, _)| *index == app.selected_cards[player_index])
        })
        .unwrap_or(0);
    let first_row = selected_row.saturating_sub(3);
    for (row_index, row) in rows.into_iter().skip(first_row).take(4).enumerate() {
        let mut x = rect.x + 1;
        for (index, entry) in row {
            canvas.text(
                x,
                rect.y + 1 + row_index as u16,
                &entry,
                Style::fg(hand[index].color.map_or(UiColor::Magenta, color))
                    .selected(index == app.selected_cards[player_index]),
            );
            x += UnicodeWidthStr::width(entry.as_str()) as u16;
        }
    }
}

fn render_overlay(canvas: &mut Canvas, title: &str, body: &str) {
    let mut next = Canvas::new(canvas.width, canvas.height);
    let rect = Rect {
        x: 6,
        y: 3,
        width: canvas.width - 12,
        height: canvas.height - 6,
    };
    next.border(rect, title);
    for (index, line) in body.lines().enumerate() {
        next.text(
            rect.x + 3,
            rect.y + 2 + index as u16,
            line,
            Style::default(),
        );
    }
    *canvas = next;
}

fn render_compact_overlay(canvas: &mut Canvas, title: &str, body: &str) {
    let body_width = body.lines().map(UnicodeWidthStr::width).max().unwrap_or(0);
    let width = (body_width + 6)
        .max(UnicodeWidthStr::width(title) + 4)
        .min(usize::from(canvas.width)) as u16;
    let height = body.lines().count() as u16 + 3;
    let rect = Rect {
        x: canvas.width.saturating_sub(width) / 2,
        y: 4,
        width,
        height,
    };
    canvas.clear(rect);
    canvas.border(rect, title);
    for (index, line) in body.lines().enumerate() {
        canvas.text(
            rect.x + 3,
            rect.y + 2 + index as u16,
            line,
            Style::default(),
        );
    }
}

fn render_result(canvas: &mut Canvas, app: &App) {
    let winner = app
        .game
        .as_ref()
        .and_then(|game| {
            let state = game.public_state();
            state.winner.and_then(|id| {
                state
                    .players
                    .into_iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name)
            })
        })
        .unwrap_or_default();
    render_overlay(
        canvas,
        app.language.text(Message::Winner),
        &format!("{winner}\n\n{}", app.language.text(Message::NewMatchHint)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PendingWild;
    use crate::frontend::{FallbackReason, GraphicsChoice};
    use crate::i18n::Language;

    #[test]
    fn setup_and_small_terminal_are_renderable_without_ratatui() {
        let app = App::with_graphics(Language::English, GraphicsChoice::Text);
        let setup = render(
            &app,
            GraphicsBackend::Text(FallbackReason::Manual),
            Viewport {
                columns: 80,
                rows: 28,
            },
        )
        .plain_text();
        assert!(setup.contains("New local match"));
        let small = render(
            &app,
            GraphicsBackend::Text(FallbackReason::Manual),
            Viewport {
                columns: 60,
                rows: 20,
            },
        );
        assert!(small.images.is_empty());
        assert!(small.plain_text().contains("Terminal too small"));
    }

    #[test]
    fn game_images_are_semantic_and_overlays_remove_them() {
        let mut app = App::with_graphics(Language::English, GraphicsChoice::GraphicsBeta);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let viewport = Viewport {
            columns: 80,
            rows: 28,
        };
        let game = render(&app, GraphicsBackend::Sixel, viewport);
        assert_eq!(game.images.len(), 2);
        app.screen = Screen::Help;
        let help = render(&app, GraphicsBackend::Sixel, viewport);
        assert!(help.images.is_empty());
        assert!(help.plain_text().contains("Shortcuts"));
    }

    #[test]
    fn dual_mode_renders_both_hands_and_current_preview() {
        let mut app = App::with_graphics(Language::English, GraphicsChoice::GraphicsBeta);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 0;
        app.setup.names = ["Left".to_owned(), "Right".to_owned()];
        app.start_match().unwrap();
        let game = render(
            &app,
            GraphicsBackend::Sixel,
            Viewport {
                columns: 70,
                rows: 26,
            },
        );
        let text = game.plain_text();

        assert!(text.contains("*Left [WASD]"));
        assert!(text.contains("Right [hjkl/Arrows]"));
        assert!(text.contains("Enter play · X draw · P pass"));
        assert_eq!(game.images.len(), 2);
    }

    #[test]
    fn compact_color_picker_preserves_the_hand_and_game_details() {
        let mut app = App::with_graphics(Language::English, GraphicsChoice::GraphicsBeta);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.pending_wild = Some(PendingWild {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
        });
        let picker = render(
            &app,
            GraphicsBackend::Sixel,
            Viewport {
                columns: 70,
                rows: 26,
            },
        );
        let text = picker.plain_text();

        assert!(picker.images.is_empty());
        assert!(text.contains("Choose a color"));
        assert!(text.contains("[RED]  YELLOW  GREEN  BLUE"));
        assert!(text.contains("Your hand"));
        assert!(text.contains("Events"));
        assert!(text.contains("? help · Q quit"));
    }

    #[test]
    fn compact_color_picker_fits_chinese_at_minimum_size() {
        let mut app = App::with_graphics(Language::Chinese, GraphicsChoice::Text);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.pending_wild = Some(PendingWild {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
        });
        let picker = render(
            &app,
            GraphicsBackend::Text(FallbackReason::Manual),
            Viewport {
                columns: 70,
                rows: 26,
            },
        )
        .plain_text();

        assert!(picker.contains("选择颜色"));
        assert!(picker.contains("[红]  黄  绿  蓝"));
        assert!(picker.contains("你的手牌"));
        assert!(picker.contains("事件"));
    }
}
