//! * STAR CARNIVAL CORE *
//!
//! UNO cards, deck variants, rules, turn state, and game events.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 5;
pub const STARTING_HAND_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiDrawRule {
    ExcludeDrawEightAndSixteen,
    ExcludeDrawSixteen,
    GuaranteeDrawEightPerSeven,
    TwoDrawEightAndOneSixteenPerSeven,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AiDrawState {
    rule: AiDrawRule,
    received: usize,
}

// ===== * DECK VARIANTS * =====

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DeckVariant {
    Standard,
    #[default]
    Holiday,
}

impl DeckVariant {
    pub const ALL: [Self; 2] = [Self::Standard, Self::Holiday];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
}

impl Color {
    pub const ALL: [Self; 4] = [Self::Red, Self::Yellow, Self::Green, Self::Blue];
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Red => "red",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
        })
    }
}

impl std::str::FromStr for Color {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "red" | "r" => Ok(Self::Red),
            "yellow" | "y" => Ok(Self::Yellow),
            "green" | "g" => Ok(Self::Green),
            "blue" | "b" => Ok(Self::Blue),
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
    DrawEight,
    Wild,
    WildDrawFour,
    WildDrawSixteen,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{number}"),
            Self::Skip => f.write_str("skip"),
            Self::Reverse => f.write_str("reverse"),
            Self::DrawTwo => f.write_str("draw-two"),
            Self::DrawEight => f.write_str("draw-eight"),
            Self::Wild => f.write_str("wild"),
            Self::WildDrawFour => f.write_str("wild-draw-four"),
            Self::WildDrawSixteen => f.write_str("wild-draw-sixteen"),
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

    pub fn is_wild(self) -> bool {
        matches!(
            self.rank,
            Rank::Wild | Rank::WildDrawFour | Rank::WildDrawSixteen
        )
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
    fn reverse(&mut self) {
        *self = match self {
            Self::Clockwise => Self::CounterClockwise,
            Self::CounterClockwise => Self::Clockwise,
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
struct Player {
    id: PlayerId,
    name: String,
    hand: Vec<Card>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnPhase {
    AwaitingAction,
    Drew(Card),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Play {
        card: Card,
        chosen_color: Option<Color>,
    },
    Draw,
    Pass,
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
    pub has_drawn: bool,
    pub winner: Option<PlayerId>,
    pub next_sequence: u64,
}

#[derive(Debug)]
pub struct Game {
    deck_variant: DeckVariant,
    players: Vec<Player>,
    draw_pile: Vec<Card>,
    discard_pile: Vec<Card>,
    active_color: Color,
    current_index: usize,
    direction: Direction,
    phase: TurnPhase,
    events: Vec<GameEvent>,
    winner: Option<PlayerId>,
    rng: StdRng,
    ai_draw_states: BTreeMap<PlayerId, AiDrawState>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GameError {
    InvalidPlayerCount(usize),
    DuplicatePlayer(PlayerId),
    UnknownPlayer(PlayerId),
    NotPlayerTurn(PlayerId),
    CardNotOwned(Card),
    CardNotPlayable(Card),
    DrawnCardOnly(Card),
    MissingColorChoice,
    UnexpectedColorChoice,
    WildDrawFourNotAllowed,
    InvalidColor(String),
    AlreadyDrew,
    CannotPassBeforeDrawing,
    GameAlreadyWon,
    EmptyDrawPile,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for GameError {}

impl Game {
    pub fn new(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            BTreeMap::new(),
            StdRng::from_entropy(),
        )
    }

    pub fn new_with_ai_draw_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        ai_draw_rules: BTreeMap<PlayerId, AiDrawRule>,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(players, deck_variant, ai_draw_rules, StdRng::from_entropy())
    }

    #[cfg(test)]
    fn new_seeded(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        seed: u64,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            BTreeMap::new(),
            StdRng::seed_from_u64(seed),
        )
    }

    #[cfg(test)]
    fn new_seeded_with_ai_draw_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        ai_draw_rules: BTreeMap<PlayerId, AiDrawRule>,
        seed: u64,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            ai_draw_rules,
            StdRng::seed_from_u64(seed),
        )
    }

    fn new_with_rng(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        ai_draw_rules: BTreeMap<PlayerId, AiDrawRule>,
        mut rng: StdRng,
    ) -> Result<Self, GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players.len()) {
            return Err(GameError::InvalidPlayerCount(players.len()));
        }
        let mut seen = BTreeSet::new();
        for (id, _) in &players {
            if !seen.insert(id.clone()) {
                return Err(GameError::DuplicatePlayer(id.clone()));
            }
        }

        let mut deck = deck(deck_variant);
        deck.shuffle(&mut rng);
        let mut ai_draw_states: BTreeMap<PlayerId, AiDrawState> = ai_draw_rules
            .into_iter()
            .map(|(id, rule)| (id, AiDrawState { rule, received: 0 }))
            .collect();
        let mut player_states = Vec::with_capacity(players.len());
        for (id, name) in players {
            let mut hand = Vec::with_capacity(STARTING_HAND_SIZE);
            for _ in 0..STARTING_HAND_SIZE {
                let card = match ai_draw_states.get(&id).copied() {
                    Some(state) if deck_variant == DeckVariant::Holiday => {
                        draw_card_with_rule(&mut deck, state.rule, state.received, &mut rng)?
                    }
                    _ => deck.pop().ok_or(GameError::EmptyDrawPile)?,
                };
                hand.push(card);
                if let Some(state) = ai_draw_states.get_mut(&id) {
                    state.received += 1;
                }
            }
            player_states.push(Player { id, name, hand });
        }

        let discard_index = deck
            .iter()
            .rposition(|card| matches!(card.rank, Rank::Number(_)))
            .ok_or(GameError::EmptyDrawPile)?;
        let first_discard = deck.swap_remove(discard_index);
        let active_color = first_discard.color.expect("number cards have a color");
        let mut game = Self {
            deck_variant,
            players: player_states,
            draw_pile: deck,
            discard_pile: vec![first_discard],
            active_color,
            current_index: 0,
            direction: Direction::Clockwise,
            phase: TurnPhase::AwaitingAction,
            events: Vec::new(),
            winner: None,
            rng,
            ai_draw_states,
        };
        game.push_event(EventKind::GameStarted);
        Ok(game)
    }

    pub fn current_player(&self) -> &PlayerId {
        &self.players[self.current_index].id
    }

    pub const fn deck_variant(&self) -> DeckVariant {
        self.deck_variant
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
            discard_top: *self.discard_pile.last().expect("discard always has a top"),
            active_color: self.active_color,
            current_player: self.current_player().clone(),
            direction: self.direction,
            has_drawn: matches!(self.phase, TurnPhase::Drew(_)),
            winner: self.winner.clone(),
            next_sequence: self.events.len() as u64,
        }
    }

    pub fn legal_actions(&self, player: &PlayerId) -> Result<Vec<Action>, GameError> {
        self.ensure_turn(player)?;
        if self.winner.is_some() {
            return Err(GameError::GameAlreadyWon);
        }

        let hand = &self.player(player)?.hand;
        let playable: Vec<Card> = match self.phase {
            TurnPhase::AwaitingAction => hand
                .iter()
                .copied()
                .filter(|card| self.is_playable_for(hand, *card))
                .collect(),
            TurnPhase::Drew(drawn) => self
                .is_playable_for(hand, drawn)
                .then_some(drawn)
                .into_iter()
                .collect(),
        };
        let mut actions = Vec::new();
        for card in playable {
            if card.is_wild() {
                actions.extend(Color::ALL.map(|chosen_color| Action::Play {
                    card,
                    chosen_color: Some(chosen_color),
                }));
            } else {
                actions.push(Action::Play {
                    card,
                    chosen_color: None,
                });
            }
        }
        actions.push(match self.phase {
            TurnPhase::AwaitingAction => Action::Draw,
            TurnPhase::Drew(_) => Action::Pass,
        });
        Ok(actions)
    }

    pub fn apply_action(
        &mut self,
        player: &PlayerId,
        action: Action,
    ) -> Result<GameEvent, GameError> {
        self.ensure_turn(player)?;
        if self.winner.is_some() {
            return Err(GameError::GameAlreadyWon);
        }
        match action {
            Action::Play { card, chosen_color } => self.play(player, card, chosen_color),
            Action::Draw => self.draw(player),
            Action::Pass => self.pass(player),
        }
    }

    fn play(
        &mut self,
        player: &PlayerId,
        card: Card,
        chosen_color: Option<Color>,
    ) -> Result<GameEvent, GameError> {
        if let TurnPhase::Drew(drawn) = self.phase
            && card != drawn
        {
            return Err(GameError::DrawnCardOnly(card));
        }
        let player_index = self.player_index(player)?;
        let hand = &self.players[player_index].hand;
        if !hand.contains(&card) {
            return Err(GameError::CardNotOwned(card));
        }
        if !self.is_playable_for(hand, card) {
            return Err(if matches!(card.rank, Rank::WildDrawFour) {
                GameError::WildDrawFourNotAllowed
            } else {
                GameError::CardNotPlayable(card)
            });
        }
        if card.is_wild() && chosen_color.is_none() {
            return Err(GameError::MissingColorChoice);
        }
        if !card.is_wild() && chosen_color.is_some() {
            return Err(GameError::UnexpectedColorChoice);
        }

        // ===== * NUMBER CARNIVAL * =====
        // Number cards may be stacked as a house rule: playing one discards every
        // card with the same number. Keep the explicitly played card on top so
        // its color determines the next legal play.
        if let Rank::Number(number) = card.rank {
            let hand = &mut self.players[player_index].hand;
            let mut stacked = Vec::new();
            hand.retain(|owned| {
                if matches!(owned.rank, Rank::Number(candidate) if candidate == number) {
                    stacked.push(*owned);
                    false
                } else {
                    true
                }
            });
            let selected_index = stacked
                .iter()
                .position(|owned| *owned == card)
                .expect("ownership checked above");
            stacked.remove(selected_index);
            self.discard_pile.extend(stacked);
        } else {
            let hand_index = self.players[player_index]
                .hand
                .iter()
                .position(|owned| *owned == card)
                .expect("ownership checked above");
            self.players[player_index].hand.remove(hand_index);
        }
        self.discard_pile.push(card);
        self.active_color = chosen_color.or(card.color).expect("play color validated");
        self.phase = TurnPhase::AwaitingAction;

        let won = self.players[player_index].hand.is_empty();
        if !won {
            self.apply_card_effect(card);
        }
        let event = self.push_event(EventKind::CardPlayed {
            player: player.clone(),
            card,
            chosen_color,
        });
        if won {
            self.winner = Some(player.clone());
            self.push_event(EventKind::GameWon {
                player: player.clone(),
            });
        }
        Ok(event)
    }

    fn draw(&mut self, player: &PlayerId) -> Result<GameEvent, GameError> {
        if matches!(self.phase, TurnPhase::Drew(_)) {
            return Err(GameError::AlreadyDrew);
        }
        let card = self.draw_card_for(player)?;
        let player_index = self.player_index(player)?;
        self.players[player_index].hand.push(card);
        self.phase = TurnPhase::Drew(card);
        Ok(self.push_event(EventKind::CardDrawn {
            player: player.clone(),
            count: 1,
        }))
    }

    fn pass(&mut self, player: &PlayerId) -> Result<GameEvent, GameError> {
        if !matches!(self.phase, TurnPhase::Drew(_)) {
            return Err(GameError::CannotPassBeforeDrawing);
        }
        self.phase = TurnPhase::AwaitingAction;
        self.advance_turn(1);
        Ok(self.push_event(EventKind::TurnPassed {
            player: player.clone(),
        }))
    }

    // ===== * ACTION CARD FIREWORKS * =====

    fn apply_card_effect(&mut self, card: Card) {
        match card.rank {
            Rank::Reverse => {
                self.direction.reverse();
                self.advance_turn(if self.players.len() == 2 { 2 } else { 1 });
            }
            Rank::Skip => self.advance_turn(2),
            Rank::DrawTwo => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 2);
                self.advance_turn(1);
            }
            Rank::DrawEight => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 8);
                self.advance_turn(1);
            }
            Rank::WildDrawFour => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 4);
                self.advance_turn(1);
            }
            Rank::WildDrawSixteen => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 16);
                self.advance_turn(1);
            }
            Rank::Number(_) | Rank::Wild => self.advance_turn(1),
        }
    }

    fn is_playable_for(&self, hand: &[Card], card: Card) -> bool {
        if matches!(card.rank, Rank::WildDrawFour)
            && hand
                .iter()
                .any(|candidate| candidate.color == Some(self.active_color))
        {
            return false;
        }
        let top = self.discard_pile.last().expect("discard always has a top");
        card.is_wild()
            || card.color == Some(self.active_color)
            || (!top.is_wild() && card.rank == top.rank)
    }

    fn draw_available_cards_to_player(&mut self, player: &PlayerId, count: usize) -> usize {
        let index = self
            .player_index(player)
            .expect("penalty target is always a player");
        let mut drawn = 0;
        for _ in 0..count {
            let Ok(card) = self.draw_card_for(player) else {
                break;
            };
            self.players[index].hand.push(card);
            drawn += 1;
        }
        drawn
    }

    fn draw_card(&mut self) -> Result<Card, GameError> {
        if self.draw_pile.is_empty() {
            if self.discard_pile.len() <= 1 {
                return Err(GameError::EmptyDrawPile);
            }
            let top = self.discard_pile.pop().expect("discard has a top");
            self.draw_pile.append(&mut self.discard_pile);
            self.draw_pile.shuffle(&mut self.rng);
            self.discard_pile.push(top);
        }
        self.draw_pile.pop().ok_or(GameError::EmptyDrawPile)
    }

    fn draw_card_for(&mut self, player: &PlayerId) -> Result<Card, GameError> {
        let Some(state) = self.ai_draw_states.get(player).copied() else {
            return self.draw_card();
        };
        if self.deck_variant != DeckVariant::Holiday {
            return self.draw_card();
        }
        if self.draw_pile.is_empty() && self.discard_pile.len() > 1 {
            let top = self.discard_pile.pop().expect("discard has a top");
            self.draw_pile.append(&mut self.discard_pile);
            self.draw_pile.shuffle(&mut self.rng);
            self.discard_pile.push(top);
        }
        let card = draw_card_with_rule(
            &mut self.draw_pile,
            state.rule,
            state.received,
            &mut self.rng,
        )?;
        self.ai_draw_states
            .get_mut(player)
            .expect("AI draw state still exists")
            .received += 1;
        Ok(card)
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
        self.events.push(event.clone());
        event
    }
}

fn draw_card_with_rule<R: Rng + ?Sized>(
    deck: &mut Vec<Card>,
    rule: AiDrawRule,
    received: usize,
    rng: &mut R,
) -> Result<Card, GameError> {
    let block_position = received % STARTING_HAND_SIZE;
    let required_rank = match rule {
        AiDrawRule::GuaranteeDrawEightPerSeven if block_position == 0 => Some(Rank::DrawEight),
        AiDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position < 2 => {
            Some(Rank::DrawEight)
        }
        AiDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position == 2 => {
            Some(Rank::WildDrawSixteen)
        }
        _ => None,
    };
    if let Some(rank) = required_rank {
        if let Some(index) = deck.iter().rposition(|card| card.rank == rank) {
            return Ok(deck.swap_remove(index));
        }
        return Ok(match rank {
            Rank::DrawEight => Card::new(
                Color::ALL[rng.gen_range(0..Color::ALL.len())],
                Rank::DrawEight,
            ),
            Rank::WildDrawSixteen => Card::wild(Rank::WildDrawSixteen),
            _ => unreachable!("only Holiday cards are guaranteed"),
        });
    }

    let allowed = |card: &Card| match rule {
        AiDrawRule::ExcludeDrawEightAndSixteen => {
            !matches!(card.rank, Rank::DrawEight | Rank::WildDrawSixteen)
        }
        AiDrawRule::ExcludeDrawSixteen => !matches!(card.rank, Rank::WildDrawSixteen),
        AiDrawRule::GuaranteeDrawEightPerSeven => true,
        AiDrawRule::TwoDrawEightAndOneSixteenPerSeven => {
            !matches!(card.rank, Rank::DrawEight | Rank::WildDrawSixteen)
        }
    };
    let index = deck
        .iter()
        .rposition(allowed)
        .ok_or(GameError::EmptyDrawPile)?;
    Ok(deck.swap_remove(index))
}

pub fn deck(variant: DeckVariant) -> Vec<Card> {
    match variant {
        DeckVariant::Standard => standard_deck(),
        DeckVariant::Holiday => holiday_deck(),
    }
}

pub fn standard_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(108);
    for color in Color::ALL {
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
    deck
}

pub fn holiday_deck() -> Vec<Card> {
    let mut deck = standard_deck();
    deck.reserve(10);
    for color in Color::ALL {
        deck.push(Card::new(color, Rank::DrawEight));
        deck.push(Card::new(color, Rank::DrawEight));
    }
    deck.push(Card::wild(Rank::WildDrawSixteen));
    deck.push(Card::wild(Rank::WildDrawSixteen));
    deck
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players(count: usize) -> Vec<(PlayerId, String)> {
        (0..count)
            .map(|index| (PlayerId::new(format!("p{index}")), format!("P{index}")))
            .collect()
    }

    fn game() -> Game {
        Game::new_seeded(players(2), DeckVariant::Standard, 7).unwrap()
    }

    #[test]
    fn standard_deck_has_108_cards() {
        assert_eq!(standard_deck().len(), 108);
    }

    #[test]
    fn holiday_deck_has_exact_expansion_cards() {
        let deck = holiday_deck();
        assert_eq!(deck.len(), 118);
        for color in Color::ALL {
            assert_eq!(
                deck.iter()
                    .filter(|card| **card == Card::new(color, Rank::DrawEight))
                    .count(),
                2
            );
        }
        assert_eq!(
            deck.iter()
                .filter(|card| **card == Card::wild(Rank::WildDrawSixteen))
                .count(),
            2
        );
    }

    #[test]
    fn player_count_is_two_to_five() {
        assert_eq!(
            Game::new_seeded(players(1), DeckVariant::Standard, 1).unwrap_err(),
            GameError::InvalidPlayerCount(1)
        );
        assert!(Game::new_seeded(players(5), DeckVariant::Holiday, 1).is_ok());
        assert_eq!(
            Game::new_seeded(players(6), DeckVariant::Standard, 1).unwrap_err(),
            GameError::InvalidPlayerCount(6)
        );
    }

    #[test]
    fn seed_reproduces_initial_state() {
        let first = Game::new_seeded(players(3), DeckVariant::Holiday, 42).unwrap();
        let second = Game::new_seeded(players(3), DeckVariant::Holiday, 42).unwrap();
        assert_eq!(first.public_state(), second.public_state());
        assert_eq!(
            first.hand_for(&PlayerId::new("p0")),
            second.hand_for(&PlayerId::new("p0"))
        );
    }

    fn ai_rules(rule: AiDrawRule, count: usize) -> BTreeMap<PlayerId, AiDrawRule> {
        (1..count)
            .map(|index| (PlayerId::new(format!("p{index}")), rule))
            .collect()
    }

    #[test]
    fn easy_ai_never_receives_draw_eight_or_sixteen() {
        let mut game = Game::new_seeded_with_ai_draw_rules(
            players(2),
            DeckVariant::Holiday,
            ai_rules(AiDrawRule::ExcludeDrawEightAndSixteen, 2),
            11,
        )
        .unwrap();
        let bot = PlayerId::new("p1");
        assert!(
            game.hand_for(&bot)
                .unwrap()
                .iter()
                .all(|card| !matches!(card.rank, Rank::DrawEight | Rank::WildDrawSixteen))
        );
        for _ in 0..30 {
            let card = game.draw_card_for(&bot).unwrap();
            assert!(!matches!(
                card.rank,
                Rank::DrawEight | Rank::WildDrawSixteen
            ));
        }
    }

    #[test]
    fn normal_ai_never_receives_draw_sixteen() {
        let mut game = Game::new_seeded_with_ai_draw_rules(
            players(2),
            DeckVariant::Holiday,
            ai_rules(AiDrawRule::ExcludeDrawSixteen, 2),
            12,
        )
        .unwrap();
        let bot = PlayerId::new("p1");
        assert!(
            game.hand_for(&bot)
                .unwrap()
                .iter()
                .all(|card| card.rank != Rank::WildDrawSixteen)
        );
        for _ in 0..30 {
            assert_ne!(
                game.draw_card_for(&bot).unwrap().rank,
                Rank::WildDrawSixteen
            );
        }
    }

    #[test]
    fn hard_ai_receives_a_draw_eight_in_each_initial_hand() {
        let game = Game::new_seeded_with_ai_draw_rules(
            players(5),
            DeckVariant::Holiday,
            ai_rules(AiDrawRule::GuaranteeDrawEightPerSeven, 5),
            13,
        )
        .unwrap();
        for index in 1..5 {
            assert!(
                game.hand_for(&PlayerId::new(format!("p{index}")))
                    .unwrap()
                    .iter()
                    .any(|card| card.rank == Rank::DrawEight)
            );
        }
    }

    #[test]
    fn extreme_ai_gets_exact_holiday_ratio_in_every_seven_cards() {
        let mut game = Game::new_seeded_with_ai_draw_rules(
            players(5),
            DeckVariant::Holiday,
            ai_rules(AiDrawRule::TwoDrawEightAndOneSixteenPerSeven, 5),
            14,
        )
        .unwrap();
        for index in 1..5 {
            let bot = PlayerId::new(format!("p{index}"));
            let hand = game.hand_for(&bot).unwrap();
            assert_eq!(
                hand.iter()
                    .filter(|card| card.rank == Rank::DrawEight)
                    .count(),
                2
            );
            assert_eq!(
                hand.iter()
                    .filter(|card| card.rank == Rank::WildDrawSixteen)
                    .count(),
                1
            );
        }

        let bot = PlayerId::new("p1");
        let next_seven: Vec<Card> = (0..7).map(|_| game.draw_card_for(&bot).unwrap()).collect();
        assert_eq!(
            next_seven
                .iter()
                .filter(|card| card.rank == Rank::DrawEight)
                .count(),
            2
        );
        assert_eq!(
            next_seven
                .iter()
                .filter(|card| card.rank == Rank::WildDrawSixteen)
                .count(),
            1
        );
    }

    #[test]
    fn pass_requires_a_draw() {
        let mut game = game();
        let current = game.current_player().clone();
        assert_eq!(
            game.apply_action(&current, Action::Pass).unwrap_err(),
            GameError::CannotPassBeforeDrawing
        );
        game.apply_action(&current, Action::Draw).unwrap();
        assert!(game.apply_action(&current, Action::Pass).is_ok());
    }

    #[test]
    fn only_drawn_card_can_be_played_after_draw() {
        let mut game = game();
        let current = game.current_player().clone();
        let old_card = game.hand_for(&current).unwrap()[0];
        game.apply_action(&current, Action::Draw).unwrap();
        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: old_card,
                    chosen_color: old_card.is_wild().then_some(Color::Red),
                },
            )
            .unwrap_err(),
            GameError::DrawnCardOnly(old_card)
        );
    }

    #[test]
    fn wild_draw_four_is_illegal_with_active_color_in_hand() {
        let mut game = game();
        let current = game.current_player().clone();
        game.active_color = Color::Red;
        game.players[0].hand = vec![
            Card::new(Color::Red, Rank::Number(3)),
            Card::wild(Rank::WildDrawFour),
        ];
        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: Card::wild(Rank::WildDrawFour),
                    chosen_color: Some(Color::Blue),
                },
            )
            .unwrap_err(),
            GameError::WildDrawFourNotAllowed
        );
    }

    #[test]
    fn draw_eight_matches_color_or_rank_and_skips_target() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let selected = Card::new(Color::Red, Rank::DrawEight);
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(3))];
        game.players[0].hand = vec![selected, Card::new(Color::Blue, Rank::Number(1))];
        let before = game.hand_for(&target).unwrap().len();

        game.apply_action(
            &current,
            Action::Play {
                card: selected,
                chosen_color: None,
            },
        )
        .unwrap();

        assert_eq!(game.hand_for(&target).unwrap().len(), before + 8);
        assert_eq!(game.current_player(), &current);

        game.active_color = Color::Blue;
        game.discard_pile = vec![Card::new(Color::Yellow, Rank::DrawEight)];
        game.players[0].hand = vec![Card::new(Color::Green, Rank::DrawEight)];
        assert!(game.legal_actions(&current).unwrap().iter().any(|action| {
            matches!(
                action,
                Action::Play {
                    card: Card {
                        color: Some(Color::Green),
                        rank: Rank::DrawEight
                    },
                    chosen_color: None
                }
            )
        }));
    }

    #[test]
    fn wild_draw_sixteen_is_unrestricted_and_changes_color() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let wild = Card::wild(Rank::WildDrawSixteen);
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(4))];
        game.players[0].hand = vec![
            Card::new(Color::Red, Rank::Number(7)),
            wild,
            Card::new(Color::Blue, Rank::Number(2)),
        ];
        let before = game.hand_for(&target).unwrap().len();

        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: wild,
                    chosen_color: None,
                },
            )
            .unwrap_err(),
            GameError::MissingColorChoice
        );
        game.apply_action(
            &current,
            Action::Play {
                card: wild,
                chosen_color: Some(Color::Green),
            },
        )
        .unwrap();

        assert_eq!(game.active_color, Color::Green);
        assert_eq!(game.hand_for(&target).unwrap().len(), before + 16);
        assert_eq!(game.current_player(), &current);
    }

    #[test]
    fn large_penalty_draws_all_available_cards_without_failing() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let wild = Card::wild(Rank::WildDrawSixteen);
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(4))];
        game.draw_pile = vec![
            Card::new(Color::Yellow, Rank::Number(1)),
            Card::new(Color::Yellow, Rank::Number(2)),
            Card::new(Color::Yellow, Rank::Number(3)),
        ];
        game.players[0].hand = vec![wild, Card::new(Color::Blue, Rank::Number(2))];
        let before = game.hand_for(&target).unwrap().len();

        game.apply_action(
            &current,
            Action::Play {
                card: wild,
                chosen_color: Some(Color::Blue),
            },
        )
        .unwrap();

        // The old discard top also becomes recyclable after the three draw-pile cards.
        assert_eq!(game.hand_for(&target).unwrap().len(), before + 4);
        assert!(game.draw_pile.is_empty());
        assert_eq!(game.discard_pile, vec![wild]);
        assert_eq!(game.current_player(), &current);
    }

    #[test]
    fn final_holiday_card_wins_without_penalty() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let wild = Card::wild(Rank::WildDrawSixteen);
        game.players[0].hand = vec![wild];
        let before = game.hand_for(&target).unwrap().len();

        game.apply_action(
            &current,
            Action::Play {
                card: wild,
                chosen_color: Some(Color::Yellow),
            },
        )
        .unwrap();

        assert_eq!(game.public_state().winner, Some(current));
        assert_eq!(game.hand_for(&target).unwrap().len(), before);
    }

    #[test]
    fn playing_number_stacks_all_cards_with_same_number() {
        let mut game = game();
        let current = game.current_player().clone();
        let selected = Card::new(Color::Blue, Rank::Number(3));
        let other_number = Card::new(Color::Red, Rank::Number(3));
        let remaining = Card::new(Color::Green, Rank::Number(8));
        game.active_color = Color::Blue;
        game.discard_pile = vec![Card::new(Color::Blue, Rank::Number(6))];
        game.players[0].hand = vec![other_number, remaining, selected];

        game.apply_action(
            &current,
            Action::Play {
                card: selected,
                chosen_color: None,
            },
        )
        .unwrap();

        assert_eq!(game.players[0].hand, vec![remaining]);
        assert_eq!(
            game.discard_pile,
            vec![
                Card::new(Color::Blue, Rank::Number(6)),
                other_number,
                selected,
            ]
        );
        assert_eq!(game.active_color, Color::Blue);
    }

    #[test]
    fn number_stack_can_win_round() {
        let mut game = game();
        let current = game.current_player().clone();
        let selected = Card::new(Color::Yellow, Rank::Number(4));
        game.active_color = Color::Yellow;
        game.discard_pile = vec![Card::new(Color::Yellow, Rank::Number(7))];
        game.players[0].hand = vec![selected, Card::new(Color::Red, Rank::Number(4))];

        game.apply_action(
            &current,
            Action::Play {
                card: selected,
                chosen_color: None,
            },
        )
        .unwrap();

        assert_eq!(game.public_state().winner, Some(current));
    }

    #[test]
    fn reverse_skips_opponent_in_two_player_game() {
        let mut game = game();
        let current = game.current_player().clone();
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![
            Card::new(Color::Red, Rank::Reverse),
            Card::new(Color::Blue, Rank::Number(1)),
        ];
        game.apply_action(
            &current,
            Action::Play {
                card: Card::new(Color::Red, Rank::Reverse),
                chosen_color: None,
            },
        )
        .unwrap();
        assert_eq!(game.current_player(), &current);
    }

    #[test]
    fn discard_is_recycled_when_draw_pile_is_empty() {
        let mut game = game();
        let top = Card::new(Color::Blue, Rank::Number(9));
        game.draw_pile.clear();
        game.discard_pile = vec![
            Card::new(Color::Red, Rank::Number(1)),
            Card::new(Color::Green, Rank::Number(2)),
            top,
        ];
        let drawn = game.draw_card().unwrap();
        assert_ne!(drawn, top);
        assert_eq!(game.discard_pile, vec![top]);
    }

    #[test]
    fn final_card_wins_round() {
        let mut game = game();
        let current = game.current_player().clone();
        let card = Card::new(Color::Red, Rank::Number(4));
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(2))];
        game.players[0].hand = vec![card];
        game.apply_action(
            &current,
            Action::Play {
                card,
                chosen_color: None,
            },
        )
        .unwrap();
        assert_eq!(game.public_state().winner, Some(current.clone()));
        assert_eq!(
            game.apply_action(&current, Action::Draw).unwrap_err(),
            GameError::GameAlreadyWon
        );
    }

    #[test]
    fn public_state_hides_card_identities() {
        let game = game();
        assert_eq!(game.public_state().players[0].hand_len, STARTING_HAND_SIZE);
    }
}
