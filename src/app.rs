//! * STAR CARNIVAL APP *
//!
//! Setup, input, local turns, and Holiday color selection.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use crate::ai::{Difficulty, choose_action};
use crate::core::{
    Action, Card, Color, DeckVariant, EventKind, Game, GameEvent, HandEffect, HouseRules,
    PlayerDrawRule, PlayerId, Rank,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::frontend::{GraphicsChoice, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::i18n::{Language, Message};

const AI_DELAY: Duration = Duration::from_secs(1);
const MAX_LOGS: usize = 8;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingWild {
    pub player_index: usize,
    pub card: Card,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingSeven {
    pub player_index: usize,
    pub card: Card,
    pub targets: Vec<PlayerId>,
    pub selected_target: usize,
}

pub struct App {
    pub language: Language,
    pub screen: Screen,
    pub setup: Setup,
    pub game: Option<Game>,
    pub human_ids: [PlayerId; 2],
    pub ai_ids: Vec<PlayerId>,
    pub selected_cards: [usize; 2],
    pub command_mode: bool,
    pub command: String,
    pub pending_wild: Option<PendingWild>,
    pub pending_seven: Option<PendingSeven>,
    pub selected_color: usize,
    pub logs: VecDeque<String>,
    pub status: String,
    pub should_exit: bool,
    previous_screen: Screen,
    ai_deadline: Instant,
    ai_rng: StdRng,
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
            command_mode: false,
            command: String::new(),
            pending_wild: None,
            pending_seven: None,
            selected_color: 0,
            logs: VecDeque::new(),
            status: String::new(),
            should_exit: false,
            previous_screen: Screen::Setup,
            ai_deadline: Instant::now(),
            ai_rng: StdRng::from_entropy(),
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
        self.command_mode = false;
        self.pending_wild = None;
        self.pending_seven = None;
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
            || self.pending_wild.is_some()
            || self.pending_seven.is_some()
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
        if self.pending_wild.is_some() {
            self.handle_color_key(key);
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
                let row_delta = if code == KeyCode::Up { -1 } else { 1 };
                self.selected_cards[player_index] = crate::view::adjacent_hand_card(
                    self.language,
                    self.human_hand(player_index).unwrap_or_default(),
                    self.selected_cards[player_index],
                    hand_width,
                    row_delta,
                );
            }
            KeyCode::Left => {
                self.selected_cards[player_index] =
                    self.selected_cards[player_index].saturating_sub(1);
            }
            KeyCode::Right => {
                let len = self.human_hand(player_index).map_or(0, <[Card]>::len);
                if len > 0 {
                    self.selected_cards[player_index] =
                        (self.selected_cards[player_index] + 1).min(len - 1);
                }
            }
            _ => {}
        }
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
        let Some(pending) = self.pending_wild else {
            return;
        };
        let navigation = self
            .player_navigation(key)
            .filter(|(player_index, _)| *player_index == pending.player_index)
            .map(|(_, code)| code)
            .unwrap_or(key.code);
        match navigation {
            KeyCode::Left => self.selected_color = self.selected_color.saturating_sub(1),
            KeyCode::Right => self.selected_color = (self.selected_color + 1).min(3),
            KeyCode::Esc => self.pending_wild = None,
            KeyCode::Enter => {
                if let Some(pending) = self.pending_wild.take() {
                    self.submit_human(
                        pending.player_index,
                        Action::Play {
                            card: pending.card,
                            chosen_color: Some(Color::ALL[self.selected_color]),
                            swap_target: None,
                        },
                    );
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
                            chosen_color: None,
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
        let Some(card) = self
            .human_hand(player_index)
            .and_then(|hand| hand.get(self.selected_cards[player_index]))
            .copied()
        else {
            self.status = self.language.text(Message::InvalidCardIndex).to_owned();
            return;
        };
        if card.is_wild() {
            self.pending_wild = Some(PendingWild { player_index, card });
            self.selected_color = 0;
        } else if self
            .game
            .as_ref()
            .is_some_and(|game| game.house_rules().seven_zero)
            && card.rank == Rank::Number(7)
        {
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
                targets,
                selected_target: 0,
            });
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

    fn take_ai_turn(&mut self) {
        let (player, action) = {
            let game = self.game.as_ref().expect("game screen has game");
            let player = game.current_player().clone();
            let state = game.public_state();
            let hand = game
                .hand_for(&player)
                .expect("current player is in game")
                .to_vec();
            let legal = game
                .legal_actions(&player)
                .expect("current player has legal actions");
            let action = choose_action(
                self.setup.difficulty,
                &state,
                &hand,
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
        let state = self.game.as_ref().expect("game exists").public_state();
        let name = state
            .players
            .iter()
            .find(|candidate| match &event.kind {
                EventKind::CardPlayed { player, .. }
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
                    None => played,
                }
            }
            EventKind::CardDrawn { .. } => {
                format!("{name} {}", self.language.text(Message::DrewCard))
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
            let hand_len = self.human_hand(player_index).map_or(0, <[Card]>::len);
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
                if index == 0 {
                    self.status = self.language.text(Message::InvalidCardIndex).to_owned();
                    return;
                }
                let Some(player_index) = self.current_human_index() else {
                    return;
                };
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
        self.pending_wild = None;
        self.pending_seven = None;
        self.logs.clear();
        self.status.clear();
    }

    pub fn human_hand(&self, player_index: usize) -> Option<&[Card]> {
        self.game
            .as_ref()
            .and_then(|game| game.hand_for(&self.human_ids[player_index]).ok())
    }

    pub fn current_human_index(&self) -> Option<usize> {
        let current = self.game.as_ref()?.current_player();
        self.human_ids[..self.setup.mode.human_count()]
            .iter()
            .position(|id| id == current)
    }

    pub fn selected_human_card(&self) -> Option<Card> {
        let player_index = self.current_human_index()?;
        self.human_hand(player_index)?
            .get(self.selected_cards[player_index])
            .copied()
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
        app.pending_wild = Some(PendingWild {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
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
        app.pending_wild = Some(PendingWild {
            player_index: 0,
            card: Card::wild(crate::core::Rank::Wild),
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
