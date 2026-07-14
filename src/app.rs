//! * STAR CARNIVAL APP *
//!
//! Setup, input, local turns, and Holiday color selection.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use crate::ai::{Difficulty, choose_action};
use crate::core::{
    Action, Card, Color, DeckVariant, EventKind, Game, GameEvent, PlayerDrawRule, PlayerId,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::graphics::GraphicsChoice;
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

#[derive(Debug)]
pub struct Setup {
    pub name: String,
    pub bot_count: usize,
    pub difficulty: Difficulty,
    pub deck_variant: DeckVariant,
    pub graphics: GraphicsChoice,
    pub selected: usize,
}

impl Setup {
    fn new(language: Language) -> Self {
        Self {
            name: language.default_player_name().to_owned(),
            bot_count: 3,
            difficulty: Difficulty::Normal,
            deck_variant: DeckVariant::Holiday,
            graphics: GraphicsChoice::Auto,
            selected: 0,
        }
    }
}

pub struct App {
    pub language: Language,
    pub screen: Screen,
    pub setup: Setup,
    pub game: Option<Game>,
    pub human_id: PlayerId,
    pub ai_ids: Vec<PlayerId>,
    pub selected_card: usize,
    pub command_mode: bool,
    pub command: String,
    pub pending_wild: Option<Card>,
    pub selected_color: usize,
    pub logs: VecDeque<String>,
    pub status: String,
    pub should_exit: bool,
    previous_screen: Screen,
    ai_deadline: Instant,
    ai_rng: StdRng,
}

impl App {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            screen: Screen::Setup,
            setup: Setup::new(language),
            game: None,
            human_id: PlayerId::new("human"),
            ai_ids: Vec::new(),
            selected_card: 0,
            command_mode: false,
            command: String::new(),
            pending_wild: None,
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
        let player_name = if self.setup.name.trim().is_empty() {
            self.language.default_player_name().to_owned()
        } else {
            self.setup.name.trim().to_owned()
        };
        let mut players = vec![(self.human_id.clone(), player_name)];
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
        let player_draw_rules =
            draw_rules_for_match(self.setup.difficulty, &self.human_id, &self.ai_ids);
        self.game = Some(
            if self.setup.deck_variant == DeckVariant::Holiday {
                Game::new_with_draw_rules(players, self.setup.deck_variant, player_draw_rules)
            } else {
                Game::new(players, self.setup.deck_variant)
            }
            .map_err(|error| error.to_string())?,
        );
        self.screen = Screen::Game;
        self.selected_card = 0;
        self.command_mode = false;
        self.pending_wild = None;
        self.logs.clear();
        self.status = self.language.text(Message::YourTurn).to_owned();
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
        if self.screen != Screen::Game || self.pending_wild.is_some() || self.command_mode {
            return;
        }
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if game.public_state().winner.is_some() || game.current_player() == &self.human_id {
            return;
        }
        if Instant::now() < self.ai_deadline {
            return;
        }
        self.take_ai_turn();
    }

    fn handle_setup_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.setup.selected = self.setup.selected.saturating_sub(1),
            KeyCode::Down => self.setup.selected = (self.setup.selected + 1).min(6),
            KeyCode::Left => self.adjust_setup(-1),
            KeyCode::Right => self.adjust_setup(1),
            KeyCode::Enter if self.setup.selected == 6 => {
                if let Err(error) = self.start_match() {
                    self.status = error;
                }
            }
            KeyCode::Enter => self.setup.selected = (self.setup.selected + 1).min(6),
            KeyCode::Backspace if self.setup.selected == 0 => {
                self.setup.name.pop();
            }
            KeyCode::Esc => self.should_exit = true,
            KeyCode::Char(character)
                if self.setup.selected == 0
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.setup.name.chars().count() < 20 =>
            {
                self.setup.name.push(character);
            }
            _ => {}
        }
    }

    fn adjust_setup(&mut self, delta: isize) {
        match self.setup.selected {
            1 => {
                self.setup.bot_count = self
                    .setup
                    .bot_count
                    .saturating_add_signed(delta)
                    .clamp(1, 4);
            }
            2 => {
                let index = Difficulty::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.difficulty)
                    .unwrap_or(1)
                    .saturating_add_signed(delta)
                    .clamp(0, Difficulty::ALL.len() - 1);
                self.setup.difficulty = Difficulty::ALL[index];
            }
            3 => {
                let index = DeckVariant::ALL
                    .iter()
                    .position(|candidate| *candidate == self.setup.deck_variant)
                    .unwrap_or(1)
                    .saturating_add_signed(delta)
                    .clamp(0, DeckVariant::ALL.len() - 1);
                self.setup.deck_variant = DeckVariant::ALL[index];
            }
            4 => {
                let old_language = self.language;
                let index = Language::ALL
                    .iter()
                    .position(|candidate| *candidate == self.language)
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .clamp(0, Language::ALL.len() - 1);
                self.language = Language::ALL[index];
                if self.setup.name == old_language.default_player_name() {
                    self.setup.name = self.language.default_player_name().to_owned();
                }
            }
            5 => {
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
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                let row_delta = if key.code == KeyCode::Up { -1 } else { 1 };
                let selected_card = crate::ui::adjacent_hand_card(
                    self.language,
                    self.human_hand().unwrap_or_default(),
                    self.selected_card,
                    terminal_width.saturating_sub(2) as usize,
                    row_delta,
                );
                self.selected_card = selected_card;
            }
            KeyCode::Left => self.selected_card = self.selected_card.saturating_sub(1),
            KeyCode::Right => {
                let len = self.human_hand().map_or(0, <[Card]>::len);
                if len > 0 {
                    self.selected_card = (self.selected_card + 1).min(len - 1);
                }
            }
            KeyCode::Enter => self.play_selected(),
            KeyCode::Char('d' | 'D') => self.submit_human(Action::Draw),
            KeyCode::Char('p' | 'P') => self.submit_human(Action::Pass),
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command.clear();
            }
            KeyCode::Char('?') => self.open_help(),
            KeyCode::Char('q' | 'Q') => self.open_quit(),
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
        match key.code {
            KeyCode::Left => self.selected_color = self.selected_color.saturating_sub(1),
            KeyCode::Right => self.selected_color = (self.selected_color + 1).min(3),
            KeyCode::Esc => self.pending_wild = None,
            KeyCode::Enter => {
                if let Some(card) = self.pending_wild.take() {
                    self.submit_human(Action::Play {
                        card,
                        chosen_color: Some(Color::ALL[self.selected_color]),
                    });
                }
            }
            _ => {}
        }
    }

    fn play_selected(&mut self) {
        let Some(card) = self
            .human_hand()
            .and_then(|hand| hand.get(self.selected_card))
            .copied()
        else {
            self.status = self.language.text(Message::InvalidCardIndex).to_owned();
            return;
        };
        if card.is_wild() {
            self.pending_wild = Some(card);
            self.selected_color = 0;
        } else {
            self.submit_human(Action::Play {
                card,
                chosen_color: None,
            });
        }
    }

    fn submit_human(&mut self, action: Action) {
        let result = self
            .game
            .as_mut()
            .expect("game screen has game")
            .apply_action(&self.human_id, action);
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
            EventKind::CardPlayed { card, .. } => format!(
                "{name} {} {}",
                self.language.text(Message::Played),
                self.language.card(card)
            ),
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
        let hand_len = self.human_hand().map_or(0, <[Card]>::len);
        self.selected_card = self.selected_card.min(hand_len.saturating_sub(1));
        if state.winner.is_some() {
            self.screen = Screen::Result;
        } else if state.current_player == self.human_id {
            self.status = self.language.text(Message::YourTurn).to_owned();
        } else {
            self.status = self.language.text(Message::Thinking).to_owned();
            self.ai_deadline = Instant::now() + AI_DELAY;
        }
    }

    fn run_command(&mut self, input: &str) {
        match AppCommand::parse(input) {
            Ok(AppCommand::Play(index)) => {
                if index == 0 {
                    self.status = self.language.text(Message::InvalidCardIndex).to_owned();
                    return;
                }
                self.selected_card = index - 1;
                self.play_selected();
            }
            Ok(AppCommand::Draw) => self.submit_human(Action::Draw),
            Ok(AppCommand::Pass) => self.submit_human(Action::Pass),
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
        self.logs.clear();
        self.status.clear();
    }

    pub fn human_hand(&self) -> Option<&[Card]> {
        self.game
            .as_ref()
            .and_then(|game| game.hand_for(&self.human_id).ok())
    }
}

fn draw_rules_for_match(
    difficulty: Difficulty,
    human_id: &PlayerId,
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
        rules.insert(human_id.clone(), rule);
    }
    rules
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

    #[test]
    fn difficulty_assigns_human_guarantees_without_changing_ai_rules() {
        let human = PlayerId::new("human");
        let bots = [PlayerId::new("ai-1"), PlayerId::new("ai-2")];

        let easy = draw_rules_for_match(Difficulty::Easy, &human, &bots);
        assert_eq!(
            easy[&human],
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
        );
        assert!(
            bots.iter()
                .all(|id| { easy[id] == PlayerDrawRule::ExcludeDrawEightAndSixteen })
        );

        let normal = draw_rules_for_match(Difficulty::Normal, &human, &bots);
        assert_eq!(normal[&human], PlayerDrawRule::GuaranteeDrawEightPerTwenty);
        assert!(
            bots.iter()
                .all(|id| normal[id] == PlayerDrawRule::ExcludeDrawSixteen)
        );

        for difficulty in [Difficulty::Hard, Difficulty::Extreme] {
            assert!(!draw_rules_for_match(difficulty, &human, &bots).contains_key(&human));
        }
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
    fn setup_starts_two_to_five_player_game() {
        let mut app = App::new(Language::English);
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
            app.return_to_setup();
        }

        app.setup.deck_variant = DeckVariant::Standard;
        app.start_match().unwrap();
        assert_eq!(
            app.game.as_ref().unwrap().deck_variant(),
            DeckVariant::Standard
        );
    }

    #[test]
    fn setup_adjustments_stay_in_range() {
        let mut app = App::new(Language::English);
        app.setup.selected = 1;
        for _ in 0..10 {
            app.adjust_setup(-1);
        }
        assert_eq!(app.setup.bot_count, 1);
        for _ in 0..10 {
            app.adjust_setup(1);
        }
        assert_eq!(app.setup.bot_count, 4);

        assert_eq!(app.setup.deck_variant, DeckVariant::Holiday);
        app.setup.selected = 3;
        app.adjust_setup(-1);
        assert_eq!(app.setup.deck_variant, DeckVariant::Standard);
        app.adjust_setup(1);
        app.adjust_setup(1);
        assert_eq!(app.setup.deck_variant, DeckVariant::Holiday);

        app.setup.selected = 2;
        for _ in 0..10 {
            app.adjust_setup(1);
        }
        assert_eq!(app.setup.difficulty, Difficulty::Extreme);
        for _ in 0..10 {
            app.adjust_setup(-1);
        }
        assert_eq!(app.setup.difficulty, Difficulty::Easy);
    }

    #[test]
    fn setup_language_setting_switches_copy_and_preserves_custom_name() {
        let mut app = App::new(Language::English);
        app.setup.selected = 4;

        app.adjust_setup(1);
        assert_eq!(app.language, Language::Chinese);
        assert_eq!(app.setup.name, "玩家");

        app.setup.name = "Alex".to_owned();
        app.adjust_setup(-1);
        assert_eq!(app.language, Language::English);
        assert_eq!(app.setup.name, "Alex");
    }

    #[test]
    fn setup_graphics_setting_switches_between_auto_and_text() {
        let mut app = App::new(Language::English);
        assert_eq!(app.setup.graphics, GraphicsChoice::Auto);
        app.setup.selected = 5;

        app.adjust_setup(1);
        assert_eq!(app.setup.graphics, GraphicsChoice::Text);
        app.adjust_setup(-1);
        assert_eq!(app.setup.graphics, GraphicsChoice::Auto);
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

        app.setup.selected = 1;
        app.setup.bot_count = 3;
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Release),
            80,
        );
        assert_eq!(app.setup.bot_count, 3);
    }

    #[test]
    fn game_up_and_down_keys_move_between_visual_hand_rows() {
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        app.selected_card = 1;

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 12);
        assert_eq!(app.selected_card, 0);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 12);
        assert_eq!(app.selected_card, 1);

        app.handle_key(
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release),
            12,
        );
        assert_eq!(app.selected_card, 1);
    }
}
