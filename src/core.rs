//! * STAR CARNIVAL CORE *
//!
//! UNO cards, deck variants, rules, turn state, and game events.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rand::rngs::{OsRng, StdRng};
use rand::seq::SliceRandom;
use rand::{Rng, RngCore, SeedableRng};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 5;
pub const STARTING_HAND_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HouseRules {
    pub seven_zero: bool,
}

impl Default for HouseRules {
    fn default() -> Self {
        Self { seven_zero: true }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerDrawRule {
    ExcludeDrawEightAndSixteen,
    ExcludeDrawSixteen,
    GuaranteeDrawEightPerSeven,
    TwoDrawEightAndOneSixteenPerSeven,
    GuaranteeDrawEightPerFiveAndSixteenPerTen,
    GuaranteeDrawEightPerTwenty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlayerDrawState {
    rule: PlayerDrawRule,
    received: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RefillSeedSource {
    Runtime,
    #[cfg(test)]
    Deterministic,
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
    WildDiscardThirtyTwo,
    WildDiscardSixtyFour,
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
            Self::WildDiscardThirtyTwo => f.write_str("wild-discard-thirty-two"),
            Self::WildDiscardSixtyFour => f.write_str("wild-discard-sixty-four"),
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
            Rank::Wild
                | Rank::WildDrawFour
                | Rank::WildDrawSixteen
                | Rank::WildDiscardThirtyTwo
                | Rank::WildDiscardSixtyFour
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
        swap_target: Option<PlayerId>,
    },
    Draw,
    Pass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandEffect {
    Swap {
        target: PlayerId,
    },
    Rotate {
        direction: Direction,
    },
    Redistribute {
        discarded: usize,
        distributed: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind {
    GameStarted,
    CardPlayed {
        player: PlayerId,
        card: Card,
        chosen_color: Option<Color>,
        hand_effect: Option<HandEffect>,
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
    house_rules: HouseRules,
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
    refill_seed_source: RefillSeedSource,
    player_draw_states: BTreeMap<PlayerId, PlayerDrawState>,
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
    MissingSwapTarget,
    UnexpectedSwapTarget,
    InvalidSwapTarget(PlayerId),
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
            HouseRules::default(),
            BTreeMap::new(),
            StdRng::from_entropy(),
            RefillSeedSource::Runtime,
        )
    }

    pub fn new_with_house_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        house_rules: HouseRules,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            house_rules,
            BTreeMap::new(),
            StdRng::from_entropy(),
            RefillSeedSource::Runtime,
        )
    }

    pub fn new_with_draw_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        player_draw_rules: BTreeMap<PlayerId, PlayerDrawRule>,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            HouseRules::default(),
            player_draw_rules,
            StdRng::from_entropy(),
            RefillSeedSource::Runtime,
        )
    }

    pub fn new_with_house_rules_and_draw_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        house_rules: HouseRules,
        player_draw_rules: BTreeMap<PlayerId, PlayerDrawRule>,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            house_rules,
            player_draw_rules,
            StdRng::from_entropy(),
            RefillSeedSource::Runtime,
        )
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
            HouseRules::default(),
            BTreeMap::new(),
            StdRng::seed_from_u64(seed),
            RefillSeedSource::Deterministic,
        )
    }

    #[cfg(test)]
    fn new_seeded_with_draw_rules(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        player_draw_rules: BTreeMap<PlayerId, PlayerDrawRule>,
        seed: u64,
    ) -> Result<Self, GameError> {
        Self::new_with_rng(
            players,
            deck_variant,
            HouseRules::default(),
            player_draw_rules,
            StdRng::seed_from_u64(seed),
            RefillSeedSource::Deterministic,
        )
    }

    fn new_with_rng(
        players: Vec<(PlayerId, String)>,
        deck_variant: DeckVariant,
        house_rules: HouseRules,
        player_draw_rules: BTreeMap<PlayerId, PlayerDrawRule>,
        mut rng: StdRng,
        refill_seed_source: RefillSeedSource,
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
        let mut player_draw_states: BTreeMap<PlayerId, PlayerDrawState> = player_draw_rules
            .into_iter()
            .map(|(id, rule)| (id, PlayerDrawState { rule, received: 0 }))
            .collect();
        let mut player_states = Vec::with_capacity(players.len());
        for (id, name) in players {
            let mut hand = Vec::with_capacity(STARTING_HAND_SIZE);
            for _ in 0..STARTING_HAND_SIZE {
                let card = match player_draw_states.get(&id).copied() {
                    Some(state) if deck_variant == DeckVariant::Holiday => {
                        draw_card_with_rule(&mut deck, state.rule, state.received, &mut rng)?
                    }
                    _ => deck.pop().ok_or(GameError::EmptyDrawPile)?,
                };
                hand.push(card);
                if let Some(state) = player_draw_states.get_mut(&id) {
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
            house_rules,
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
            refill_seed_source,
            player_draw_states,
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

    pub const fn house_rules(&self) -> HouseRules {
        self.house_rules
    }

    pub fn hand_for(&self, player: &PlayerId) -> Result<&[Card], GameError> {
        Ok(&self.player(player)?.hand)
    }

    #[cfg(test)]
    pub(crate) fn set_test_turn(&mut self, player: &PlayerId, hand: Vec<Card>, discard_top: Card) {
        let index = self.player_index(player).expect("test player exists");
        self.current_index = index;
        self.players[index].hand = hand;
        self.active_color = discard_top.color.expect("test discard is colored");
        self.discard_pile = vec![discard_top];
        self.phase = TurnPhase::AwaitingAction;
        self.winner = None;
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
                    swap_target: None,
                }));
            } else if self.house_rules.seven_zero && card.rank == Rank::Number(7) {
                actions.extend(
                    self.players
                        .iter()
                        .filter(|candidate| candidate.id != *player)
                        .map(|candidate| Action::Play {
                            card,
                            chosen_color: None,
                            swap_target: Some(candidate.id.clone()),
                        }),
                );
            } else {
                actions.push(Action::Play {
                    card,
                    chosen_color: None,
                    swap_target: None,
                });
            }
        }
        actions.push(match self.phase {
            TurnPhase::AwaitingAction if self.can_draw_card_for(player) => Action::Draw,
            TurnPhase::AwaitingAction | TurnPhase::Drew(_) => Action::Pass,
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
            Action::Play {
                card,
                chosen_color,
                swap_target,
            } => self.play(player, card, chosen_color, swap_target),
            Action::Draw => self.draw(player),
            Action::Pass => self.pass(player),
        }
    }

    fn play(
        &mut self,
        player: &PlayerId,
        card: Card,
        chosen_color: Option<Color>,
        swap_target: Option<PlayerId>,
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
        let requires_swap_target = self.house_rules.seven_zero && card.rank == Rank::Number(7);
        match (requires_swap_target, swap_target.as_ref()) {
            (true, None) => return Err(GameError::MissingSwapTarget),
            (true, Some(target)) if target == player || self.player_index(target).is_err() => {
                return Err(GameError::InvalidSwapTarget(target.clone()));
            }
            (false, Some(_)) => return Err(GameError::UnexpectedSwapTarget),
            _ => {}
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
        let hand_effect = (!won)
            .then(|| self.apply_card_effect(card, swap_target))
            .flatten();
        let event = self.push_event(EventKind::CardPlayed {
            player: player.clone(),
            card,
            chosen_color,
            hand_effect,
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
        if !matches!(self.phase, TurnPhase::Drew(_)) && self.can_draw_card_for(player) {
            return Err(GameError::CannotPassBeforeDrawing);
        }
        self.phase = TurnPhase::AwaitingAction;
        self.advance_turn(1);
        Ok(self.push_event(EventKind::TurnPassed {
            player: player.clone(),
        }))
    }

    // ===== * ACTION CARD FIREWORKS * =====

    fn apply_card_effect(
        &mut self,
        card: Card,
        swap_target: Option<PlayerId>,
    ) -> Option<HandEffect> {
        match card.rank {
            Rank::Reverse => {
                self.direction.reverse();
                self.advance_turn(if self.players.len() == 2 { 2 } else { 1 });
                None
            }
            Rank::Skip => {
                self.advance_turn(2);
                None
            }
            Rank::DrawTwo => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 2);
                self.advance_turn(1);
                None
            }
            Rank::DrawEight => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 8);
                self.advance_turn(1);
                None
            }
            Rank::WildDrawFour => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 4);
                self.advance_turn(1);
                None
            }
            Rank::WildDrawSixteen => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                self.draw_available_cards_to_player(&target, 16);
                self.advance_turn(1);
                None
            }
            Rank::WildDiscardThirtyTwo => {
                self.redistribute_and_discard(44, 12);
                self.advance_turn(2);
                Some(HandEffect::Redistribute {
                    discarded: 32,
                    distributed: 12,
                })
            }
            Rank::WildDiscardSixtyFour => {
                self.redistribute_and_discard(88, 24);
                self.advance_turn(2);
                Some(HandEffect::Redistribute {
                    discarded: 64,
                    distributed: 24,
                })
            }
            Rank::Number(7) if self.house_rules.seven_zero => {
                let target = swap_target.expect("validated seven target");
                let target_index = self
                    .player_index(&target)
                    .expect("validated seven target remains a player");
                self.swap_hands(self.current_index, target_index);
                self.advance_turn(1);
                Some(HandEffect::Swap { target })
            }
            Rank::Number(0) if self.house_rules.seven_zero => {
                let direction = self.direction;
                self.rotate_hands(direction);
                self.advance_turn(1);
                Some(HandEffect::Rotate { direction })
            }
            Rank::Number(_) | Rank::Wild => {
                self.advance_turn(1);
                None
            }
        }
    }

    fn redistribute_and_discard(&mut self, processed_count: usize, distributed_count: usize) {
        let actor = self.current_index;
        let mut indices = (0..self.players[actor].hand.len()).collect::<Vec<_>>();
        indices.shuffle(&mut self.rng);
        indices.truncate(processed_count);
        let selected = indices.into_iter().collect::<BTreeSet<_>>();

        let original_hand = std::mem::take(&mut self.players[actor].hand);
        let mut processed = Vec::with_capacity(processed_count);
        for (index, card) in original_hand.into_iter().enumerate() {
            if selected.contains(&index) {
                processed.push(card);
            } else {
                self.players[actor].hand.push(card);
            }
        }
        processed.shuffle(&mut self.rng);

        let player_count = self.players.len();
        let recipients = (1..player_count)
            .map(|offset| (actor + offset) % player_count)
            .collect::<Vec<_>>();
        for (index, card) in processed.drain(..distributed_count).enumerate() {
            self.players[recipients[index % recipients.len()]]
                .hand
                .push(card);
        }

        let effect_card = self
            .discard_pile
            .pop()
            .expect("played effect card is on the discard pile");
        self.discard_pile.extend(processed);
        self.discard_pile.push(effect_card);
    }

    fn swap_hands(&mut self, first: usize, second: usize) {
        if first < second {
            let (left, right) = self.players.split_at_mut(second);
            std::mem::swap(&mut left[first].hand, &mut right[0].hand);
        } else {
            let (left, right) = self.players.split_at_mut(first);
            std::mem::swap(&mut right[0].hand, &mut left[second].hand);
        }
    }

    fn rotate_hands(&mut self, direction: Direction) {
        let player_count = self.players.len();
        let hands = self
            .players
            .iter_mut()
            .map(|player| std::mem::take(&mut player.hand))
            .collect::<Vec<_>>();
        for (source, hand) in hands.into_iter().enumerate() {
            let target = match direction {
                Direction::Clockwise => (source + 1) % player_count,
                Direction::CounterClockwise => (source + player_count - 1) % player_count,
            };
            self.players[target].hand = hand;
        }
    }

    fn is_playable_for(&self, hand: &[Card], card: Card) -> bool {
        let minimum_hand = match card.rank {
            Rank::WildDiscardThirtyTwo => Some(66),
            Rank::WildDiscardSixtyFour => Some(132),
            _ => None,
        };
        if minimum_hand.is_some_and(|minimum| hand.len() < minimum) {
            return false;
        }
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
        self.refill_draw_pile_if_empty();
        self.draw_pile.pop().ok_or(GameError::EmptyDrawPile)
    }

    fn draw_card_for(&mut self, player: &PlayerId) -> Result<Card, GameError> {
        let Some(state) = self.player_draw_states.get(player).copied() else {
            return self.draw_card();
        };
        if self.deck_variant != DeckVariant::Holiday {
            return self.draw_card();
        }
        self.refill_draw_pile_if_empty();
        let card = draw_card_with_rule(
            &mut self.draw_pile,
            state.rule,
            state.received,
            &mut self.rng,
        )?;
        self.player_draw_states
            .get_mut(player)
            .expect("player draw state still exists")
            .received += 1;
        Ok(card)
    }

    fn can_draw_card_for(&self, player: &PlayerId) -> bool {
        let Some(state) = self.player_draw_states.get(player) else {
            return true;
        };
        if self.deck_variant != DeckVariant::Holiday {
            return true;
        }

        if required_rank_for_rule(state.rule, state.received).is_some() {
            return true;
        }

        let allowed = |card: &Card| card_allowed_for_rule(state.rule, card);
        if self.draw_pile.is_empty() {
            true
        } else {
            self.draw_pile.iter().any(allowed)
        }
    }

    fn refill_draw_pile_if_empty(&mut self) {
        if !self.draw_pile.is_empty() {
            return;
        }

        let seed = match self.refill_seed_source {
            RefillSeedSource::Runtime => runtime_refill_seed(),
            #[cfg(test)]
            RefillSeedSource::Deterministic => {
                let mut seed = [0; 32];
                self.rng.fill_bytes(&mut seed);
                seed
            }
        };
        let mut refill_rng = StdRng::from_seed(seed);
        self.draw_pile = deck(self.deck_variant);
        self.draw_pile.shuffle(&mut refill_rng);
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

fn runtime_refill_seed() -> [u8; 32] {
    let mut seed = [0; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

fn draw_card_with_rule<R: Rng + ?Sized>(
    deck: &mut Vec<Card>,
    rule: PlayerDrawRule,
    received: usize,
    rng: &mut R,
) -> Result<Card, GameError> {
    if let Some(rank) = required_rank_for_rule(rule, received) {
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

    let index = deck
        .iter()
        .rposition(|card| card_allowed_for_rule(rule, card))
        .ok_or(GameError::EmptyDrawPile)?;
    Ok(deck.swap_remove(index))
}

fn required_rank_for_rule(rule: PlayerDrawRule, received: usize) -> Option<Rank> {
    let block_position = received % STARTING_HAND_SIZE;
    let card_number = received + 1;
    match rule {
        PlayerDrawRule::GuaranteeDrawEightPerSeven if block_position == 0 => Some(Rank::DrawEight),
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position < 2 => {
            Some(Rank::DrawEight)
        }
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position == 2 => {
            Some(Rank::WildDrawSixteen)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
            if card_number.is_multiple_of(10) =>
        {
            Some(Rank::WildDrawSixteen)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
            if card_number.is_multiple_of(5) =>
        {
            Some(Rank::DrawEight)
        }
        PlayerDrawRule::GuaranteeDrawEightPerTwenty if card_number.is_multiple_of(20) => {
            Some(Rank::DrawEight)
        }
        _ => None,
    }
}

fn card_allowed_for_rule(rule: PlayerDrawRule, card: &Card) -> bool {
    let high_wild = matches!(
        card.rank,
        Rank::WildDrawSixteen | Rank::WildDiscardThirtyTwo | Rank::WildDiscardSixtyFour
    );
    match rule {
        PlayerDrawRule::ExcludeDrawEightAndSixteen => card.rank != Rank::DrawEight && !high_wild,
        PlayerDrawRule::ExcludeDrawSixteen => !high_wild,
        PlayerDrawRule::GuaranteeDrawEightPerSeven
        | PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
        | PlayerDrawRule::GuaranteeDrawEightPerTwenty => true,
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven => {
            card.rank != Rank::DrawEight && !high_wild
        }
    }
}

pub fn deck(variant: DeckVariant) -> Vec<Card> {
    match variant {
        DeckVariant::Standard => standard_deck(),
        DeckVariant::Holiday => holiday_deck(),
    }
}

pub fn standard_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(112);
    for color in Color::ALL {
        for _ in 0..2 {
            deck.push(Card::new(color, Rank::Number(0)));
        }
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
    deck.reserve(14);
    for color in Color::ALL {
        deck.push(Card::new(color, Rank::DrawEight));
        deck.push(Card::new(color, Rank::DrawEight));
    }
    deck.push(Card::wild(Rank::WildDrawSixteen));
    deck.push(Card::wild(Rank::WildDrawSixteen));
    deck.push(Card::wild(Rank::WildDiscardThirtyTwo));
    deck.push(Card::wild(Rank::WildDiscardThirtyTwo));
    deck.push(Card::wild(Rank::WildDiscardSixtyFour));
    deck.push(Card::wild(Rank::WildDiscardSixtyFour));
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
    fn standard_deck_has_112_cards_and_two_zeros_per_color() {
        let deck = standard_deck();
        assert_eq!(deck.len(), 112);
        for color in Color::ALL {
            assert_eq!(
                deck.iter()
                    .filter(|card| **card == Card::new(color, Rank::Number(0)))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn holiday_deck_has_exact_expansion_cards() {
        let deck = holiday_deck();
        assert_eq!(deck.len(), 126);
        for color in Color::ALL {
            assert_eq!(
                deck.iter()
                    .filter(|card| **card == Card::new(color, Rank::Number(0)))
                    .count(),
                2
            );
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
        assert_eq!(
            deck.iter()
                .filter(|card| **card == Card::wild(Rank::WildDiscardThirtyTwo))
                .count(),
            2
        );
        assert_eq!(
            deck.iter()
                .filter(|card| **card == Card::wild(Rank::WildDiscardSixtyFour))
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

    fn ai_rules(rule: PlayerDrawRule, count: usize) -> BTreeMap<PlayerId, PlayerDrawRule> {
        (1..count)
            .map(|index| (PlayerId::new(format!("p{index}")), rule))
            .collect()
    }

    #[test]
    fn easy_ai_never_receives_draw_eight_or_sixteen() {
        let mut game = Game::new_seeded_with_draw_rules(
            players(2),
            DeckVariant::Holiday,
            ai_rules(PlayerDrawRule::ExcludeDrawEightAndSixteen, 2),
            11,
        )
        .unwrap();
        let bot = PlayerId::new("p1");
        assert!(game.hand_for(&bot).unwrap().iter().all(|card| !matches!(
            card.rank,
            Rank::DrawEight
                | Rank::WildDrawSixteen
                | Rank::WildDiscardThirtyTwo
                | Rank::WildDiscardSixtyFour
        )));
        for _ in 0..30 {
            let card = game.draw_card_for(&bot).unwrap();
            assert!(!matches!(
                card.rank,
                Rank::DrawEight
                    | Rank::WildDrawSixteen
                    | Rank::WildDiscardThirtyTwo
                    | Rank::WildDiscardSixtyFour
            ));
        }
    }

    #[test]
    fn normal_ai_never_receives_draw_sixteen() {
        let mut game = Game::new_seeded_with_draw_rules(
            players(2),
            DeckVariant::Holiday,
            ai_rules(PlayerDrawRule::ExcludeDrawSixteen, 2),
            12,
        )
        .unwrap();
        let bot = PlayerId::new("p1");
        assert!(game.hand_for(&bot).unwrap().iter().all(|card| !matches!(
            card.rank,
            Rank::WildDrawSixteen | Rank::WildDiscardThirtyTwo | Rank::WildDiscardSixtyFour
        )));
        for _ in 0..30 {
            assert!(!matches!(
                game.draw_card_for(&bot).unwrap().rank,
                Rank::WildDrawSixteen | Rank::WildDiscardThirtyTwo | Rank::WildDiscardSixtyFour
            ));
        }
    }

    #[test]
    fn hard_ai_receives_a_draw_eight_in_each_initial_hand() {
        let game = Game::new_seeded_with_draw_rules(
            players(5),
            DeckVariant::Holiday,
            ai_rules(PlayerDrawRule::GuaranteeDrawEightPerSeven, 5),
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
        let mut game = Game::new_seeded_with_draw_rules(
            players(5),
            DeckVariant::Holiday,
            ai_rules(PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven, 5),
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
    fn easy_human_gets_draw_eight_and_sixteen_on_their_guaranteed_cards() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(
            human.clone(),
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen,
        )]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Holiday, rules, 15).unwrap();

        assert_eq!(game.hand_for(&human).unwrap()[4].rank, Rank::DrawEight);
        let cards: Vec<Card> = (8..=20)
            .map(|_| game.draw_card_for(&human).unwrap())
            .collect();
        assert_eq!(cards[2].rank, Rank::WildDrawSixteen);
        assert_eq!(cards[7].rank, Rank::DrawEight);
        assert_eq!(cards[12].rank, Rank::WildDrawSixteen);
    }

    #[test]
    fn normal_human_gets_draw_eight_on_every_twentieth_card() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(human.clone(), PlayerDrawRule::GuaranteeDrawEightPerTwenty)]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Holiday, rules, 16).unwrap();

        let cards: Vec<Card> = (8..=40)
            .map(|_| game.draw_card_for(&human).unwrap())
            .collect();
        assert_eq!(cards[12].rank, Rank::DrawEight);
        assert_eq!(cards[32].rank, Rank::DrawEight);
    }

    #[test]
    fn human_guarantees_allow_random_holiday_cards_between_guarantees() {
        let mut deck = vec![Card::wild(Rank::WildDrawSixteen)];
        let card = draw_card_with_rule(
            &mut deck,
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen,
            0,
            &mut StdRng::seed_from_u64(17),
        )
        .unwrap();
        assert_eq!(card.rank, Rank::WildDrawSixteen);
    }

    #[test]
    fn penalty_draws_advance_the_same_player_guarantee_counter() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(
            human.clone(),
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen,
        )]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Holiday, rules, 18).unwrap();
        game.player_draw_states.get_mut(&human).unwrap().received = 4;
        let before = game.hand_for(&human).unwrap().len();

        assert_eq!(game.draw_available_cards_to_player(&human, 1), 1);
        assert_eq!(game.hand_for(&human).unwrap()[before].rank, Rank::DrawEight);
        assert_eq!(game.player_draw_states[&human].received, 5);
    }

    #[test]
    fn standard_deck_ignores_player_draw_guarantees() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(
            human.clone(),
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen,
        )]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Standard, rules, 19).unwrap();
        for _ in 0..30 {
            assert!(!matches!(
                game.draw_card_for(&human).unwrap().rank,
                Rank::DrawEight
                    | Rank::WildDrawSixteen
                    | Rank::WildDiscardThirtyTwo
                    | Rank::WildDiscardSixtyFour
            ));
        }
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
    fn empty_draw_pile_refills_instead_of_allowing_pass() {
        let mut game = game();
        let current = game.current_player().clone();
        game.draw_pile.clear();
        game.discard_pile.truncate(1);

        assert_eq!(
            game.legal_actions(&current).unwrap().last(),
            Some(&Action::Draw)
        );
        assert_eq!(
            game.apply_action(&current, Action::Pass).unwrap_err(),
            GameError::CannotPassBeforeDrawing
        );
        assert!(game.apply_action(&current, Action::Draw).is_ok());
    }

    #[test]
    fn player_can_pass_when_draw_rule_excludes_every_remaining_card() {
        let mut game = Game::new_seeded(players(2), DeckVariant::Holiday, 20).unwrap();
        let current = game.current_player().clone();
        game.player_draw_states.insert(
            current.clone(),
            PlayerDrawState {
                rule: PlayerDrawRule::ExcludeDrawEightAndSixteen,
                received: 0,
            },
        );
        game.draw_pile = vec![
            Card::new(Color::Red, Rank::DrawEight),
            Card::wild(Rank::WildDrawSixteen),
        ];
        game.discard_pile.truncate(1);

        assert_eq!(
            game.legal_actions(&current).unwrap().last(),
            Some(&Action::Pass)
        );
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
                    swap_target: None,
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
                    swap_target: None,
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
                swap_target: None,
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
                    chosen_color: None,
                    swap_target: None,
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
                    swap_target: None,
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
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.active_color, Color::Green);
        assert_eq!(game.hand_for(&target).unwrap().len(), before + 16);
        assert_eq!(game.current_player(), &current);
    }

    #[test]
    fn discard_wilds_require_their_full_pre_play_hand_threshold() {
        for (rank, minimum) in [
            (Rank::WildDiscardThirtyTwo, 66),
            (Rank::WildDiscardSixtyFour, 132),
        ] {
            let mut game = game();
            let current = game.current_player().clone();
            let card = Card::wild(rank);
            game.players[0].hand = std::iter::once(card)
                .chain(
                    (1..minimum - 1)
                        .map(|number| Card::new(Color::Red, Rank::Number(number as u8 % 10))),
                )
                .collect();
            assert_eq!(game.players[0].hand.len(), minimum - 1);
            assert!(!game.legal_actions(&current).unwrap().iter().any(
                |action| matches!(action, Action::Play { card: candidate, .. } if *candidate == card)
            ));
            let before = game.players[0].hand.clone();
            assert_eq!(
                game.apply_action(
                    &current,
                    Action::Play {
                        card,
                        chosen_color: Some(Color::Blue),
                        swap_target: None,
                    },
                )
                .unwrap_err(),
                GameError::CardNotPlayable(card)
            );
            assert_eq!(game.players[0].hand, before);

            game.players[0]
                .hand
                .push(Card::new(Color::Yellow, Rank::Number(1)));
            assert!(game.legal_actions(&current).unwrap().iter().any(
                |action| matches!(action, Action::Play { card: candidate, .. } if *candidate == card)
            ));
        }
    }

    #[test]
    fn discard_wilds_redistribute_evenly_discard_exactly_and_skip_next_player() {
        for player_count in MIN_PLAYERS..=MAX_PLAYERS {
            for (rank, minimum, processed, distributed, discarded) in [
                (Rank::WildDiscardThirtyTwo, 66, 44, 12, 32),
                (Rank::WildDiscardSixtyFour, 132, 88, 24, 64),
            ] {
                let mut game =
                    Game::new_seeded(players(player_count), DeckVariant::Holiday, 31).unwrap();
                let current = game.current_player().clone();
                let card = Card::wild(rank);
                game.active_color = Color::Red;
                game.discard_pile = vec![Card::new(Color::Red, Rank::Number(3))];
                game.players[0].hand = std::iter::once(card)
                    .chain((1..minimum).map(|number| {
                        Card::new(Color::ALL[number % 4], Rank::Number((number % 10) as u8))
                    }))
                    .collect();
                for player in &mut game.players[1..] {
                    player.hand = vec![Card::new(Color::Blue, Rank::Number(9))];
                }
                let before_total = game
                    .players
                    .iter()
                    .map(|player| player.hand.len())
                    .sum::<usize>();
                let event = game
                    .apply_action(
                        &current,
                        Action::Play {
                            card,
                            chosen_color: Some(Color::Green),
                            swap_target: None,
                        },
                    )
                    .unwrap();

                assert_eq!(game.players[0].hand.len(), minimum - processed - 1);
                let received_each = distributed / (player_count - 1);
                assert!(
                    game.players[1..]
                        .iter()
                        .all(|player| player.hand.len() == received_each + 1)
                );
                assert_eq!(game.discard_pile.len(), discarded + 2);
                assert_eq!(game.discard_pile.last(), Some(&card));
                assert_eq!(game.active_color, Color::Green);
                assert_eq!(game.current_index, 2 % player_count);
                assert_eq!(
                    game.players
                        .iter()
                        .map(|player| player.hand.len())
                        .sum::<usize>(),
                    before_total - discarded - 1
                );
                assert!(matches!(
                    event.kind,
                    EventKind::CardPlayed {
                        hand_effect: Some(HandEffect::Redistribute {
                            discarded: event_discarded,
                            distributed: event_distributed,
                        }),
                        ..
                    } if event_discarded == discarded && event_distributed == distributed
                ));
            }
        }
    }

    #[test]
    fn discard_wild_randomization_is_reproducible() {
        let setup = || {
            let mut game = Game::new_seeded(players(3), DeckVariant::Holiday, 41).unwrap();
            game.players[0].hand = std::iter::once(Card::wild(Rank::WildDiscardThirtyTwo))
                .chain((1..66).map(|number| {
                    Card::new(Color::ALL[number % 4], Rank::Number((number % 10) as u8))
                }))
                .collect();
            game
        };
        let mut first = setup();
        let mut second = setup();
        let player = first.current_player().clone();
        let action = Action::Play {
            card: Card::wild(Rank::WildDiscardThirtyTwo),
            chosen_color: Some(Color::Yellow),
            swap_target: None,
        };
        first.apply_action(&player, action.clone()).unwrap();
        second.apply_action(&player, action).unwrap();

        assert_eq!(first.players, second.players);
        assert_eq!(first.discard_pile, second.discard_pile);
    }

    #[test]
    fn discard_wild_skips_in_counter_clockwise_direction() {
        let mut game = Game::new_seeded(players(4), DeckVariant::Holiday, 42).unwrap();
        let player = game.current_player().clone();
        let card = Card::wild(Rank::WildDiscardThirtyTwo);
        game.direction = Direction::CounterClockwise;
        game.players[0].hand =
            std::iter::once(card)
                .chain((1..66).map(|number| {
                    Card::new(Color::ALL[number % 4], Rank::Number((number % 10) as u8))
                }))
                .collect();

        game.apply_action(
            &player,
            Action::Play {
                card,
                chosen_color: Some(Color::Blue),
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.current_index, 2);
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
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.hand_for(&target).unwrap().len(), before + 16);
        assert_eq!(game.draw_pile.len(), standard_deck().len() - 13);
        assert_eq!(
            game.discard_pile,
            vec![Card::new(Color::Red, Rank::Number(4)), wild]
        );
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
                swap_target: None,
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
                swap_target: None,
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
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.public_state().winner, Some(current));
    }

    #[test]
    fn seven_swaps_remaining_hands_and_requires_another_player() {
        let mut game = Game::new_seeded(players(3), DeckVariant::Standard, 30).unwrap();
        let current = game.current_player().clone();
        let target = game.players[2].id.clone();
        let seven = Card::new(Color::Red, Rank::Number(7));
        let actors_remaining = Card::new(Color::Blue, Rank::Number(1));
        let targets_hand = vec![
            Card::new(Color::Yellow, Rank::Number(3)),
            Card::new(Color::Green, Rank::Number(4)),
        ];
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![seven, actors_remaining];
        game.players[2].hand.clone_from(&targets_hand);

        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: seven,
                    chosen_color: None,
                    swap_target: None,
                },
            )
            .unwrap_err(),
            GameError::MissingSwapTarget
        );
        assert_eq!(game.players[0].hand, vec![seven, actors_remaining]);

        let event = game
            .apply_action(
                &current,
                Action::Play {
                    card: seven,
                    chosen_color: None,
                    swap_target: Some(target.clone()),
                },
            )
            .unwrap();

        assert_eq!(game.players[0].hand, targets_hand);
        assert_eq!(game.players[2].hand, vec![actors_remaining]);
        assert_eq!(game.current_player(), &game.players[1].id);
        assert!(matches!(
            event.kind,
            EventKind::CardPlayed {
                hand_effect: Some(HandEffect::Swap { target: event_target }),
                ..
            } if event_target == target
        ));
    }

    #[test]
    fn seven_rejects_self_and_unexpected_targets_without_mutation() {
        let mut game = game();
        let current = game.current_player().clone();
        let seven = Card::new(Color::Red, Rank::Number(7));
        let other = Card::new(Color::Blue, Rank::Number(2));
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![seven, other];

        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: seven,
                    chosen_color: None,
                    swap_target: Some(current.clone()),
                },
            )
            .unwrap_err(),
            GameError::InvalidSwapTarget(current.clone())
        );
        assert_eq!(game.players[0].hand, vec![seven, other]);

        game.house_rules.seven_zero = false;
        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: seven,
                    chosen_color: None,
                    swap_target: Some(game.players[1].id.clone()),
                },
            )
            .unwrap_err(),
            GameError::UnexpectedSwapTarget
        );
    }

    #[test]
    fn disabled_seven_zero_treats_numbers_normally() {
        let mut game = game();
        let current = game.current_player().clone();
        let seven = Card::new(Color::Red, Rank::Number(7));
        let remaining = Card::new(Color::Blue, Rank::Number(2));
        game.house_rules.seven_zero = false;
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![seven, remaining];
        let target_before = game.players[1].hand.clone();

        let event = game
            .apply_action(
                &current,
                Action::Play {
                    card: seven,
                    chosen_color: None,
                    swap_target: None,
                },
            )
            .unwrap();

        assert_eq!(game.players[0].hand, vec![remaining]);
        assert_eq!(game.players[1].hand, target_before);
        assert!(matches!(
            event.kind,
            EventKind::CardPlayed {
                hand_effect: None,
                ..
            }
        ));
    }

    #[test]
    fn zero_rotates_hands_in_both_directions() {
        for direction in [Direction::Clockwise, Direction::CounterClockwise] {
            let mut game = Game::new_seeded(players(3), DeckVariant::Standard, 31).unwrap();
            let current = game.current_player().clone();
            let zero = Card::new(Color::Red, Rank::Number(0));
            let first = vec![Card::new(Color::Blue, Rank::Number(1))];
            let second = vec![Card::new(Color::Green, Rank::Number(2))];
            let third = vec![Card::new(Color::Yellow, Rank::Number(3))];
            game.direction = direction;
            game.active_color = Color::Red;
            game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
            game.players[0].hand = [vec![zero], first.clone()].concat();
            game.players[1].hand.clone_from(&second);
            game.players[2].hand.clone_from(&third);

            let event = game
                .apply_action(
                    &current,
                    Action::Play {
                        card: zero,
                        chosen_color: None,
                        swap_target: None,
                    },
                )
                .unwrap();

            let expected = match direction {
                Direction::Clockwise => [third.clone(), first.clone(), second.clone()],
                Direction::CounterClockwise => [second.clone(), third.clone(), first.clone()],
            };
            assert_eq!(game.players[0].hand, expected[0]);
            assert_eq!(game.players[1].hand, expected[1]);
            assert_eq!(game.players[2].hand, expected[2]);
            assert!(matches!(
                event.kind,
                EventKind::CardPlayed {
                    hand_effect: Some(HandEffect::Rotate { direction: event_direction }),
                    ..
                } if event_direction == direction
            ));
        }
    }

    #[test]
    fn two_player_zero_swaps_hands() {
        let mut game = game();
        let current = game.current_player().clone();
        let zero = Card::new(Color::Red, Rank::Number(0));
        let first = Card::new(Color::Blue, Rank::Number(1));
        let second = vec![Card::new(Color::Green, Rank::Number(2))];
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![zero, first];
        game.players[1].hand.clone_from(&second);

        game.apply_action(
            &current,
            Action::Play {
                card: zero,
                chosen_color: None,
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.players[0].hand, second);
        assert_eq!(game.players[1].hand, vec![first]);
    }

    #[test]
    fn multi_discard_seven_wins_without_swapping() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let red = Card::new(Color::Red, Rank::Number(7));
        let blue = Card::new(Color::Blue, Rank::Number(7));
        game.active_color = Color::Red;
        game.discard_pile = vec![Card::new(Color::Red, Rank::Number(5))];
        game.players[0].hand = vec![red, blue];
        let target_before = game.players[1].hand.clone();

        let event = game
            .apply_action(
                &current,
                Action::Play {
                    card: red,
                    chosen_color: None,
                    swap_target: Some(target),
                },
            )
            .unwrap();

        assert_eq!(game.public_state().winner, Some(current));
        assert_eq!(game.players[1].hand, target_before);
        assert!(matches!(
            event.kind,
            EventKind::CardPlayed {
                hand_effect: None,
                ..
            }
        ));
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
                swap_target: None,
            },
        )
        .unwrap();
        assert_eq!(game.current_player(), &current);
    }

    #[test]
    fn standard_draw_refills_a_complete_deck_and_preserves_discards() {
        let mut game = game();
        let top = Card::new(Color::Blue, Rank::Number(9));
        game.draw_pile.clear();
        let discards = vec![
            Card::new(Color::Red, Rank::Number(1)),
            Card::new(Color::Green, Rank::Number(2)),
            top,
        ];
        game.discard_pile.clone_from(&discards);

        let drawn = game.draw_card().unwrap();

        assert_eq!(game.draw_pile.len(), standard_deck().len() - 1);
        assert_eq!(
            game.draw_pile
                .iter()
                .filter(|card| card.rank == Rank::Number(0))
                .count()
                + usize::from(drawn.rank == Rank::Number(0)),
            8
        );
        assert_eq!(game.discard_pile, discards);
    }

    #[test]
    fn holiday_draw_refills_a_complete_deck_and_preserves_discards() {
        let mut game = Game::new_seeded(players(2), DeckVariant::Holiday, 21).unwrap();
        let discards = vec![
            Card::new(Color::Yellow, Rank::Number(4)),
            Card::wild(Rank::WildDrawSixteen),
        ];
        game.draw_pile.clear();
        game.discard_pile.clone_from(&discards);

        let drawn = game.draw_card().unwrap();

        assert_eq!(game.draw_pile.len(), holiday_deck().len() - 1);
        assert_eq!(
            game.draw_pile
                .iter()
                .filter(|card| card.rank == Rank::Number(0))
                .count()
                + usize::from(drawn.rank == Rank::Number(0)),
            8
        );
        assert_eq!(game.discard_pile, discards);
    }

    #[test]
    fn seeded_refills_are_reproducible_across_multiple_decks() {
        let mut first = Game::new_seeded(players(2), DeckVariant::Holiday, 22).unwrap();
        let mut second = Game::new_seeded(players(2), DeckVariant::Holiday, 22).unwrap();

        for _ in 0..2 {
            first.draw_pile.clear();
            second.draw_pile.clear();
            let first_cards: Vec<Card> = (0..12).map(|_| first.draw_card().unwrap()).collect();
            let second_cards: Vec<Card> = (0..12).map(|_| second.draw_card().unwrap()).collect();
            assert_eq!(first_cards, second_cards);
            assert_eq!(first.draw_pile.len(), holiday_deck().len() - 12);
            assert_eq!(second.draw_pile.len(), holiday_deck().len() - 12);
        }
    }

    #[test]
    fn holiday_draw_rules_still_apply_after_a_refill() {
        let bot = PlayerId::new("p1");
        let mut game = Game::new_seeded_with_draw_rules(
            players(2),
            DeckVariant::Holiday,
            BTreeMap::from([(bot.clone(), PlayerDrawRule::ExcludeDrawEightAndSixteen)]),
            23,
        )
        .unwrap();
        game.draw_pile.clear();

        for _ in 0..30 {
            let card = game.draw_card_for(&bot).unwrap();
            assert!(!matches!(
                card.rank,
                Rank::DrawEight
                    | Rank::WildDrawSixteen
                    | Rank::WildDiscardThirtyTwo
                    | Rank::WildDiscardSixtyFour
            ));
        }
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
                swap_target: None,
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
