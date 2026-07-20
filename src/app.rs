//! * STAR CARNIVAL APP *
//!
//! Setup, input, local turns, and Holiday color selection.

use std::cell::{Ref, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use crate::ai::{Difficulty, choose_action};
use crate::core::{
    Action, Card, Color, DeckVariant, EventKind, Game, GameEvent, HandEffect, HouseRules,
    PlayerDrawRule, PlayerId, PlusPlay, Rank,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::frontend::{GraphicsChoice, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::i18n::{Language, Message};

const AI_DELAY: Duration = Duration::from_secs(1);
const MAX_LOGS: usize = 8;
const HAND_WINDOW_RADIUS: usize = crate::core::HAND_BATCH_SIZE;
const PAGE_SWITCH_PRESSES: u8 = 5;

#[derive(Default)]
struct VisibleHandCache {
    indices: [Vec<usize>; 2],
    valid: [bool; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Setup,
    Game,
    Help,
    Result,
    QuitConfirm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayMode {
    Single,
    Dual,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HandFilter {
    #[default]
    All,
    Positive,
    Negative,
    SevenZero,
    Mathematical,
}

impl HandFilter {
    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::Positive,
            Self::Positive => Self::Negative,
            Self::Negative => Self::SevenZero,
            Self::SevenZero => Self::Mathematical,
            Self::Mathematical => Self::All,
        }
    }

    pub const fn matches(self, card: Card) -> bool {
        match self {
            Self::All => true,
            Self::Positive => matches!(
                card.rank,
                Rank::DrawTwo | Rank::DrawEight | Rank::WildDrawFour | Rank::WildDrawSixteen
            ),
            Self::Negative => matches!(
                card.rank,
                Rank::WildDiscardThirtyTwo | Rank::WildDiscardSixtyFour
            ),
            Self::SevenZero => matches!(card.rank, Rank::Number(0 | 7)),
            Self::Mathematical => {
                matches!(card.rank, Rank::WildFactorial | Rank::WildSquareRoot)
            }
        }
    }
}

impl PlayMode {
    pub const ALL: [Self; 2] = [Self::Single, Self::Dual];

    pub const fn human_count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Dual => 2,
        }
    }
}

#[derive(Debug)]
pub struct Setup {
    pub mode: PlayMode,
    pub names: [String; 2],
    pub bot_count: usize,
    pub difficulty: Difficulty,
    pub deck_variant: DeckVariant,
    pub seven_zero: bool,
    /// 用户选择的牌面显示策略；实际协议仍由 `GraphicsRuntime` 根据终端决定。
    pub graphics: GraphicsChoice,
    pub selected: usize,
}

impl Setup {
    fn new(language: Language, graphics: GraphicsChoice) -> Self {
        Self {
            mode: PlayMode::Single,
            names: default_player_names(language),
            bot_count: 3,
            difficulty: Difficulty::Normal,
            deck_variant: DeckVariant::Holiday,
            seven_zero: true,
            graphics,
            selected: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingColor {
    pub player_index: usize,
    pub card: Card,
    pub colors: Vec<Color>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingSeven {
    pub player_index: usize,
    pub card: Card,
    pub chosen_color: Option<Color>,
    pub targets: Vec<PlayerId>,
    pub selected_target: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingPlusBatch {
    pub player_index: usize,
    pub plays: Vec<PlusPlay>,
}

pub struct App {
    pub language: Language,
    pub screen: Screen,
    pub setup: Setup,
    pub game: Option<Game>,
    pub human_ids: [PlayerId; 2],
    pub ai_ids: Vec<PlayerId>,
    pub selected_cards: [usize; 2],
    pub hand_filter: HandFilter,
    pub command_mode: bool,
    pub command: String,
    pub pending_color: Option<PendingColor>,
    pub pending_seven: Option<PendingSeven>,
    pub pending_plus_batch: Option<PendingPlusBatch>,
    pub selected_color: usize,
    pub logs: VecDeque<String>,
    pub status: String,
    pub should_exit: bool,
    previous_screen: Screen,
    ai_deadline: Instant,
    ai_rng: StdRng,
    visible_hand_cache: RefCell<VisibleHandCache>,
    page_up_presses: [u8; 2],
    page_down_presses: [u8; 2],
}

impl App {
    #[cfg(test)]
    pub fn new(language: Language) -> Self {
        Self::with_graphics(language, GraphicsChoice::Text)
    }

    /// 使用终端环境推荐的初始图形选项创建应用状态。
    pub fn with_graphics(language: Language, graphics: GraphicsChoice) -> Self {
        Self {
            language,
            screen: Screen::Setup,
            setup: Setup::new(language, graphics),
            game: None,
            human_ids: [PlayerId::new("human-1"), PlayerId::new("human-2")],
            ai_ids: Vec::new(),
            selected_cards: [0; 2],
            hand_filter: HandFilter::All,
            command_mode: false,
            command: String::new(),
            pending_color: None,
            pending_seven: None,
            pending_plus_batch: None,
            selected_color: 0,
            logs: VecDeque::new(),
            status: String::new(),
            should_exit: false,
            previous_screen: Screen::Setup,
            ai_deadline: Instant::now(),
            ai_rng: StdRng::from_entropy(),
            visible_hand_cache: RefCell::new(VisibleHandCache::default()),
            page_up_presses: [0; 2],
            page_down_presses: [0; 2],
        }
    }

    pub fn start_match(&mut self) -> Result<(), String> {
        let human_count = self.setup.mode.human_count();
        let mut players = (0..human_count)
            .map(|index| {
                let name = if self.setup.names[index].trim().is_empty() {
                    default_player_names(self.language)[index].clone()
                } else {
                    self.setup.names[index].trim().to_owned()
                };
                (self.human_ids[index].clone(), name)
            })
            .collect::<Vec<_>>();
        self.ai_ids = (1..=self.setup.bot_count)
            .map(|index| PlayerId::new(format!("ai-{index}")))
            .collect();
        players.extend(self.ai_ids.iter().enumerate().map(|(index, id)| {
            let name = match self.language {
                Language::English => format!("AI {}", index + 1),
                Language::Chinese => format!("电脑 {}", index + 1),
            };
            (id.clone(), name)
        }));
        let player_draw_rules = draw_rules_for_match(
            self.setup.difficulty,
            &self.human_ids[..human_count],
            &self.ai_ids,
        );
        let house_rules = HouseRules {
            seven_zero: self.setup.seven_zero,
        };
        self.game = Some(
            match (self.setup.deck_variant, self.setup.seven_zero) {
                (DeckVariant::Holiday, true) => {
                    Game::new_with_draw_rules(players, self.setup.deck_variant, player_draw_rules)
                }
                (DeckVariant::Holiday, false) => Game::new_with_house_rules_and_draw_rules(
                    players,
                    self.setup.deck_variant,
                    house_rules,
                    player_draw_rules,
                ),
                (DeckVariant::Standard, true) => Game::new(players, self.setup.deck_variant),
                (DeckVariant::Standard, false) => {
                    Game::new_with_house_rules(players, self.setup.deck_variant, house_rules)
                }
            }
            .map_err(|error| error.to_string())?,
        );
        self.screen = Screen::Game;
        self.selected_cards = [0; 2];
        self.hand_filter = HandFilter::All;
        self.invalidate_visible_hands();
        self.reset_page_presses();
        self.command_mode = false;
        self.pending_color = None;
        self.pending_seven = None;
        self.pending_plus_batch = None;
        self.logs.clear();
        self.update_turn_status();
        self.ai_deadline = Instant::now() + AI_DELAY;
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent, terminal_width: u16) {
        // Terminals using an enhanced keyboard protocol report both press and
        // release events. A release must not apply the same action twice.
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_exit = true;
            return;
        }
        match self.screen {
            Screen::Setup => self.handle_setup_key(key),
            Screen::Game => self.handle_game_key(key, terminal_width),
            Screen::Help => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                    self.screen = self.previous_screen;
                }
            }
            Screen::Result => match key.code {
                KeyCode::Char('n' | 'N') => self.return_to_setup(),
                KeyCode::Char('q' | 'Q') | KeyCode::Esc => self.should_exit = true,
                _ => {}
            },
            Screen::QuitConfirm => match key.code {
                KeyCode::Char('y' | 'Y') => self.should_exit = true,
                KeyCode::Char('n' | 'N') | KeyCode::Esc => self.screen = self.previous_screen,
                _ => {}
            },
        }
    }

    pub fn tick(&mut self) {
        if self.screen != Screen::Game
            || self.pending_color.is_some()
            || self.pending_seven.is_some()
            || self.pending_plus_batch.is_some()
            || self.command_mode
        {
            return;
        }
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if game.public_state().winner.is_some() || self.is_human(game.current_player()) {
            return;
        }
        if Instant::now() < self.ai_deadline {
            return;
        }
        self.take_ai_turn();
    }

    fn handle_setup_key(&mut self, key: KeyEvent) {
        // The name field remains a text input, so Vim keys only become
        // navigation aliases after the player leaves that field.
        let editing_name = self.setup.selected == 1
            || (self.setup.selected == 2 && self.setup.mode == PlayMode::Dual);
        let code = if editing_name {
            key.code
        } else {
            navigation_code(key)
        };
        match code {
            KeyCode::Up => self.setup.selected = self.setup.selected.saturating_sub(1),
            KeyCode::Down => self.setup.selected = (self.setup.selected + 1).min(9),
            KeyCode::Left => self.adjust_setup(-1),
            KeyCode::Right => self.adjust_setup(1),
            KeyCode::Enter if self.setup.selected == 9 => {
                if let Err(error) = self.start_match() {
                    self.status = error;
                }
            }
            KeyCode::Enter => self.setup.selected = (self.setup.selected + 1).min(9),
            KeyCode::Backspace if editing_name => {
                self.setup.names[self.setup.selected - 1].pop();
            }
            KeyCode::Esc => self.should_exit = true,
            KeyCode::Char(character)
                if editing_name
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.setup.names[self.setup.selected - 1].chars().count() < 20 =>
            {
                self.setup.names[self.setup.selected - 1].push(character);
            }
            _ => {}
        }
    }

    fn adjust_setup(&mut self, delta: isize) {
        match self.setup.selected {
            0 => {
                let index = PlayMode::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.mode)
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .clamp(0, PlayMode::ALL.len() - 1);
                self.setup.mode = PlayMode::ALL[index];
                self.setup.bot_count = match self.setup.mode {
                    PlayMode::Single => self.setup.bot_count.clamp(1, 4),
                    PlayMode::Dual => self.setup.bot_count.min(3),
                };
            }
            3 => {
                let (minimum, maximum) = match self.setup.mode {
                    PlayMode::Single => (1, 4),
                    PlayMode::Dual => (0, 3),
                };
                self.setup.bot_count = self
                    .setup
                    .bot_count
                    .saturating_add_signed(delta)
                    .clamp(minimum, maximum);
            }
            4 => {
                let index = Difficulty::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.difficulty)
                    .unwrap_or(1)
                    .saturating_add_signed(delta)
                    .clamp(0, Difficulty::ALL.len() - 1);
                self.setup.difficulty = Difficulty::ALL[index];
            }
            5 => {
                let index = DeckVariant::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.deck_variant)
                    .unwrap_or(1)
                    .saturating_add_signed(delta)
                    .clamp(0, DeckVariant::ALL.len() - 1);
                self.setup.deck_variant = DeckVariant::ALL[index];
            }
            6 => self.setup.seven_zero = delta >= 0,
            7 => {
                let old_language = self.language;
                let old_defaults = default_player_names(old_language);
                let index = Language::ALL
                    .iter()
                    .position(|candidate| *candidate == self.language)
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .clamp(0, Language::ALL.len() - 1);
                self.language = Language::ALL[index];
                let new_defaults = default_player_names(self.language);
                for index in 0..2 {
                    if self.setup.names[index] == old_defaults[index] {
                        self.setup.names[index] = new_defaults[index].clone();
                    }
                }
            }
            8 => {
                // 此处只保存 Text/Graphics Beta 偏好，不在输入处理阶段探测或切换协议；
                // UI 每帧通过 GraphicsRuntime 解析实际后端。
                let index = GraphicsChoice::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.graphics)
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .clamp(0, GraphicsChoice::ALL.len() - 1);
                self.setup.graphics = GraphicsChoice::ALL[index];
            }
            _ => {}
        }
    }

    fn handle_game_key(&mut self, key: KeyEvent, terminal_width: u16) {
        if self.command_mode {
            self.handle_command_key(key);
            return;
        }
        if self.pending_color.is_some() {
            self.handle_color_key(key);
            return;
        }
        if self.pending_plus_batch.is_some() {
            self.handle_plus_batch_color_key(key);
            return;
        }
        if self.pending_seven.is_some() {
            self.handle_seven_target_key(key);
            return;
        }
        if let Some((player_index, code)) = self.player_navigation(key) {
            self.navigate_hand(player_index, code, terminal_width);
            return;
        }
        match key.code {
            KeyCode::Enter => self.play_current_human_selected(),
            KeyCode::Char('f' | 'F') => {
                self.hand_filter = self.hand_filter.next();
                self.selected_cards = [0; 2];
                self.invalidate_visible_hands();
                self.reset_page_presses();
            }
            KeyCode::Char('g' | 'G') => self.play_best_plus_batch(),
            KeyCode::Char('d' | 'D') if self.setup.mode == PlayMode::Single => {
                self.submit_current_human(Action::Draw)
            }
            KeyCode::Char('x' | 'X') if self.setup.mode == PlayMode::Dual => {
                self.submit_current_human(Action::Draw)
            }
            KeyCode::Char('p' | 'P') => self.submit_current_human(Action::Pass),
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command.clear();
            }
            KeyCode::Char('?') => self.open_help(),
            KeyCode::Char('q' | 'Q') => self.open_quit(),
            _ => {}
        }
    }

    fn player_navigation(&self, key: KeyEvent) -> Option<(usize, KeyCode)> {
        if self.setup.mode == PlayMode::Single {
            let code = navigation_code(key);
            return matches!(
                code,
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
            )
            .then_some((0, code));
        }
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        match key.code {
            KeyCode::Char('w') => Some((0, KeyCode::Up)),
            KeyCode::Char('s') => Some((0, KeyCode::Down)),
            KeyCode::Char('a') => Some((0, KeyCode::Left)),
            KeyCode::Char('d') => Some((0, KeyCode::Right)),
            KeyCode::Char('k') => Some((1, KeyCode::Up)),
            KeyCode::Char('j') => Some((1, KeyCode::Down)),
            KeyCode::Char('h') => Some((1, KeyCode::Left)),
            KeyCode::Char('l') => Some((1, KeyCode::Right)),
            code @ (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right) => self
                .current_human_index()
                .map(|player_index| (player_index, code)),
            _ => None,
        }
    }

    fn navigate_hand(&mut self, player_index: usize, code: KeyCode, terminal_width: u16) {
        let hand_width = match self.setup.mode {
            PlayMode::Single => terminal_width.saturating_sub(4),
            PlayMode::Dual => terminal_width.saturating_div(2).saturating_sub(3),
        } as usize;
        match code {
            KeyCode::Up | KeyCode::Down => {
                let at_boundary = self.selection_is_vertical_boundary(
                    player_index,
                    hand_width,
                    code == KeyCode::Up,
                );
                if at_boundary && self.count_page_boundary_press(player_index, code) {
                    return;
                }
                let row_delta = if code == KeyCode::Up { -1 } else { 1 };
                let (first_index, hand) = self.visible_human_window(player_index);
                let previous = self.selected_cards[player_index];
                let selected = crate::view::adjacent_hand_card(
                    self.language,
                    &hand,
                    first_index,
                    previous,
                    hand_width,
                    row_delta,
                );
                self.selected_cards[player_index] = selected;
                if selected != previous || !at_boundary {
                    self.page_up_presses[player_index] = 0;
                    self.page_down_presses[player_index] = 0;
                }
            }
            KeyCode::Left => {
                self.selected_cards[player_index] =
                    self.selected_cards[player_index].saturating_sub(1);
                self.reset_page_press_if_off_boundary(player_index, hand_width);
            }
            KeyCode::Right => {
                let len = self.visible_human_hand_len(player_index);
                if len > 0 {
                    self.selected_cards[player_index] =
                        (self.selected_cards[player_index] + 1).min(len - 1);
                }
                self.reset_page_press_if_off_boundary(player_index, hand_width);
            }
            _ => {}
        }
    }

    fn reset_page_press_if_off_boundary(&mut self, player_index: usize, hand_width: usize) {
        if !self.selection_is_vertical_boundary(player_index, hand_width, true) {
            self.page_up_presses[player_index] = 0;
        }
        if !self.selection_is_vertical_boundary(player_index, hand_width, false) {
            self.page_down_presses[player_index] = 0;
        }
    }

    fn selection_is_vertical_boundary(
        &self,
        player_index: usize,
        hand_width: usize,
        top: bool,
    ) -> bool {
        let (first_index, hand) = self.visible_human_window(player_index);
        if hand.is_empty() {
            return true;
        }
        let rows =
            crate::view::wrap_hand_indexed(self.language, &hand, first_index, hand_width.max(1));
        let selected_row = rows.iter().position(|row| {
            row.iter()
                .any(|(index, _)| *index == self.selected_cards[player_index])
        });
        selected_row.is_some_and(|row| if top { row == 0 } else { row + 1 == rows.len() })
    }

    fn count_page_boundary_press(&mut self, player_index: usize, code: KeyCode) -> bool {
        let up = code == KeyCode::Up;
        let (counter, direction) = if up {
            (&mut self.page_up_presses[player_index], -1_isize)
        } else {
            (&mut self.page_down_presses[player_index], 1_isize)
        };
        *counter = counter.saturating_add(1);
        let presses = *counter;
        if presses < PAGE_SWITCH_PRESSES {
            self.status = self
                .language
                .page_press_status(up, PAGE_SWITCH_PRESSES - presses);
            return true;
        }
        let player = self.human_ids[player_index].clone();
        let Some(game) = self.game.as_ref() else {
            return true;
        };
        let Ok((page, pages, _)) = game.hand_page_for(&player) else {
            return true;
        };
        let target = if direction < 0 {
            page.checked_sub(1)
        } else {
            (page + 1 < pages).then_some(page + 1)
        };
        self.reset_page_presses();
        let Some(target) = target else {
            self.status = self.language.page_edge_status(up).to_owned();
            return true;
        };
        if self
            .game
            .as_mut()
            .is_some_and(|game| game.materialize_hand_page(&player, target).is_ok())
        {
            self.invalidate_visible_hands();
            self.selected_cards[player_index] = if direction < 0 {
                self.visible_human_hand_len(player_index).saturating_sub(1)
            } else {
                0
            };
            self.status = self.language.page_loaded_status(target, pages);
        }
        true
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command.clear();
            }
            KeyCode::Backspace => {
                self.command.pop();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.command);
                self.command_mode = false;
                self.run_command(&input);
            }
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.command.chars().count() < 80 =>
            {
                self.command.push(character);
            }
            _ => {}
        }
    }

    fn handle_color_key(&mut self, key: KeyEvent) {
        let Some(pending) = self.pending_color.as_ref() else {
            return;
        };
        let player_index = pending.player_index;
        let color_count = pending.colors.len();
        let navigation = self
            .player_navigation(key)
            .filter(|(candidate, _)| *candidate == player_index)
            .map(|(_, code)| code)
            .unwrap_or(key.code);
        match navigation {
            KeyCode::Left => self.selected_color = self.selected_color.saturating_sub(1),
            KeyCode::Right => {
                self.selected_color = (self.selected_color + 1).min(color_count.saturating_sub(1))
            }
            KeyCode::Esc => self.pending_color = None,
            KeyCode::Enter => {
                if let Some(pending) = self.pending_color.take()
                    && let Some(chosen_color) = pending.colors.get(self.selected_color).copied()
                {
                    if self
                        .game
                        .as_ref()
                        .is_some_and(|game| game.house_rules().seven_zero)
                        && pending.card.rank == Rank::Number(7)
                    {
                        self.begin_seven_target(
                            pending.player_index,
                            pending.card,
                            Some(chosen_color),
                        );
                    } else {
                        self.submit_human(
                            pending.player_index,
                            Action::Play {
                                card: pending.card,
                                chosen_color: Some(chosen_color),
                                swap_target: None,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn play_best_plus_batch(&mut self) {
        let Some(player_index) = self.current_human_index() else {
            return;
        };
        let player = self.human_ids[player_index].clone();
        let result = self
            .game
            .as_ref()
            .expect("game screen has game")
            .best_plus_batch(&player);
        let plays = match result {
            Ok(plays) if plays.is_empty() => {
                self.status = self.language.text(Message::NoPlayablePlusBatch).to_owned();
                return;
            }
            Ok(plays) => plays,
            Err(error) => {
                self.status = self.language.game_error(&error);
                return;
            }
        };
        if plays
            .last()
            .is_some_and(|play| play.card.rank == Rank::WildDrawSixteen)
        {
            self.pending_plus_batch = Some(PendingPlusBatch {
                player_index,
                plays,
            });
            self.selected_color = 0;
        } else {
            self.submit_plus_batch(player_index, plays);
        }
    }

    fn handle_plus_batch_color_key(&mut self, key: KeyEvent) {
        let Some(pending) = self.pending_plus_batch.as_ref() else {
            return;
        };
        let player_index = pending.player_index;
        let navigation = self
            .player_navigation(key)
            .filter(|(candidate, _)| *candidate == player_index)
            .map(|(_, code)| code)
            .unwrap_or(key.code);
        match navigation {
            KeyCode::Left => self.selected_color = self.selected_color.saturating_sub(1),
            KeyCode::Right => self.selected_color = (self.selected_color + 1).min(3),
            KeyCode::Esc => self.pending_plus_batch = None,
            KeyCode::Enter => {
                if let Some(mut pending) = self.pending_plus_batch.take() {
                    if let Some(last) = pending.plays.last_mut() {
                        last.chosen_color = Some(Color::ALL[self.selected_color]);
                    }
                    self.submit_plus_batch(pending.player_index, pending.plays);
                }
            }
            _ => {}
        }
    }

    fn handle_seven_target_key(&mut self, key: KeyEvent) {
        let Some(pending) = self.pending_seven.as_ref() else {
            return;
        };
        let player_index = pending.player_index;
        let navigation = self
            .player_navigation(key)
            .filter(|(candidate, _)| *candidate == player_index)
            .map(|(_, code)| code)
            .unwrap_or(key.code);
        match navigation {
            KeyCode::Left => {
                if let Some(pending) = self.pending_seven.as_mut() {
                    pending.selected_target = pending.selected_target.saturating_sub(1);
                }
            }
            KeyCode::Right => {
                if let Some(pending) = self.pending_seven.as_mut() {
                    pending.selected_target =
                        (pending.selected_target + 1).min(pending.targets.len().saturating_sub(1));
                }
            }
            KeyCode::Esc => self.pending_seven = None,
            KeyCode::Enter => {
                if let Some(pending) = self.pending_seven.take()
                    && let Some(target) = pending.targets.get(pending.selected_target).cloned()
                {
                    self.submit_human(
                        pending.player_index,
                        Action::Play {
                            card: pending.card,
                            chosen_color: pending.chosen_color,
                            swap_target: Some(target),
                        },
                    );
                }
            }
            _ => {}
        }
    }

    fn play_current_human_selected(&mut self) {
        let Some(player_index) = self.current_human_index() else {
            return;
        };
        self.play_selected(player_index);
    }

    fn play_selected(&mut self, player_index: usize) {
        let Some(card) = self.visible_human_card(player_index, self.selected_cards[player_index])
        else {
            self.status = self.language.text(Message::InvalidCardIndex).to_owned();
            return;
        };
        let selectable_colors = if card.is_wild() {
            Color::ALL.to_vec()
        } else {
            let player = &self.human_ids[player_index];
            self.game
                .as_ref()
                .expect("game screen has game")
                .legal_actions(player)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|action| match action {
                    Action::Play {
                        card: candidate,
                        chosen_color,
                        ..
                    } if candidate == card => chosen_color,
                    _ => None,
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };
        if !selectable_colors.is_empty() {
            self.pending_color = Some(PendingColor {
                player_index,
                card,
                colors: selectable_colors,
            });
            self.selected_color = 0;
        } else if self
            .game
            .as_ref()
            .is_some_and(|game| game.house_rules().seven_zero)
            && card.rank == Rank::Number(7)
        {
            self.begin_seven_target(player_index, card, None);
        } else {
            self.submit_human(
                player_index,
                Action::Play {
                    card,
                    chosen_color: None,
                    swap_target: None,
                },
            );
        }
    }

    fn begin_seven_target(&mut self, player_index: usize, card: Card, chosen_color: Option<Color>) {
        let current = self.human_ids[player_index].clone();
        let targets = self
            .game
            .as_ref()
            .expect("game screen has game")
            .public_state()
            .players
            .into_iter()
            .filter(|player| player.id != current)
            .map(|player| player.id)
            .collect();
        self.pending_seven = Some(PendingSeven {
            player_index,
            card,
            chosen_color,
            targets,
            selected_target: 0,
        });
    }

    fn submit_current_human(&mut self, action: Action) {
        let Some(player_index) = self.current_human_index() else {
            return;
        };
        self.submit_human(player_index, action);
    }

    fn submit_human(&mut self, player_index: usize, action: Action) {
        let result = self
            .game
            .as_mut()
            .expect("game screen has game")
            .apply_action(&self.human_ids[player_index], action);
        match result {
            Ok(event) => self.after_event(event),
            Err(error) => self.status = self.language.game_error(&error),
        }
    }

    fn submit_plus_batch(&mut self, player_index: usize, plays: Vec<PlusPlay>) {
        let result = self
            .game
            .as_mut()
            .expect("game screen has game")
            .apply_plus_batch(&self.human_ids[player_index], plays);
        match result {
            Ok(event) => self.after_event(event),
            Err(error) => self.status = self.language.game_error(&error),
        }
    }

    fn take_ai_turn(&mut self) {
        {
            let game = self.game.as_mut().expect("game screen has game");
            let player = game.current_player().clone();
            game.materialize_next_batch_if_empty(&player)
                .expect("current player remains in game");
        }
        let (player, action) = {
            let game = self.game.as_ref().expect("game screen has game");
            let player = game.current_player().clone();
            let state = game.public_state();
            let hand = game.hand_for(&player).expect("current player is in game");
            let legal = game
                .legal_actions(&player)
                .expect("current player has legal actions");
            let action = choose_action(
                self.setup.difficulty,
                &state,
                hand,
                &legal,
                &mut self.ai_rng,
            );
            (player, action)
        };
        match self
            .game
            .as_mut()
            .expect("game screen has game")
            .apply_action(&player, action)
        {
            Ok(event) => self.after_event(event),
            Err(error) => self.status = self.language.game_error(&error),
        }
    }

    fn after_event(&mut self, event: GameEvent) {
        if let Some(game) = self.game.as_mut() {
            for player in &self.human_ids[..self.setup.mode.human_count()] {
                game.materialize_next_batch_if_empty(player)
                    .expect("configured human remains in game");
            }
        }
        self.invalidate_visible_hands();
        self.reset_page_presses();
        let state = self.game.as_ref().expect("game exists").public_state();
        let name = state
            .players
            .iter()
            .find(|candidate| match &event.kind {
                EventKind::CardPlayed { player, .. }
                | EventKind::PlusBatchPlayed { player, .. }
                | EventKind::CardDrawn { player, .. }
                | EventKind::TurnPassed { player }
                | EventKind::GameWon { player } => &candidate.id == player,
                EventKind::GameStarted => false,
            })
            .map(|player| player.name.clone())
            .unwrap_or_default();
        let line = match event.kind {
            EventKind::CardPlayed {
                card, hand_effect, ..
            } => {
                let played = format!(
                    "{name} {} {}",
                    self.language.text(Message::Played),
                    self.language.card(card)
                );
                match hand_effect {
                    Some(HandEffect::Swap { target }) => {
                        let target_name = state
                            .players
                            .iter()
                            .find(|player| player.id == target)
                            .map_or(target.0.as_str(), |player| player.name.as_str());
                        self.language.swap_log(&played, target_name)
                    }
                    Some(HandEffect::Rotate { direction }) => {
                        self.language.rotate_log(&played, direction)
                    }
                    Some(HandEffect::Redistribute {
                        discarded,
                        distributed,
                    }) => self
                        .language
                        .redistribute_log(&played, discarded, distributed),
                    Some(HandEffect::Factorial {
                        target,
                        before,
                        after,
                    })
                    | Some(HandEffect::SquareRoot {
                        target,
                        before,
                        after,
                    }) => {
                        let target_name = state
                            .players
                            .iter()
                            .find(|player| player.id == target)
                            .map_or(target.0.as_str(), |player| player.name.as_str());
                        self.language
                            .hand_transform_log(&played, target_name, before, after)
                    }
                    None => played,
                }
            }
            EventKind::CardDrawn { .. } => {
                format!("{name} {}", self.language.text(Message::DrewCard))
            }
            EventKind::PlusBatchPlayed {
                cards,
                target,
                penalty,
                drawn,
                ..
            } => {
                let target_name = state
                    .players
                    .iter()
                    .find(|player| player.id == target)
                    .map_or(target.0.as_str(), |player| player.name.as_str());
                self.language
                    .plus_batch_log(&name, cards.len(), target_name, penalty, drawn)
            }
            EventKind::TurnPassed { .. } => {
                format!("{name} {}", self.language.text(Message::Passed))
            }
            EventKind::GameWon { .. } => name,
            EventKind::GameStarted => String::new(),
        };
        if !line.is_empty() {
            self.logs.push_back(line.clone());
            while self.logs.len() > MAX_LOGS {
                self.logs.pop_front();
            }
            self.status = line;
        }
        for player_index in 0..self.setup.mode.human_count() {
            let hand_len = self.visible_human_hand_len(player_index);
            self.selected_cards[player_index] =
                self.selected_cards[player_index].min(hand_len.saturating_sub(1));
        }
        if state.winner.is_some() {
            self.screen = Screen::Result;
        } else {
            self.update_turn_status();
        }
    }

    fn run_command(&mut self, input: &str) {
        match AppCommand::parse(input) {
            Ok(AppCommand::Play(index)) => {
                let Some(player_index) = self.current_human_index() else {
                    return;
                };
                if index == 0 || index > self.visible_human_hand_len(player_index) {
                    self.status = self.language.text(Message::InvalidCardIndex).to_owned();
                    return;
                }
                self.selected_cards[player_index] = index - 1;
                self.play_selected(player_index);
            }
            Ok(AppCommand::Draw) => self.submit_current_human(Action::Draw),
            Ok(AppCommand::Pass) => self.submit_current_human(Action::Pass),
            Ok(AppCommand::Help) => self.open_help(),
            Ok(AppCommand::New) => self.return_to_setup(),
            Ok(AppCommand::Quit) => self.open_quit(),
            Err(()) => self.status = self.language.text(Message::InvalidCommand).to_owned(),
        }
    }

    fn open_help(&mut self) {
        self.previous_screen = self.screen;
        self.screen = Screen::Help;
    }

    fn open_quit(&mut self) {
        self.previous_screen = self.screen;
        self.screen = Screen::QuitConfirm;
    }

    fn return_to_setup(&mut self) {
        self.game = None;
        self.screen = Screen::Setup;
        self.command_mode = false;
        self.pending_color = None;
        self.pending_seven = None;
        self.pending_plus_batch = None;
        self.logs.clear();
        self.status.clear();
        self.reset_page_presses();
    }

    pub fn human_hand(&self, player_index: usize) -> Option<&[Card]> {
        self.game
            .as_ref()
            .and_then(|game| game.hand_for(&self.human_ids[player_index]).ok())
    }

    #[cfg(test)]
    pub fn visible_human_hand(&self, player_index: usize) -> Vec<Card> {
        let indices = self.visible_hand_indices(player_index);
        let hand = self.human_hand(player_index).unwrap_or_default();
        indices.iter().map(|index| hand[*index]).collect()
    }

    pub fn visible_human_hand_len(&self, player_index: usize) -> usize {
        self.visible_hand_indices(player_index).len()
    }

    pub fn visible_human_card(&self, player_index: usize, visible_index: usize) -> Option<Card> {
        let hand_index = self
            .visible_hand_indices(player_index)
            .get(visible_index)
            .copied()?;
        self.human_hand(player_index)?.get(hand_index).copied()
    }

    pub fn visible_human_window(&self, player_index: usize) -> (usize, Vec<Card>) {
        let indices = self.visible_hand_indices(player_index);
        let selected = self.selected_cards[player_index].min(indices.len().saturating_sub(1));
        let start = selected.saturating_sub(HAND_WINDOW_RADIUS);
        let end = indices
            .len()
            .min(selected.saturating_add(HAND_WINDOW_RADIUS + 1));
        let hand = self.human_hand(player_index).unwrap_or_default();
        let cards = indices[start..end]
            .iter()
            .map(|index| hand[*index])
            .collect();
        (start, cards)
    }

    fn visible_hand_indices(&self, player_index: usize) -> Ref<'_, Vec<usize>> {
        if !self.visible_hand_cache.borrow().valid[player_index] {
            let indices = self
                .human_hand(player_index)
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter_map(|(index, card)| self.hand_filter.matches(*card).then_some(index))
                .collect();
            let mut cache = self.visible_hand_cache.borrow_mut();
            cache.indices[player_index] = indices;
            cache.valid[player_index] = true;
        }
        Ref::map(self.visible_hand_cache.borrow(), |cache| {
            &cache.indices[player_index]
        })
    }

    fn invalidate_visible_hands(&self) {
        self.visible_hand_cache.borrow_mut().valid = [false; 2];
    }

    fn reset_page_presses(&mut self) {
        self.page_up_presses = [0; 2];
        self.page_down_presses = [0; 2];
    }

    pub fn human_hand_page(&self, player_index: usize) -> Option<(usize, usize, usize, usize)> {
        let game = self.game.as_ref()?;
        let player = &self.human_ids[player_index];
        let total = game.hand_len_for(player).ok()?;
        let (page, pages, concrete) = game.hand_page_for(player).ok()?;
        Some((page, pages, concrete, total))
    }

    pub fn current_human_index(&self) -> Option<usize> {
        let current = self.game.as_ref()?.current_player();
        self.human_ids[..self.setup.mode.human_count()]
            .iter()
            .position(|id| id == current)
    }

    pub fn selected_human_card(&self) -> Option<Card> {
        let player_index = self.current_human_index()?;
        self.visible_human_card(player_index, self.selected_cards[player_index])
    }

    pub fn is_human(&self, player: &PlayerId) -> bool {
        self.human_ids[..self.setup.mode.human_count()].contains(player)
    }

    fn update_turn_status(&mut self) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if let Some(player_index) = self.current_human_index() {
            let name = game
                .public_state()
                .players
                .into_iter()
                .find(|player| player.id == self.human_ids[player_index])
                .map(|player| player.name)
                .unwrap_or_default();
            self.status = self.language.turn_status(&name);
        } else {
            self.status = self.language.text(Message::Thinking).to_owned();
            self.ai_deadline = Instant::now() + AI_DELAY;
        }
    }
}

/// Maps unmodified lowercase Vim movement keys to their arrow-key equivalents.
/// Arrow keys and every non-navigation key pass through unchanged.
fn navigation_code(key: KeyEvent) -> KeyCode {
    if key.modifiers != KeyModifiers::NONE {
        return key.code;
    }
    match key.code {
        KeyCode::Char('h') => KeyCode::Left,
        KeyCode::Char('j') => KeyCode::Down,
        KeyCode::Char('k') => KeyCode::Up,
        KeyCode::Char('l') => KeyCode::Right,
        code => code,
    }
}

fn draw_rules_for_match(
    difficulty: Difficulty,
    human_ids: &[PlayerId],
    ai_ids: &[PlayerId],
) -> BTreeMap<PlayerId, PlayerDrawRule> {
    let ai_draw_rule = match difficulty {
        Difficulty::Easy => PlayerDrawRule::ExcludeDrawEightAndSixteen,
        Difficulty::Normal => PlayerDrawRule::ExcludeDrawSixteen,
        Difficulty::Hard => PlayerDrawRule::GuaranteeDrawEightPerSeven,
        Difficulty::Extreme => PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven,
    };
    let mut rules = ai_ids
        .iter()
        .cloned()
        .map(|id| (id, ai_draw_rule))
        .collect::<BTreeMap<_, _>>();
    let human_draw_rule = match difficulty {
        Difficulty::Easy => Some(PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen),
        Difficulty::Normal => Some(PlayerDrawRule::GuaranteeDrawEightPerTwenty),
        Difficulty::Hard | Difficulty::Extreme => None,
    };
    if let Some(rule) = human_draw_rule {
        rules.extend(human_ids.iter().cloned().map(|id| (id, rule)));
    }
    rules
}

fn default_player_names(language: Language) -> [String; 2] {
    match language {
        Language::English => ["Player".to_owned(), "Player 2".to_owned()],
        Language::Chinese => ["玩家".to_owned(), "玩家 2".to_owned()],
    }
}

#[derive(Debug, Eq, PartialEq)]
enum AppCommand {
    Play(usize),
    Draw,
    Pass,
    Help,
    New,
    Quit,
}

impl AppCommand {
    fn parse(input: &str) -> Result<Self, ()> {
        let mut parts = input.split_whitespace();
        let command = parts.next().ok_or(())?.to_ascii_lowercase();
        let parsed = match command.as_str() {
            "play" => Self::Play(parts.next().ok_or(())?.parse().map_err(|_| ())?),
            "draw" => Self::Draw,
            "pass" => Self::Pass,
            "help" => Self::Help,
            "new" => Self::New,
            "quit" => Self::Quit,
            _ => return Err(()),
        };
        if parts.next().is_some() {
            return Err(());
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare_human_seven(app: &mut App) {
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![
                Card::new(Color::Red, Rank::Number(7)),
                Card::new(Color::Blue, Rank::Number(1)),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.selected_cards[0] = 0;
    }

    #[test]
    fn difficulty_assigns_human_guarantees_without_changing_ai_rules() {
        let human = PlayerId::new("human");
        let bots = [PlayerId::new("ai-1"), PlayerId::new("ai-2")];

        let easy = draw_rules_for_match(Difficulty::Easy, std::slice::from_ref(&human), &bots);
        assert_eq!(
            easy[&human],
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
        );
        assert!(
            bots.iter()
                .all(|id| { easy[id] == PlayerDrawRule::ExcludeDrawEightAndSixteen })
        );

        let normal = draw_rules_for_match(Difficulty::Normal, std::slice::from_ref(&human), &bots);
        assert_eq!(normal[&human], PlayerDrawRule::GuaranteeDrawEightPerTwenty);
        assert!(
            bots.iter()
                .all(|id| normal[id] == PlayerDrawRule::ExcludeDrawSixteen)
        );

        for difficulty in [Difficulty::Hard, Difficulty::Extreme] {
            assert!(
                !draw_rules_for_match(difficulty, std::slice::from_ref(&human), &bots)
                    .contains_key(&human)
            );
        }

        let second_human = PlayerId::new("human-2");
        let dual = draw_rules_for_match(
            Difficulty::Easy,
            &[human.clone(), second_human.clone()],
            &bots,
        );
        assert_eq!(dual[&human], dual[&second_human]);
    }

    #[test]
    fn command_parser_accepts_documented_commands() {
        assert_eq!(AppCommand::parse("play 3"), Ok(AppCommand::Play(3)));
        assert_eq!(AppCommand::parse("DRAW"), Ok(AppCommand::Draw));
        assert_eq!(AppCommand::parse("pass"), Ok(AppCommand::Pass));
        assert_eq!(AppCommand::parse("help"), Ok(AppCommand::Help));
        assert_eq!(AppCommand::parse("new"), Ok(AppCommand::New));
        assert_eq!(AppCommand::parse("quit"), Ok(AppCommand::Quit));
        assert!(AppCommand::parse("play nope").is_err());
        assert!(AppCommand::parse("draw now").is_err());
    }

    #[test]
    fn hand_filter_categories_match_the_documented_ranks() {
        let colored = |rank| Card::new(Color::Red, rank);
        for card in [
            colored(Rank::DrawTwo),
            Card::wild(Rank::WildDrawFour),
            colored(Rank::DrawEight),
            Card::wild(Rank::WildDrawSixteen),
        ] {
            assert!(HandFilter::Positive.matches(card));
        }
        for card in [
            Card::wild(Rank::WildDiscardThirtyTwo),
            Card::wild(Rank::WildDiscardSixtyFour),
        ] {
            assert!(HandFilter::Negative.matches(card));
        }
        for rank in [Rank::Number(0), Rank::Number(7)] {
            assert!(HandFilter::SevenZero.matches(colored(rank)));
        }
        for card in [
            Card::wild(Rank::WildFactorial),
            Card::wild(Rank::WildSquareRoot),
        ] {
            assert!(HandFilter::Mathematical.matches(card));
        }
        assert!(!HandFilter::Positive.matches(colored(Rank::Number(7))));
        assert!(!HandFilter::Negative.matches(Card::wild(Rank::WildDrawSixteen)));
        assert!(!HandFilter::SevenZero.matches(colored(Rank::Number(1))));
        assert!(!HandFilter::Mathematical.matches(Card::wild(Rank::WildDrawFour)));
    }

    #[test]
    fn million_card_hand_uses_a_bounded_random_page() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.setup.deck_variant = DeckVariant::Standard;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            std::iter::repeat_n(Card::new(Color::Red, Rank::Number(5)), 1_000_000).collect(),
            Card::new(Color::Red, Rank::Number(1)),
        );
        app.invalidate_visible_hands();
        assert_eq!(
            app.game.as_ref().unwrap().hand_len_for(&human).unwrap(),
            1_000_000
        );
        assert_eq!(app.visible_human_hand_len(0), 200);
        let (first_index, window) = app.visible_human_window(0);
        assert_eq!(first_index, 0);
        assert_eq!(window.len(), 200);
        assert_eq!(
            app.visible_human_card(0, 0),
            Some(Card::new(Color::Red, Rank::Number(5)))
        );
    }

    #[test]
    fn five_boundary_presses_change_random_hand_pages_in_both_directions() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.setup.deck_variant = DeckVariant::Standard;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![Card::new(Color::Red, Rank::Number(5)); 401],
            Card::new(Color::Red, Rank::Number(1)),
        );
        app.invalidate_visible_hands();
        app.selected_cards[0] = 199;

        for _ in 0..4 {
            app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 80);
        }
        assert_eq!(app.human_hand_page(0), Some((0, 3, 200, 401)));
        assert_eq!(app.page_down_presses[0], 4);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        assert_eq!(app.page_down_presses[0], 4);
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 80);

        assert_eq!(app.human_hand_page(0), Some((1, 3, 200, 401)));
        assert_eq!(app.selected_cards[0], 0);
        for _ in 0..5 {
            app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 80);
        }
        assert_eq!(app.human_hand_page(0), Some((0, 3, 200, 401)));
        assert_eq!(app.selected_cards[0], 199);
    }

    #[test]
    fn filter_change_resets_boundary_page_progress() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![Card::new(Color::Red, Rank::Number(5)); 201],
            Card::new(Color::Red, Rank::Number(1)),
        );
        app.invalidate_visible_hands();
        app.selected_cards[0] = 199;
        for _ in 0..4 {
            app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 80);
        }
        assert_eq!(app.page_down_presses[0], 4);

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);

        assert_eq!(app.page_down_presses, [0; 2]);
        assert_eq!(app.page_up_presses, [0; 2]);
    }

    #[test]
    fn f_cycles_filters_and_resets_visible_selections() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 0;
        app.start_match().unwrap();
        let left = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &left,
            vec![
                Card::new(Color::Red, Rank::Number(1)),
                Card::new(Color::Blue, Rank::DrawTwo),
                Card::wild(Rank::WildDiscardThirtyTwo),
                Card::new(Color::Green, Rank::Number(7)),
                Card::wild(Rank::WildFactorial),
                Card::wild(Rank::WildSquareRoot),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.selected_cards = [2, 3];

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);
        assert_eq!(app.hand_filter, HandFilter::Positive);
        assert_eq!(app.selected_cards, [0, 0]);
        assert_eq!(
            app.visible_human_hand(0),
            vec![Card::new(Color::Blue, Rank::DrawTwo)]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT), 80);
        assert_eq!(app.hand_filter, HandFilter::Negative);
        assert_eq!(
            app.visible_human_hand(0),
            vec![Card::wild(Rank::WildDiscardThirtyTwo)]
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);
        assert_eq!(app.hand_filter, HandFilter::SevenZero);
        assert_eq!(
            app.visible_human_hand(0),
            vec![Card::new(Color::Green, Rank::Number(7))]
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);
        assert_eq!(app.hand_filter, HandFilter::Mathematical);
        assert_eq!(
            app.visible_human_hand(0),
            vec![
                Card::wild(Rank::WildFactorial),
                Card::wild(Rank::WildSquareRoot),
            ]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);
        assert_eq!(app.hand_filter, HandFilter::All);
        assert_eq!(app.visible_human_hand(0).len(), 6);

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), 80);
        app.start_match().unwrap();
        assert_eq!(app.hand_filter, HandFilter::All);
    }

    #[test]
    fn play_command_uses_the_visible_filter_index() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![
                Card::new(Color::Red, Rank::Number(1)),
                Card::new(Color::Red, Rank::DrawTwo),
                Card::new(Color::Blue, Rank::Number(7)),
                Card::new(Color::Red, Rank::DrawEight),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.hand_filter = HandFilter::Positive;

        app.run_command("play 3");
        assert_eq!(app.status, app.language.text(Message::InvalidCardIndex));
        assert_eq!(app.selected_cards[0], 0);

        app.run_command("play 2");

        assert_eq!(
            app.game.as_ref().unwrap().public_state().discard_top,
            Card::new(Color::Red, Rank::DrawEight)
        );
        assert_eq!(app.selected_cards[0], 0);
    }

    #[test]
    fn multicolor_number_batch_prompts_with_only_batch_colors_and_uses_the_choice() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        let blue_five = Card::new(Color::Blue, Rank::Number(5));
        let green_five = Card::new(Color::Green, Rank::Number(5));
        let remaining = Card::new(Color::Yellow, Rank::Number(8));
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![blue_five, green_five, remaining],
            Card::new(Color::Blue, Rank::Number(3)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        let pending = app.pending_color.as_ref().unwrap();
        assert_eq!(pending.colors, vec![Color::Green, Color::Blue]);
        assert_eq!(app.human_hand(0).unwrap().len(), 3);

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        let state = app.game.as_ref().unwrap().public_state();
        assert!(app.pending_color.is_none());
        assert_eq!(state.discard_top, green_five);
        assert_eq!(state.active_color, Color::Green);
        assert_eq!(app.human_hand(0).unwrap(), &[remaining]);
    }

    #[test]
    fn same_color_number_batch_plays_without_a_picker() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        let blue_five = Card::new(Color::Blue, Rank::Number(5));
        let remaining = Card::new(Color::Yellow, Rank::Number(8));
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![blue_five, blue_five, remaining],
            Card::new(Color::Blue, Rank::Number(3)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        assert!(app.pending_color.is_none());
        assert_eq!(
            app.game.as_ref().unwrap().public_state().active_color,
            Color::Blue
        );
        assert_eq!(app.human_hand(0).unwrap(), &[remaining]);
    }

    #[test]
    fn g_uses_the_full_hand_and_prompts_only_when_the_batch_ends_in_plus_sixteen() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        let target = app.ai_ids[0].clone();
        let before = app.game.as_ref().unwrap().hand_for(&target).unwrap().len();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![
                Card::new(Color::Red, Rank::DrawTwo),
                Card::wild(Rank::WildDrawSixteen),
                Card::new(Color::Yellow, Rank::Number(1)),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.hand_filter = HandFilter::Negative;

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE), 80);

        assert!(app.pending_plus_batch.is_some());
        assert_eq!(app.human_hand(0).unwrap().len(), 3);
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        assert!(app.pending_plus_batch.is_none());
        assert_eq!(
            app.game.as_ref().unwrap().public_state().active_color,
            Color::Green
        );
        assert_eq!(
            app.game.as_ref().unwrap().hand_for(&target).unwrap().len(),
            before + 18
        );
        assert_eq!(app.human_hand(0).unwrap().len(), 1);
        assert!(
            app.logs
                .back()
                .unwrap()
                .contains("auto-played 2 plus cards")
        );
    }

    #[test]
    fn g_auto_uses_an_intermediate_plus_sixteen_and_reports_when_none_are_playable() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![
                Card::new(Color::Red, Rank::DrawEight),
                Card::wild(Rank::WildDrawSixteen),
                Card::new(Color::Blue, Rank::DrawTwo),
                Card::new(Color::Green, Rank::DrawTwo),
                Card::new(Color::Yellow, Rank::Number(1)),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE), 80);

        assert!(app.pending_plus_batch.is_none());
        assert_eq!(app.human_hand(0).unwrap().len(), 1);
        assert!(
            app.logs
                .back()
                .unwrap()
                .contains("auto-played 4 plus cards")
        );

        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![Card::new(Color::Blue, Rank::DrawTwo)],
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE), 80);
        assert_eq!(app.status, app.language.text(Message::NoPlayablePlusBatch));
    }

    #[test]
    fn g_plays_thirty_two_wild_draw_sixteens_for_a_five_hundred_twelve_penalty() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.setup.deck_variant = DeckVariant::Standard;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        let target = app.ai_ids[0].clone();
        let target_before = app.game.as_ref().unwrap().hand_len_for(&target).unwrap();
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![Card::wild(Rank::WildDrawSixteen); 32],
            Card::new(Color::Red, Rank::Number(5)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE), 80);

        assert_eq!(
            app.pending_plus_batch
                .as_ref()
                .map(|pending| pending.plays.len()),
            Some(32)
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        assert!(app.pending_plus_batch.is_none());
        assert!(app.human_hand(0).unwrap().is_empty());
        assert_eq!(
            app.game.as_ref().unwrap().hand_len_for(&target).unwrap(),
            target_before + 512
        );
        assert!(
            app.logs
                .back()
                .unwrap()
                .contains("auto-played 32 plus cards")
        );
    }

    #[test]
    fn seven_picker_supports_keyboard_cancel_confirm_and_command_play() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 2;
        prepare_human_seven(&mut app);

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);
        assert_eq!(app.pending_seven.as_ref().unwrap().targets.len(), 2);
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        assert_eq!(app.pending_seven.as_ref().unwrap().selected_target, 1);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 80);
        assert!(app.pending_seven.is_none());
        assert_eq!(app.human_hand(0).unwrap().len(), 2);

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE), 80);
        for character in "play 1".chars() {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                80,
            );
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);
        assert!(app.pending_seven.is_some());
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);
        assert!(app.pending_seven.is_none());
        assert!(app.logs.back().unwrap().contains("swapped hands with"));
    }

    #[test]
    fn multicolor_seven_chooses_color_before_the_swap_target() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 2;
        app.start_match().unwrap();
        let human = app.human_ids[0].clone();
        let red_seven = Card::new(Color::Red, Rank::Number(7));
        let blue_seven = Card::new(Color::Blue, Rank::Number(7));
        app.game.as_mut().unwrap().set_test_turn(
            &human,
            vec![
                red_seven,
                blue_seven,
                Card::new(Color::Yellow, Rank::Number(1)),
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);
        assert_eq!(
            app.pending_color.as_ref().unwrap().colors,
            vec![Color::Red, Color::Blue]
        );
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        assert!(app.pending_color.is_none());
        assert_eq!(
            app.pending_seven.as_ref().unwrap().chosen_color,
            Some(Color::Blue)
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);

        let state = app.game.as_ref().unwrap().public_state();
        assert!(app.pending_seven.is_none());
        assert_eq!(state.discard_top, blue_seven);
        assert_eq!(state.active_color, Color::Blue);
    }

    #[test]
    fn dual_seven_picker_only_accepts_current_players_navigation() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 1;
        prepare_human_seven(&mut app);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 80);
        app.pending_seven.as_mut().unwrap().selected_target = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 80);
        assert_eq!(app.pending_seven.as_ref().unwrap().selected_target, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), 80);
        assert_eq!(app.pending_seven.as_ref().unwrap().selected_target, 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), 80);
        assert_eq!(app.pending_seven.as_ref().unwrap().selected_target, 1);
    }

    #[test]
    fn setup_starts_two_to_five_player_game() {
        let mut app = App::new(Language::English);
        assert!(app.setup.seven_zero);
        for bots in 1..=4 {
            app.setup.bot_count = bots;
            app.start_match().unwrap();
            assert_eq!(
                app.game.as_ref().unwrap().public_state().players.len(),
                bots + 1
            );
            assert_eq!(
                app.game.as_ref().unwrap().deck_variant(),
                DeckVariant::Holiday
            );
            assert!(app.game.as_ref().unwrap().house_rules().seven_zero);
            app.return_to_setup();
        }

        app.setup.deck_variant = DeckVariant::Standard;
        app.start_match().unwrap();
        assert_eq!(
            app.game.as_ref().unwrap().deck_variant(),
            DeckVariant::Standard
        );

        app.return_to_setup();
        app.setup.mode = PlayMode::Dual;
        for bots in 0..=3 {
            app.setup.bot_count = bots;
            app.start_match().unwrap();
            assert_eq!(
                app.game.as_ref().unwrap().public_state().players.len(),
                bots + 2
            );
            app.return_to_setup();
        }
    }

    #[test]
    fn setup_adjustments_stay_in_range() {
        let mut app = App::new(Language::English);
        app.setup.selected = 3;
        for _ in 0..10 {
            app.adjust_setup(-1);
        }
        assert_eq!(app.setup.bot_count, 1);
        for _ in 0..10 {
            app.adjust_setup(1);
        }
        assert_eq!(app.setup.bot_count, 4);

        assert_eq!(app.setup.deck_variant, DeckVariant::Holiday);
        app.setup.selected = 5;
        app.adjust_setup(-1);
        assert_eq!(app.setup.deck_variant, DeckVariant::Standard);
        app.adjust_setup(1);
        app.adjust_setup(1);
        assert_eq!(app.setup.deck_variant, DeckVariant::Holiday);

        app.setup.selected = 4;
        for _ in 0..10 {
            app.adjust_setup(1);
        }
        assert_eq!(app.setup.difficulty, Difficulty::Extreme);
        for _ in 0..10 {
            app.adjust_setup(-1);
        }
        assert_eq!(app.setup.difficulty, Difficulty::Easy);

        app.setup.selected = 6;
        app.adjust_setup(-1);
        assert!(!app.setup.seven_zero);
        app.adjust_setup(1);
        assert!(app.setup.seven_zero);

        app.setup.selected = 0;
        app.adjust_setup(1);
        assert_eq!(app.setup.mode, PlayMode::Dual);
        app.setup.selected = 3;
        for _ in 0..10 {
            app.adjust_setup(-1);
        }
        assert_eq!(app.setup.bot_count, 0);
        for _ in 0..10 {
            app.adjust_setup(1);
        }
        assert_eq!(app.setup.bot_count, 3);
    }

    #[test]
    fn setup_language_setting_switches_copy_and_preserves_custom_name() {
        let mut app = App::new(Language::English);
        app.setup.selected = 7;

        app.adjust_setup(1);
        assert_eq!(app.language, Language::Chinese);
        assert_eq!(app.setup.names, ["玩家", "玩家 2"]);

        app.setup.names[0] = "Alex".to_owned();
        app.adjust_setup(-1);
        assert_eq!(app.language, Language::English);
        assert_eq!(app.setup.names[0], "Alex");
        assert_eq!(app.setup.names[1], "Player 2");
    }

    #[test]
    fn setup_graphics_setting_switches_between_text_and_graphics_beta() {
        let mut app = App::new(Language::English);
        assert_eq!(app.setup.graphics, GraphicsChoice::Text);
        app.setup.selected = 8;

        app.adjust_setup(1);
        assert_eq!(app.setup.graphics, GraphicsChoice::GraphicsBeta);
        app.adjust_setup(-1);
        assert_eq!(app.setup.graphics, GraphicsChoice::Text);

        let beta = App::with_graphics(Language::English, GraphicsChoice::GraphicsBeta);
        assert_eq!(beta.setup.graphics, GraphicsChoice::GraphicsBeta);
    }

    #[test]
    fn key_release_events_do_not_repeat_navigation() {
        let mut app = App::new(Language::English);

        app.setup.selected = 2;
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Up, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        assert_eq!(app.setup.selected, 2);

        app.setup.selected = 3;
        app.setup.bot_count = 3;
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        app.handle_key(
            KeyEvent::new_with_kind(
                KeyCode::Char('h'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ),
            80,
        );
        app.handle_key(
            KeyEvent::new_with_kind(
                KeyCode::Char('l'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ),
            80,
        );
        assert_eq!(app.setup.bot_count, 3);
    }

    #[test]
    fn setup_vim_keys_navigate_outside_the_name_field() {
        let mut app = App::new(Language::English);

        for character in ['h', 'j', 'k', 'l'] {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                80,
            );
        }
        assert_eq!(app.setup.names[0], "Playerhjkl");

        app.setup.selected = 4;
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), 80);
        assert_eq!(app.setup.selected, 3);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), 80);
        assert_eq!(app.setup.selected, 4);

        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 80);
        assert_eq!(app.setup.difficulty, Difficulty::Easy);
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), 80);
        assert_eq!(app.setup.difficulty, Difficulty::Normal);

        app.handle_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT), 80);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL), 80);
        assert_eq!(app.setup.selected, 4);
    }

    #[test]
    fn command_mode_keeps_vim_keys_as_text() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE), 80);

        for character in ['h', 'j', 'k', 'l'] {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                80,
            );
        }

        assert!(app.command_mode);
        assert_eq!(app.command, "hjkl");
    }

    #[test]
    fn game_up_and_down_keys_move_between_visual_hand_rows() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.selected_cards[0] = 1;

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 0);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 1);

        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release),
            12,
        );
        assert_eq!(app.selected_cards[0], 1);
    }

    #[test]
    fn game_vim_keys_move_in_all_four_directions() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.selected_cards[0] = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), 12);
        assert_eq!(app.selected_cards[0], 1);
    }

    #[test]
    fn dual_mode_routes_navigation_and_uses_x_to_draw() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 0;
        app.start_match().unwrap();
        app.selected_cards = [1, 1];
        let first_hand_len = app.human_hand(0).unwrap().len();

        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards, [0, 0]);

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards[0], 1);
        assert_eq!(app.human_hand(0).unwrap().len(), first_hand_len);

        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), 80);
        assert_eq!(app.human_hand(0).unwrap().len(), first_hand_len + 1);
    }

    #[test]
    fn dual_mode_arrows_follow_the_current_player_and_fixed_keys_allow_preselection() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 0;
        app.start_match().unwrap();
        assert_eq!(app.current_human_index(), Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards, [1, 0]);
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards, [1, 1]);
        assert_eq!(app.current_human_index(), Some(0));

        let right = app.human_ids[1].clone();
        let right_hand = app
            .game
            .as_ref()
            .unwrap()
            .hand_for(&right)
            .unwrap()
            .to_vec();
        app.game.as_mut().unwrap().set_test_turn(
            &right,
            right_hand,
            Card::new(Color::Red, Rank::Number(5)),
        );
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards, [1, 2]);
    }

    #[test]
    fn dual_mode_arrows_do_nothing_during_an_ai_turn() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.selected_cards = [1, 1];

        let bot = app
            .game
            .as_ref()
            .unwrap()
            .public_state()
            .players
            .into_iter()
            .find(|player| !app.is_human(&player.id))
            .unwrap()
            .id;
        let bot_hand = app.game.as_ref().unwrap().hand_for(&bot).unwrap().to_vec();
        app.game.as_mut().unwrap().set_test_turn(
            &bot,
            bot_hand,
            Card::new(Color::Red, Rank::Number(5)),
        );

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 80);
        assert_eq!(app.selected_cards, [1, 1]);
    }

    #[test]
    fn dual_color_picker_only_accepts_current_players_navigation() {
        let mut app = App::new(Language::English);
        app.setup.mode = PlayMode::Dual;
        app.setup.bot_count = 0;
        app.start_match().unwrap();
        app.pending_color = Some(PendingColor {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
            colors: Color::ALL.to_vec(),
        });
        app.selected_color = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 1);
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 0);
    }

    #[test]
    fn color_picker_accepts_h_and_l_only() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.pending_color = Some(PendingColor {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
            colors: Color::ALL.to_vec(),
        });
        app.selected_color = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), 80);
        assert_eq!(app.selected_color, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), 80);
        app.handle_key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT), 80);
        assert_eq!(app.selected_color, 1);
    }
}
