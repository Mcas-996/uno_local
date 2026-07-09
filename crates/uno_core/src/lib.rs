use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS_V1: usize = 5;
pub const FUTURE_MAX_PLAYERS: usize = 10;
pub const STARTING_HAND_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Color::Red => "red",
            Color::Yellow => "yellow",
            Color::Green => "green",
            Color::Blue => "blue",
        })
    }
}

impl std::str::FromStr for Color {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "red" | "r" => Ok(Color::Red),
            "yellow" | "y" => Ok(Color::Yellow),
            "green" | "g" => Ok(Color::Green),
            "blue" | "b" => Ok(Color::Blue),
            _ => Err(GameError::InvalidColor(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Rank {
    Number(u8),
    Skip,
    Reverse,
    DrawTwo,
    Wild,
    WildDrawFour,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rank::Number(n) => write!(f, "{n}"),
            Rank::Skip => f.write_str("skip"),
            Rank::Reverse => f.write_str("reverse"),
            Rank::DrawTwo => f.write_str("draw-two"),
            Rank::Wild => f.write_str("wild"),
            Rank::WildDrawFour => f.write_str("wild-draw-four"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Card {
    pub color: Option<Color>,
    pub rank: Rank,
}

impl Card {
    pub const fn new(color: Color, rank: Rank) -> Self {
        Self {
            color: Some(color),
            rank,
        }
    }

    pub const fn wild(rank: Rank) -> Self {
        Self { color: None, rank }
    }

    pub fn is_wild(&self) -> bool {
        matches!(self.rank, Rank::Wild | Rank::WildDrawFour)
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.color {
            Some(color) => write!(f, "{color}:{}", self.rank),
            None => write!(f, "wild:{}", self.rank),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn reverse(&mut self) {
        *self = match self {
            Direction::Clockwise => Direction::CounterClockwise,
            Direction::CounterClockwise => Direction::Clockwise,
        };
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PlayerId(pub String);

impl PlayerId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub hand: Vec<Card>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Play {
        player: PlayerId,
        card: Card,
        chosen_color: Option<Color>,
    },
    Draw {
        player: PlayerId,
    },
    Pass {
        player: PlayerId,
    },
    ChooseColor {
        player: PlayerId,
        color: Color,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind {
    GameStarted,
    CardPlayed {
        player: PlayerId,
        card: Card,
        chosen_color: Option<Color>,
    },
    CardDrawn {
        player: PlayerId,
        count: usize,
    },
    TurnPassed {
        player: PlayerId,
    },
    ColorChosen {
        player: PlayerId,
        color: Color,
    },
    GameWon {
        player: PlayerId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameEvent {
    pub sequence: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicPlayerState {
    pub id: PlayerId,
    pub name: String,
    pub hand_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicGameState {
    pub players: Vec<PublicPlayerState>,
    pub discard_top: Card,
    pub active_color: Color,
    pub current_player: PlayerId,
    pub direction: Direction,
    pub winner: Option<PlayerId>,
    pub next_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    players: Vec<Player>,
    draw_pile: Vec<Card>,
    discard_pile: Vec<Card>,
    active_color: Color,
    current_index: usize,
    direction: Direction,
    events: Vec<GameEvent>,
    applied_sequences: BTreeSet<u64>,
    winner: Option<PlayerId>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GameError {
    InvalidPlayerCount(usize),
    DuplicatePlayer(PlayerId),
    UnknownPlayer(PlayerId),
    NotPlayerTurn(PlayerId),
    CardNotOwned(Card),
    CardNotPlayable(Card),
    MissingColorChoice,
    InvalidColor(String),
    GameAlreadyWon,
    EmptyDrawPile,
    EventAlreadyApplied(u64),
    EventGap { expected: u64, received: u64 },
    CannotPassBeforeDrawing,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for GameError {}

impl Game {
    pub fn new(players: Vec<(PlayerId, String)>) -> Result<Self, GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS_V1).contains(&players.len()) {
            return Err(GameError::InvalidPlayerCount(players.len()));
        }
        Self::new_unchecked(players)
    }

    pub fn new_debug(players: Vec<(PlayerId, String)>) -> Result<Self, GameError> {
        if !(1..=MAX_PLAYERS_V1).contains(&players.len()) {
            return Err(GameError::InvalidPlayerCount(players.len()));
        }
        Self::new_unchecked(players)
    }

    fn new_unchecked(players: Vec<(PlayerId, String)>) -> Result<Self, GameError> {
        let mut seen = BTreeSet::new();
        for (id, _) in &players {
            if !seen.insert(id.clone()) {
                return Err(GameError::DuplicatePlayer(id.clone()));
            }
        }

        let mut deck = standard_deck();
        let mut player_states = Vec::with_capacity(players.len());
        for (id, name) in players {
            let mut hand = Vec::with_capacity(STARTING_HAND_SIZE);
            for _ in 0..STARTING_HAND_SIZE {
                hand.push(deck.pop().ok_or(GameError::EmptyDrawPile)?);
            }
            player_states.push(Player { id, name, hand });
        }

        let mut first_discard = deck.pop().ok_or(GameError::EmptyDrawPile)?;
        if first_discard.is_wild() {
            first_discard = Card::new(Color::Red, Rank::Number(0));
        }
        let active_color = first_discard.color.expect("non-wild first discard");

        let mut game = Self {
            players: player_states,
            draw_pile: deck,
            discard_pile: vec![first_discard],
            active_color,
            current_index: 0,
            direction: Direction::Clockwise,
            events: Vec::new(),
            applied_sequences: BTreeSet::new(),
            winner: None,
        };
        game.push_event(EventKind::GameStarted);
        Ok(game)
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn next_sequence(&self) -> u64 {
        self.events.len() as u64
    }

    pub fn current_player(&self) -> &PlayerId {
        &self.players[self.current_index].id
    }

    pub fn hand_for(&self, player: &PlayerId) -> Result<&[Card], GameError> {
        Ok(&self.player(player)?.hand)
    }

    pub fn public_state(&self) -> PublicGameState {
        PublicGameState {
            players: self
                .players
                .iter()
                .map(|player| PublicPlayerState {
                    id: player.id.clone(),
                    name: player.name.clone(),
                    hand_len: player.hand.len(),
                })
                .collect(),
            discard_top: *self.discard_pile.last().expect("discard always exists"),
            active_color: self.active_color,
            current_player: self.current_player().clone(),
            direction: self.direction,
            winner: self.winner.clone(),
            next_sequence: self.next_sequence(),
        }
    }

    pub fn apply_command(&mut self, command: Command) -> Result<GameEvent, GameError> {
        if self.winner.is_some() {
            return Err(GameError::GameAlreadyWon);
        }

        match command {
            Command::Play {
                player,
                card,
                chosen_color,
            } => self.play(player, card, chosen_color),
            Command::Draw { player } => self.draw(player),
            Command::Pass { player } => self.pass(player),
            Command::ChooseColor { player, color } => {
                self.ensure_turn(&player)?;
                self.active_color = color;
                Ok(self.push_event(EventKind::ColorChosen { player, color }))
            }
        }
    }

    pub fn apply_event_once(&mut self, event: GameEvent) -> Result<(), GameError> {
        if self.applied_sequences.contains(&event.sequence) {
            return Err(GameError::EventAlreadyApplied(event.sequence));
        }
        let expected = self.next_sequence();
        if event.sequence != expected {
            return Err(GameError::EventGap {
                expected,
                received: event.sequence,
            });
        }
        self.applied_sequences.insert(event.sequence);
        self.events.push(event);
        Ok(())
    }

    pub fn snapshot_hands(&self) -> BTreeMap<PlayerId, Vec<Card>> {
        self.players
            .iter()
            .map(|player| (player.id.clone(), player.hand.clone()))
            .collect()
    }

    fn play(
        &mut self,
        player: PlayerId,
        card: Card,
        chosen_color: Option<Color>,
    ) -> Result<GameEvent, GameError> {
        self.ensure_turn(&player)?;
        if !self.is_playable(card) {
            return Err(GameError::CardNotPlayable(card));
        }
        if card.is_wild() && chosen_color.is_none() {
            return Err(GameError::MissingColorChoice);
        }

        let player_index = self.player_index(&player)?;
        let hand_index = self.players[player_index]
            .hand
            .iter()
            .position(|owned| *owned == card)
            .ok_or(GameError::CardNotOwned(card))?;
        self.players[player_index].hand.remove(hand_index);
        self.discard_pile.push(card);
        self.active_color = chosen_color
            .or(card.color)
            .ok_or(GameError::MissingColorChoice)?;

        let mut winner = None;
        if self.players[player_index].hand.is_empty() {
            winner = Some(player.clone());
            self.winner = Some(player.clone());
        }

        self.apply_card_effect(card)?;
        let played = self.push_event(EventKind::CardPlayed {
            player: player.clone(),
            card,
            chosen_color,
        });
        if let Some(player) = winner {
            self.push_event(EventKind::GameWon { player });
        }
        Ok(played)
    }

    fn draw(&mut self, player: PlayerId) -> Result<GameEvent, GameError> {
        self.ensure_turn(&player)?;
        self.draw_cards_to_player(&player, 1)?;
        Ok(self.push_event(EventKind::CardDrawn { player, count: 1 }))
    }

    fn pass(&mut self, player: PlayerId) -> Result<GameEvent, GameError> {
        self.ensure_turn(&player)?;
        self.advance_turn(1);
        Ok(self.push_event(EventKind::TurnPassed { player }))
    }

    fn apply_card_effect(&mut self, card: Card) -> Result<(), GameError> {
        match card.rank {
            Rank::Reverse => {
                self.direction.reverse();
                self.advance_turn(1);
            }
            Rank::Skip => self.advance_turn(2),
            Rank::DrawTwo => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_cards_to_player(&target, 2)?;
                self.advance_turn(1);
            }
            Rank::WildDrawFour => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_cards_to_player(&target, 4)?;
                self.advance_turn(1);
            }
            Rank::Number(_) | Rank::Wild => self.advance_turn(1),
        }
        Ok(())
    }

    fn is_playable(&self, card: Card) -> bool {
        let top = self.discard_pile.last().expect("discard always exists");
        card.is_wild()
            || card.color == Some(self.active_color)
            || (!top.is_wild() && card.rank == top.rank)
    }

    fn draw_cards_to_player(&mut self, player: &PlayerId, count: usize) -> Result<(), GameError> {
        let index = self.player_index(player)?;
        for _ in 0..count {
            let card = self.draw_pile.pop().ok_or(GameError::EmptyDrawPile)?;
            self.players[index].hand.push(card);
        }
        Ok(())
    }

    fn advance_turn(&mut self, steps: usize) {
        let len = self.players.len();
        for _ in 0..steps {
            self.current_index = match self.direction {
                Direction::Clockwise => (self.current_index + 1) % len,
                Direction::CounterClockwise => (self.current_index + len - 1) % len,
            };
        }
    }

    fn ensure_turn(&self, player: &PlayerId) -> Result<(), GameError> {
        self.player(player)?;
        if self.current_player() != player {
            return Err(GameError::NotPlayerTurn(player.clone()));
        }
        Ok(())
    }

    fn player(&self, player: &PlayerId) -> Result<&Player, GameError> {
        self.players
            .iter()
            .find(|candidate| candidate.id == *player)
            .ok_or_else(|| GameError::UnknownPlayer(player.clone()))
    }

    fn player_index(&self, player: &PlayerId) -> Result<usize, GameError> {
        self.players
            .iter()
            .position(|candidate| candidate.id == *player)
            .ok_or_else(|| GameError::UnknownPlayer(player.clone()))
    }

    fn push_event(&mut self, kind: EventKind) -> GameEvent {
        let event = GameEvent {
            sequence: self.events.len() as u64,
            kind,
        };
        self.applied_sequences.insert(event.sequence);
        self.events.push(event.clone());
        event
    }
}

pub fn standard_deck() -> Vec<Card> {
    let mut deck = Vec::new();
    for color in [Color::Red, Color::Yellow, Color::Green, Color::Blue] {
        deck.push(Card::new(color, Rank::Number(0)));
        for number in 1..=9 {
            deck.push(Card::new(color, Rank::Number(number)));
            deck.push(Card::new(color, Rank::Number(number)));
        }
        for rank in [Rank::Skip, Rank::Reverse, Rank::DrawTwo] {
            deck.push(Card::new(color, rank));
            deck.push(Card::new(color, rank));
        }
    }
    for _ in 0..4 {
        deck.push(Card::wild(Rank::Wild));
        deck.push(Card::wild(Rank::WildDrawFour));
    }
    deterministic_shuffle(&mut deck);
    deck
}

fn deterministic_shuffle(deck: &mut [Card]) {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for i in (1..deck.len()).rev() {
        state ^= state << 7;
        state ^= state >> 9;
        let j = (state as usize) % (i + 1);
        deck.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players(count: usize) -> Vec<(PlayerId, String)> {
        (0..count)
            .map(|index| {
                (
                    PlayerId::new(format!("p{index}")),
                    format!("Player {index}"),
                )
            })
            .collect()
    }

    #[test]
    fn valid_player_count_starts_game() {
        let game = Game::new(players(2)).expect("game starts");
        assert_eq!(game.players().len(), 2);
        assert_eq!(game.players()[0].hand.len(), STARTING_HAND_SIZE);
        assert_eq!(game.events()[0].sequence, 0);
    }

    #[test]
    fn invalid_player_count_is_rejected() {
        assert_eq!(
            Game::new(players(1)).unwrap_err(),
            GameError::InvalidPlayerCount(1)
        );
        assert_eq!(
            Game::new(players(6)).unwrap_err(),
            GameError::InvalidPlayerCount(6)
        );
    }

    #[test]
    fn debug_game_allows_one_player() {
        let game = Game::new_debug(players(1)).expect("debug game starts");
        assert_eq!(game.players().len(), 1);
        assert_eq!(game.current_player(), &PlayerId::new("p0"));
    }

    #[test]
    fn valid_card_play_is_accepted() {
        let mut game = Game::new(players(2)).expect("game starts");
        let current = game.current_player().clone();
        let card = game
            .hand_for(&current)
            .unwrap()
            .iter()
            .copied()
            .find(|card| game.is_playable(*card))
            .unwrap_or(Card::wild(Rank::Wild));
        if !game.hand_for(&current).unwrap().contains(&card) {
            game.players[0].hand.push(card);
        }

        let event = game
            .apply_command(Command::Play {
                player: current.clone(),
                card,
                chosen_color: card.is_wild().then_some(Color::Blue),
            })
            .expect("play accepted");
        assert!(matches!(event.kind, EventKind::CardPlayed { .. }));
    }

    #[test]
    fn invalid_card_play_is_rejected() {
        let mut game = Game::new(players(2)).expect("game starts");
        let current = game.current_player().clone();
        let card = Card::new(Color::Red, Rank::Number(9));
        game.players[0].hand.retain(|owned| *owned != card);
        let error = game
            .apply_command(Command::Play {
                player: current,
                card,
                chosen_color: None,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            GameError::CardNotOwned(_) | GameError::CardNotPlayable(_)
        ));
    }

    #[test]
    fn duplicate_event_is_rejected() {
        let mut game = Game::new(players(2)).expect("game starts");
        let duplicate = game.events()[0].clone();
        assert_eq!(
            game.apply_event_once(duplicate.clone()).unwrap_err(),
            GameError::EventAlreadyApplied(duplicate.sequence)
        );
    }

    #[test]
    fn public_state_hides_other_hands() {
        let game = Game::new(players(2)).expect("game starts");
        let public = game.public_state();
        assert_eq!(public.players[0].hand_len, STARTING_HAND_SIZE);
        assert_eq!(
            game.hand_for(&PlayerId::new("p0")).unwrap().len(),
            STARTING_HAND_SIZE
        );
    }
}
