//! * STAR CARNIVAL CORE *
//!
//! UNO cards, deck variants, rules, turn state, and game events.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use rand::rngs::{OsRng, StdRng};
use rand::seq::SliceRandom;
use rand::{Rng, RngCore, SeedableRng};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 5;
pub const STARTING_HAND_SIZE: usize = 7;
pub const HAND_BATCH_SIZE: usize = 200;

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
    WildFactorial,
    WildSquareRoot,
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
            Self::WildFactorial => f.write_str("wild-factorial"),
            Self::WildSquareRoot => f.write_str("wild-square-root"),
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
                | Rank::WildFactorial
                | Rank::WildSquareRoot
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
    virtual_len: usize,
    hand_page: usize,
    materialized_received: usize,
}

impl Player {
    fn hand_len(&self) -> usize {
        self.hand.len().saturating_add(self.virtual_len)
    }

    fn page_count(&self) -> usize {
        self.hand_len().div_ceil(HAND_BATCH_SIZE).max(1)
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlusPlay {
    pub card: Card,
    pub chosen_color: Option<Color>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IndexedPlusPlay {
    hand_index: usize,
    play: PlusPlay,
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
    Factorial {
        target: PlayerId,
        before: usize,
        after: usize,
    },
    SquareRoot {
        target: PlayerId,
        before: usize,
        after: usize,
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
    PlusBatchPlayed {
        player: PlayerId,
        cards: Vec<Card>,
        target: PlayerId,
        penalty: usize,
        drawn: usize,
        final_color: Color,
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
    InvalidNumberBatchColor(Color),
    MissingSwapTarget,
    UnexpectedSwapTarget,
    InvalidSwapTarget(PlayerId),
    WildDrawFourNotAllowed,
    InvalidColor(String),
    AlreadyDrew,
    CannotPassBeforeDrawing,
    GameAlreadyWon,
    EmptyDrawPile,
    EmptyPlusBatch,
    InvalidPlusBatch,
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
            player_states.push(Player {
                id,
                name,
                hand,
                virtual_len: 0,
                hand_page: 0,
                materialized_received: STARTING_HAND_SIZE,
            });
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

    pub fn hand_len_for(&self, player: &PlayerId) -> Result<usize, GameError> {
        Ok(self.player(player)?.hand_len())
    }

    pub fn hand_page_for(&self, player: &PlayerId) -> Result<(usize, usize, usize), GameError> {
        let player = self.player(player)?;
        let pages = player.page_count();
        Ok((player.hand_page.min(pages - 1), pages, player.hand.len()))
    }

    pub fn materialize_hand_page(
        &mut self,
        player: &PlayerId,
        page: usize,
    ) -> Result<(), GameError> {
        let index = self.player_index(player)?;
        let total = self.players[index].hand_len();
        let page_count = total.div_ceil(HAND_BATCH_SIZE).max(1);
        let page = page.min(page_count - 1);
        let page_len = total
            .saturating_sub(page.saturating_mul(HAND_BATCH_SIZE))
            .min(HAND_BATCH_SIZE);
        let rule = self.player_draw_states.get(player).map(|state| state.rule);
        let received = self.players[index].materialized_received;
        let cards =
            generate_virtual_cards(self.deck_variant, rule, received, page_len, &mut self.rng);
        self.players[index].hand = cards;
        self.players[index].virtual_len = total.saturating_sub(page_len);
        self.players[index].hand_page = page;
        self.players[index].materialized_received = received.saturating_add(page_len);
        Ok(())
    }

    pub fn materialize_next_batch_if_empty(
        &mut self,
        player: &PlayerId,
    ) -> Result<bool, GameError> {
        let owner = self.player(player)?;
        if !owner.hand.is_empty() || owner.hand_len() == 0 {
            return Ok(false);
        }
        let next_page = (owner.hand_page.min(owner.page_count() - 1) + 1) % owner.page_count();
        self.materialize_hand_page(player, next_page)?;
        Ok(true)
    }

    #[cfg(test)]
    pub(crate) fn set_test_turn(
        &mut self,
        player: &PlayerId,
        mut hand: Vec<Card>,
        discard_top: Card,
    ) {
        let index = self.player_index(player).expect("test player exists");
        self.current_index = index;
        let total = hand.len();
        hand.truncate(HAND_BATCH_SIZE);
        self.players[index].hand = hand;
        self.players[index].virtual_len = total.saturating_sub(HAND_BATCH_SIZE);
        self.players[index].hand_page = 0;
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
                    hand_len: player.hand_len(),
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

        let owner = self.player(player)?;
        let hand = &owner.hand;
        let hand_len = owner.hand_len();
        let playable: BTreeSet<Card> = match self.phase {
            TurnPhase::AwaitingAction => hand
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .filter(|card| self.is_playable_for(hand, hand_len, *card))
                .collect(),
            TurnPhase::Drew(drawn) => self
                .is_playable_for(hand, hand_len, drawn)
                .then_some(drawn)
                .into_iter()
                .collect(),
        };
        let mut actions = Vec::new();
        for card in playable {
            let chosen_colors = if card.is_wild() {
                Color::ALL.into_iter().map(Some).collect::<Vec<_>>()
            } else if let Rank::Number(number) = card.rank {
                let colors = number_batch_colors(hand, number);
                if colors.len() > 1 {
                    colors.into_iter().map(Some).collect()
                } else {
                    vec![None]
                }
            } else {
                vec![None]
            };
            let swap_targets = if self.house_rules.seven_zero && card.rank == Rank::Number(7) {
                self.players
                    .iter()
                    .filter(|candidate| candidate.id != *player)
                    .map(|candidate| Some(candidate.id.clone()))
                    .collect::<Vec<_>>()
            } else {
                vec![None]
            };
            for chosen_color in chosen_colors {
                for swap_target in &swap_targets {
                    actions.push(Action::Play {
                        card,
                        chosen_color,
                        swap_target: swap_target.clone(),
                    });
                }
            }
        }
        actions.push(match self.phase {
            TurnPhase::AwaitingAction if self.can_draw_card_for(player) => Action::Draw,
            TurnPhase::AwaitingAction | TurnPhase::Drew(_) => Action::Pass,
        });
        Ok(actions)
    }

    /// Plans the longest legal sequence containing only +2, +8, and WILD +16.
    ///
    /// The returned order is deterministic. Intermediate wild cards already carry
    /// the color needed by the continuation; when the final card is wild its color
    /// is left unset so the frontend can ask the player.
    pub fn best_plus_batch(&self, player: &PlayerId) -> Result<Vec<PlusPlay>, GameError> {
        self.ensure_turn(player)?;
        if self.winner.is_some() {
            return Err(GameError::GameAlreadyWon);
        }

        let hand = &self.player(player)?.hand;
        let mut indexed = hand.iter().copied().enumerate();
        let candidates = match self.phase {
            TurnPhase::AwaitingAction => indexed
                .filter(|(_, card)| is_plus_batch_card(*card))
                .map(indexed_plus_play)
                .collect::<Vec<_>>(),
            TurnPhase::Drew(drawn) if is_plus_batch_card(drawn) => indexed
                .find(|(_, card)| *card == drawn)
                .map(indexed_plus_play)
                .into_iter()
                .collect(),
            TurnPhase::Drew(_) => Vec::new(),
        };
        if candidates.len() > 63 {
            return Err(GameError::InvalidPlusBatch);
        }

        let top = *self.discard_pile.last().expect("discard always has a top");
        let mut memo = HashMap::new();
        let mut best = best_plus_suffix(&candidates, 0, self.active_color, top, &mut memo);
        if let Some(last) = best.last_mut()
            && last.play.card.rank == Rank::WildDrawSixteen
        {
            last.play.chosen_color = None;
        }
        Ok(best.into_iter().map(|entry| entry.play).collect())
    }

    /// Atomically applies a frontend-planned +2/+8/+16 sequence.
    pub fn apply_plus_batch(
        &mut self,
        player: &PlayerId,
        plays: Vec<PlusPlay>,
    ) -> Result<GameEvent, GameError> {
        self.ensure_turn(player)?;
        if self.winner.is_some() {
            return Err(GameError::GameAlreadyWon);
        }
        if plays.is_empty() {
            return Err(GameError::EmptyPlusBatch);
        }
        if let TurnPhase::Drew(drawn) = self.phase
            && (plays.len() != 1 || plays[0].card != drawn)
        {
            return Err(GameError::InvalidPlusBatch);
        }

        let player_index = self.player_index(player)?;
        let mut remaining = self.players[player_index].hand.clone();
        let mut active_color = self.active_color;
        let mut top = *self.discard_pile.last().expect("discard always has a top");
        for play in &plays {
            if !is_plus_batch_card(play.card)
                || !plus_card_is_playable(active_color, top, play.card)
            {
                return Err(GameError::InvalidPlusBatch);
            }
            let Some(owned_index) = remaining.iter().position(|card| *card == play.card) else {
                return Err(GameError::CardNotOwned(play.card));
            };
            remaining.remove(owned_index);
            match (play.card.rank, play.chosen_color) {
                (Rank::WildDrawSixteen, Some(color)) => active_color = color,
                (Rank::WildDrawSixteen, None) => return Err(GameError::MissingColorChoice),
                (_, Some(_)) => return Err(GameError::UnexpectedColorChoice),
                (_, None) => active_color = play.card.color.expect("colored plus card"),
            }
            top = play.card;
        }

        self.players[player_index].hand = remaining;
        self.discard_pile.extend(plays.iter().map(|play| play.card));
        self.active_color = active_color;
        self.phase = TurnPhase::AwaitingAction;
        let penalty = plays.iter().map(|play| plus_penalty(play.card)).sum();
        self.advance_turn(1);
        let target = self.current_player().clone();
        let drawn = self.draw_available_cards_to_player(&target, penalty);
        self.advance_turn(1);

        let won = self.players[player_index].hand_len() == 0;
        let event = self.push_event(EventKind::PlusBatchPlayed {
            player: player.clone(),
            cards: plays.into_iter().map(|play| play.card).collect(),
            target,
            penalty,
            drawn,
            final_color: active_color,
        });
        if won {
            self.winner = Some(player.clone());
            self.push_event(EventKind::GameWon {
                player: player.clone(),
            });
        }
        Ok(event)
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
        let total_len = self.players[player_index].hand_len();
        if !self.is_playable_for(hand, total_len, card) {
            return Err(if matches!(card.rank, Rank::WildDrawFour) {
                GameError::WildDrawFourNotAllowed
            } else {
                GameError::CardNotPlayable(card)
            });
        }
        let number_colors = match card.rank {
            Rank::Number(number) => number_batch_colors(hand, number),
            _ => Vec::new(),
        };
        let final_color = if card.is_wild() {
            chosen_color.ok_or(GameError::MissingColorChoice)?
        } else if number_colors.len() > 1 {
            let chosen = chosen_color.ok_or(GameError::MissingColorChoice)?;
            if !number_colors.contains(&chosen) {
                return Err(GameError::InvalidNumberBatchColor(chosen));
            }
            chosen
        } else {
            if chosen_color.is_some() {
                return Err(GameError::UnexpectedColorChoice);
            }
            card.color.expect("colored card")
        };
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
        // card with the same number. For a multi-color batch, keep a card of the
        // chosen final color on top so the discard and active color stay aligned.
        let top_card = if let Rank::Number(number) = card.rank {
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
            let top_index = stacked
                .iter()
                .rposition(|owned| owned.color == Some(final_color))
                .expect("final color belongs to the number batch");
            let top = stacked.remove(top_index);
            self.discard_pile.extend(stacked);
            top
        } else {
            let hand_index = self.players[player_index]
                .hand
                .iter()
                .position(|owned| *owned == card)
                .expect("ownership checked above");
            self.players[player_index].hand.remove(hand_index);
            card
        };
        self.discard_pile.push(top_card);
        self.active_color = final_color;
        self.phase = TurnPhase::AwaitingAction;

        let won = self.players[player_index].hand_len() == 0;
        let hand_effect = (!won)
            .then(|| self.apply_card_effect(top_card, swap_target))
            .flatten();
        let event = self.push_event(EventKind::CardPlayed {
            player: player.clone(),
            card: top_card,
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
        self.push_concrete_card(player_index, card);
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
            Rank::WildFactorial => {
                self.advance_turn(1);
                let target = self.current_player().clone();
                let target_index = self
                    .player_index(&target)
                    .expect("factorial target is always a player");
                let before = self.players[target_index].hand_len();
                let after = factorial_hand_size(before);
                self.draw_available_cards_to_player(&target, after.saturating_sub(before));
                let after = self.players[target_index].hand_len();
                self.advance_turn(1);
                Some(HandEffect::Factorial {
                    target,
                    before,
                    after,
                })
            }
            Rank::WildSquareRoot => {
                let target = self.current_player().clone();
                let before = self.players[self.current_index].hand_len();
                let after = before.isqrt();
                self.players[self.current_index].hand.shuffle(&mut self.rng);
                let retained = after.min(HAND_BATCH_SIZE);
                let concrete_retained = retained.min(self.players[self.current_index].hand.len());
                let discarded = self.players[self.current_index]
                    .hand
                    .split_off(concrete_retained);
                self.players[self.current_index].virtual_len =
                    after.saturating_sub(self.players[self.current_index].hand.len());
                self.players[self.current_index].hand_page = 0;
                let effect_card = self
                    .discard_pile
                    .pop()
                    .expect("played square-root card is on the discard pile");
                self.discard_pile.extend(discarded);
                self.discard_pile.push(effect_card);
                if after > self.players[self.current_index].hand.len() {
                    self.materialize_hand_page(&target, 0)
                        .expect("square-root target remains a player");
                }
                self.advance_turn(2);
                Some(HandEffect::SquareRoot {
                    target,
                    before,
                    after,
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
        if self.players[actor].virtual_len > 0 {
            let processed = processed_count.min(self.players[actor].hand_len());
            let distributed = distributed_count.min(processed);
            let actor_total = self.players[actor].hand_len().saturating_sub(processed);
            self.players[actor].hand.shuffle(&mut self.rng);
            self.players[actor]
                .hand
                .truncate(actor_total.min(HAND_BATCH_SIZE));
            self.players[actor].virtual_len =
                actor_total.saturating_sub(self.players[actor].hand.len());
            self.players[actor].hand_page = 0;

            let recipients = (1..self.players.len())
                .map(|offset| (actor + offset) % self.players.len())
                .collect::<Vec<_>>();
            let actor_id = self.players[actor].id.clone();
            let rule = self
                .player_draw_states
                .get(&actor_id)
                .map(|state| state.rule);
            let start = self.players[actor].materialized_received;
            let cards =
                generate_virtual_cards(self.deck_variant, rule, start, distributed, &mut self.rng);
            self.players[actor].materialized_received = start.saturating_add(distributed);
            for (index, card) in cards.into_iter().enumerate() {
                self.push_concrete_card(recipients[index % recipients.len()], card);
            }
            return;
        }
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
            self.push_concrete_card(recipients[index % recipients.len()], card);
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
            std::mem::swap(&mut left[first].virtual_len, &mut right[0].virtual_len);
            std::mem::swap(&mut left[first].hand_page, &mut right[0].hand_page);
        } else {
            let (left, right) = self.players.split_at_mut(first);
            std::mem::swap(&mut right[0].hand, &mut left[second].hand);
            std::mem::swap(&mut right[0].virtual_len, &mut left[second].virtual_len);
            std::mem::swap(&mut right[0].hand_page, &mut left[second].hand_page);
        }
    }

    fn rotate_hands(&mut self, direction: Direction) {
        let player_count = self.players.len();
        let hands = self
            .players
            .iter_mut()
            .map(|player| {
                (
                    std::mem::take(&mut player.hand),
                    std::mem::take(&mut player.virtual_len),
                    std::mem::take(&mut player.hand_page),
                )
            })
            .collect::<Vec<_>>();
        for (source, (hand, virtual_len, hand_page)) in hands.into_iter().enumerate() {
            let target = match direction {
                Direction::Clockwise => (source + 1) % player_count,
                Direction::CounterClockwise => (source + player_count - 1) % player_count,
            };
            self.players[target].hand = hand;
            self.players[target].virtual_len = virtual_len;
            self.players[target].hand_page = hand_page;
        }
    }

    fn is_playable_for(&self, hand: &[Card], total_len: usize, card: Card) -> bool {
        let minimum_hand = match card.rank {
            Rank::WildDiscardThirtyTwo => Some(66),
            Rank::WildDiscardSixtyFour => Some(132),
            _ => None,
        };
        if minimum_hand.is_some_and(|minimum| total_len < minimum) {
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
        let concrete_count =
            count.min(HAND_BATCH_SIZE.saturating_sub(self.players[index].hand.len()));
        let mut drawn = 0;
        for _ in 0..concrete_count {
            let Ok(card) = self.draw_card_for(player) else {
                break;
            };
            self.players[index].hand.push(card);
            drawn += 1;
        }
        let virtual_count = count.saturating_sub(drawn);
        self.players[index].virtual_len = self.players[index]
            .virtual_len
            .saturating_add(virtual_count);
        if let Some(state) = self.player_draw_states.get_mut(player) {
            state.received = state.received.saturating_add(virtual_count);
        }
        drawn = drawn.saturating_add(virtual_count);
        drawn
    }

    fn push_concrete_card(&mut self, player_index: usize, card: Card) {
        if self.players[player_index].hand.len() == HAND_BATCH_SIZE {
            self.players[player_index].hand.pop();
            self.players[player_index].virtual_len =
                self.players[player_index].virtual_len.saturating_add(1);
        }
        self.players[player_index].hand.push(card);
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

fn number_batch_colors(hand: &[Card], number: u8) -> Vec<Color> {
    hand.iter()
        .filter_map(|card| {
            if matches!(card.rank, Rank::Number(candidate) if candidate == number) {
                card.color
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_plus_batch_card(card: Card) -> bool {
    matches!(
        card.rank,
        Rank::DrawTwo | Rank::DrawEight | Rank::WildDrawSixteen
    )
}

fn indexed_plus_play((hand_index, card): (usize, Card)) -> IndexedPlusPlay {
    IndexedPlusPlay {
        hand_index,
        play: PlusPlay {
            card,
            chosen_color: None,
        },
    }
}

fn plus_penalty(card: Card) -> usize {
    match card.rank {
        Rank::DrawTwo => 2,
        Rank::DrawEight => 8,
        Rank::WildDrawSixteen => 16,
        _ => 0,
    }
}

fn plus_card_is_playable(active_color: Color, top: Card, card: Card) -> bool {
    card.rank == Rank::WildDrawSixteen
        || card.color == Some(active_color)
        || (!top.is_wild() && card.rank == top.rank)
}

fn best_plus_suffix(
    candidates: &[IndexedPlusPlay],
    used: u64,
    active_color: Color,
    top: Card,
    memo: &mut HashMap<(u64, Color, Card), Vec<IndexedPlusPlay>>,
) -> Vec<IndexedPlusPlay> {
    if let Some(cached) = memo.get(&(used, active_color, top)) {
        return cached.clone();
    }

    let mut best = Vec::new();
    let mut considered_cards = BTreeSet::new();
    for (candidate_index, candidate) in candidates.iter().copied().enumerate() {
        let bit = 1_u64 << candidate_index;
        if used & bit != 0
            || !considered_cards.insert(candidate.play.card)
            || !plus_card_is_playable(active_color, top, candidate.play.card)
        {
            continue;
        }
        let colors: &[Color] = if candidate.play.card.rank == Rank::WildDrawSixteen {
            &Color::ALL
        } else {
            std::slice::from_ref(
                candidate
                    .play
                    .card
                    .color
                    .as_ref()
                    .expect("colored plus card"),
            )
        };
        for chosen_color in colors {
            let mut entry = candidate;
            if entry.play.card.rank == Rank::WildDrawSixteen {
                entry.play.chosen_color = Some(*chosen_color);
            }
            let mut path = vec![entry];
            path.extend(best_plus_suffix(
                candidates,
                used | bit,
                *chosen_color,
                entry.play.card,
                memo,
            ));
            if plus_path_is_better(&path, &best) {
                best = path;
            }
        }
    }
    memo.insert((used, active_color, top), best.clone());
    best
}

fn plus_path_is_better(candidate: &[IndexedPlusPlay], current: &[IndexedPlusPlay]) -> bool {
    let candidate_penalty = candidate
        .iter()
        .map(|entry| plus_penalty(entry.play.card))
        .sum::<usize>();
    let current_penalty = current
        .iter()
        .map(|entry| plus_penalty(entry.play.card))
        .sum::<usize>();
    candidate.len() > current.len()
        || (candidate.len() == current.len() && candidate_penalty > current_penalty)
        || (candidate.len() == current.len()
            && candidate_penalty == current_penalty
            && candidate
                .iter()
                .map(|entry| entry.hand_index)
                .cmp(current.iter().map(|entry| entry.hand_index))
                .is_lt())
}

fn runtime_refill_seed() -> [u8; 32] {
    let mut seed = [0; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

const MAX_FACTORIAL_HAND_SIZE: usize = 2_100_000_000;

pub(crate) fn factorial_hand_size(cards: usize) -> usize {
    let power_cap = (0..15).try_fold(1_usize, |value, _| {
        value
            .checked_mul(cards)
            .filter(|result| *result < MAX_FACTORIAL_HAND_SIZE)
            .ok_or(())
    });
    let limit = power_cap.unwrap_or(MAX_FACTORIAL_HAND_SIZE);
    let mut factorial = 1_usize;
    for factor in 2..=cards {
        factorial = match factorial.checked_mul(factor) {
            Some(result) if result < limit => result,
            _ => return limit,
        };
    }
    factorial.min(limit)
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
            Rank::WildFactorial => Card::wild(Rank::WildFactorial),
            Rank::WildSquareRoot => Card::wild(Rank::WildSquareRoot),
            _ => unreachable!("only Holiday tier cards are guaranteed"),
        });
    }

    let index = deck
        .iter()
        .rposition(|card| card_allowed_for_rule(rule, card))
        .ok_or(GameError::EmptyDrawPile)?;
    Ok(deck.swap_remove(index))
}

fn generate_virtual_cards<R: Rng + ?Sized>(
    variant: DeckVariant,
    rule: Option<PlayerDrawRule>,
    received: usize,
    count: usize,
    rng: &mut R,
) -> Vec<Card> {
    let mut generated = Vec::with_capacity(count);
    let effective_rule = rule.filter(|_| variant == DeckVariant::Holiday);
    let all_types = deck(variant).into_iter().collect::<BTreeSet<_>>();
    let allowed_types = all_types
        .iter()
        .copied()
        .filter(|card| effective_rule.is_none_or(|rule| card_allowed_for_rule(rule, card)))
        .collect::<Vec<_>>();
    for offset in 0..count {
        let guaranteed =
            effective_rule.and_then(|rule| required_rank_for_rule(rule, received + offset));
        let card = match guaranteed {
            Some(Rank::DrawEight) => Card::new(
                Color::ALL[rng.gen_range(0..Color::ALL.len())],
                Rank::DrawEight,
            ),
            Some(rank) => Card::wild(rank),
            None => allowed_types[rng.gen_range(0..allowed_types.len())],
        };
        generated.push(card);
    }
    generated
}

fn required_rank_for_rule(rule: PlayerDrawRule, received: usize) -> Option<Rank> {
    let block_position = received % STARTING_HAND_SIZE;
    let card_number = received + 1;
    match rule {
        PlayerDrawRule::GuaranteeDrawEightPerSeven if block_position == 0 => Some(Rank::DrawEight),
        PlayerDrawRule::GuaranteeDrawEightPerSeven if block_position == 1 => {
            Some(Rank::WildSquareRoot)
        }
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position < 2 => {
            Some(Rank::DrawEight)
        }
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position < 4 => {
            Some(Rank::WildSquareRoot)
        }
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position == 4 => {
            Some(Rank::WildDrawSixteen)
        }
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven if block_position == 5 => {
            Some(Rank::WildFactorial)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
            if card_number.is_multiple_of(10) =>
        {
            Some(Rank::WildDrawSixteen)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen if card_number % 10 == 9 => {
            Some(Rank::WildFactorial)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
            if card_number.is_multiple_of(5) =>
        {
            Some(Rank::DrawEight)
        }
        PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen if card_number % 10 == 4 => {
            Some(Rank::WildSquareRoot)
        }
        PlayerDrawRule::GuaranteeDrawEightPerTwenty if card_number.is_multiple_of(20) => {
            Some(Rank::DrawEight)
        }
        PlayerDrawRule::GuaranteeDrawEightPerTwenty if card_number % 20 == 19 => {
            Some(Rank::WildSquareRoot)
        }
        _ => None,
    }
}

fn card_allowed_for_rule(rule: PlayerDrawRule, card: &Card) -> bool {
    let upper_or_discard_wild = matches!(
        card.rank,
        Rank::WildDrawSixteen
            | Rank::WildDiscardThirtyTwo
            | Rank::WildDiscardSixtyFour
            | Rank::WildFactorial
    );
    match rule {
        PlayerDrawRule::ExcludeDrawEightAndSixteen => {
            !matches!(card.rank, Rank::DrawEight | Rank::WildSquareRoot) && !upper_or_discard_wild
        }
        PlayerDrawRule::ExcludeDrawSixteen => !upper_or_discard_wild,
        PlayerDrawRule::GuaranteeDrawEightPerSeven
        | PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen
        | PlayerDrawRule::GuaranteeDrawEightPerTwenty => true,
        PlayerDrawRule::TwoDrawEightAndOneSixteenPerSeven => {
            !matches!(card.rank, Rank::DrawEight | Rank::WildSquareRoot) && !upper_or_discard_wild
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
    deck.reserve(18);
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
    deck.push(Card::wild(Rank::WildFactorial));
    deck.push(Card::wild(Rank::WildFactorial));
    deck.push(Card::wild(Rank::WildSquareRoot));
    deck.push(Card::wild(Rank::WildSquareRoot));
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
        assert_eq!(deck.len(), 130);
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
        assert_eq!(
            deck.iter()
                .filter(|card| **card == Card::wild(Rank::WildFactorial))
                .count(),
            2
        );
        assert_eq!(
            deck.iter()
                .filter(|card| **card == Card::wild(Rank::WildSquareRoot))
                .count(),
            2
        );
    }

    #[test]
    fn factorial_size_uses_power_and_absolute_caps_without_overflow() {
        assert_eq!(factorial_hand_size(0), 0);
        assert_eq!(factorial_hand_size(1), 1);
        assert_eq!(factorial_hand_size(5), 120);
        assert_eq!(factorial_hand_size(8), 40_320);
        assert_eq!(factorial_hand_size(9), 362_880);
        assert_eq!(factorial_hand_size(10), 3_628_800);
        assert_eq!(factorial_hand_size(12), 479_001_600);
        assert_eq!(factorial_hand_size(13), MAX_FACTORIAL_HAND_SIZE);
        assert_eq!(factorial_hand_size(usize::MAX), MAX_FACTORIAL_HAND_SIZE);
    }

    #[test]
    fn factorial_grows_and_skips_the_next_hand_in_both_directions() {
        for direction in [Direction::Clockwise, Direction::CounterClockwise] {
            let mut game = Game::new_seeded(players(3), DeckVariant::Holiday, 42).unwrap();
            let actor = game.players[0].id.clone();
            game.current_index = 0;
            game.direction = direction;
            game.players[0].hand = vec![
                Card::wild(Rank::WildFactorial),
                Card::new(Color::Blue, Rank::Number(1)),
            ];
            let target_index = if direction == Direction::Clockwise {
                1
            } else {
                2
            };
            game.players[target_index].hand = vec![
                Card::new(Color::Red, Rank::Number(1)),
                Card::new(Color::Red, Rank::Number(2)),
                Card::new(Color::Red, Rank::Number(3)),
                Card::new(Color::Red, Rank::Number(4)),
                Card::new(Color::Red, Rank::Number(5)),
            ];
            game.discard_pile = vec![Card::new(Color::Yellow, Rank::Number(7))];
            game.active_color = Color::Yellow;

            let event = game
                .apply_action(
                    &actor,
                    Action::Play {
                        card: Card::wild(Rank::WildFactorial),
                        chosen_color: Some(Color::Green),
                        swap_target: None,
                    },
                )
                .unwrap();

            assert_eq!(game.players[target_index].hand.len(), 120);
            assert_eq!(
                game.current_index,
                if direction == Direction::Clockwise {
                    2
                } else {
                    1
                }
            );
            assert!(matches!(
                event.kind,
                EventKind::CardPlayed {
                    hand_effect: Some(HandEffect::Factorial {
                        before: 5,
                        after: 120,
                        ..
                    }),
                    ..
                }
            ));
        }
    }

    #[test]
    fn factorial_virtualizes_the_two_point_one_billion_card_cap() {
        let mut game = Game::new_seeded(players(3), DeckVariant::Holiday, 43).unwrap();
        let actor = game.players[0].id.clone();
        game.current_index = 0;
        game.players[0].hand = vec![
            Card::wild(Rank::WildFactorial),
            Card::new(Color::Blue, Rank::Number(1)),
        ];
        game.players[1].hand = (0..13)
            .map(|number| Card::new(Color::Red, Rank::Number(number)))
            .collect();
        game.discard_pile = vec![Card::new(Color::Yellow, Rank::Number(7))];
        game.active_color = Color::Yellow;

        game.apply_action(
            &actor,
            Action::Play {
                card: Card::wild(Rank::WildFactorial),
                chosen_color: Some(Color::Green),
                swap_target: None,
            },
        )
        .unwrap();

        assert_eq!(game.players[1].hand_len(), MAX_FACTORIAL_HAND_SIZE);
        assert_eq!(game.players[1].hand.len(), HAND_BATCH_SIZE);
        assert_eq!(game.current_index, 2);
    }

    #[test]
    fn square_root_randomly_reduces_the_actor_and_keeps_the_wild_on_top() {
        let mut first = Game::new_seeded(players(3), DeckVariant::Holiday, 77).unwrap();
        let mut second = Game::new_seeded(players(3), DeckVariant::Holiday, 77).unwrap();
        for game in [&mut first, &mut second] {
            let actor = game.players[0].id.clone();
            game.current_index = 0;
            game.players[0].hand = std::iter::once(Card::wild(Rank::WildSquareRoot))
                .chain((0..10).map(|number| Card::new(Color::Red, Rank::Number(number))))
                .collect();
            game.discard_pile = vec![Card::new(Color::Yellow, Rank::Number(7))];
            game.active_color = Color::Yellow;
            let event = game
                .apply_action(
                    &actor,
                    Action::Play {
                        card: Card::wild(Rank::WildSquareRoot),
                        chosen_color: Some(Color::Blue),
                        swap_target: None,
                    },
                )
                .unwrap();
            assert_eq!(game.players[0].hand.len(), 3);
            assert_eq!(game.current_index, 2);
            assert_eq!(
                game.discard_pile.last(),
                Some(&Card::wild(Rank::WildSquareRoot))
            );
            assert!(matches!(
                event.kind,
                EventKind::CardPlayed {
                    hand_effect: Some(HandEffect::SquareRoot {
                        before: 10,
                        after: 3,
                        ..
                    }),
                    ..
                }
            ));
        }
        assert_eq!(first.players[0].hand, second.players[0].hand);
        assert_eq!(first.discard_pile, second.discard_pile);
    }

    #[test]
    fn final_mathematical_wild_wins_without_transforming_a_hand() {
        for rank in [Rank::WildFactorial, Rank::WildSquareRoot] {
            let mut game = Game::new_seeded(players(2), DeckVariant::Holiday, 88).unwrap();
            let actor = game.players[0].id.clone();
            game.current_index = 0;
            game.players[0].hand = vec![Card::wild(rank)];
            let target_before = game.players[1].hand.clone();
            game.discard_pile = vec![Card::new(Color::Red, Rank::Number(4))];
            game.active_color = Color::Red;

            let event = game
                .apply_action(
                    &actor,
                    Action::Play {
                        card: Card::wild(rank),
                        chosen_color: Some(Color::Blue),
                        swap_target: None,
                    },
                )
                .unwrap();

            assert!(matches!(
                event.kind,
                EventKind::CardPlayed {
                    hand_effect: None,
                    ..
                }
            ));
            assert_eq!(game.winner, Some(actor));
            assert_eq!(game.players[1].hand, target_before);
        }
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
    fn easy_ai_never_receives_either_guarantee_tier() {
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
                | Rank::WildFactorial
                | Rank::WildSquareRoot
        )));
        for _ in 0..30 {
            let card = game.draw_card_for(&bot).unwrap();
            assert!(!matches!(
                card.rank,
                Rank::DrawEight
                    | Rank::WildDrawSixteen
                    | Rank::WildDiscardThirtyTwo
                    | Rank::WildDiscardSixtyFour
                    | Rank::WildFactorial
                    | Rank::WildSquareRoot
            ));
        }
    }

    #[test]
    fn normal_ai_excludes_sixteen_factorial_and_discard_wilds_but_allows_square_root() {
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
            Rank::WildDrawSixteen
                | Rank::WildDiscardThirtyTwo
                | Rank::WildDiscardSixtyFour
                | Rank::WildFactorial
        )));
        game.draw_pile = vec![Card::wild(Rank::WildSquareRoot)];
        assert_eq!(game.draw_card_for(&bot).unwrap().rank, Rank::WildSquareRoot);
        for _ in 0..30 {
            assert!(!matches!(
                game.draw_card_for(&bot).unwrap().rank,
                Rank::WildDrawSixteen
                    | Rank::WildDiscardThirtyTwo
                    | Rank::WildDiscardSixtyFour
                    | Rank::WildFactorial
            ));
        }
    }

    #[test]
    fn hard_ai_receives_draw_eight_and_square_root_in_each_initial_hand() {
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
            assert!(
                game.hand_for(&PlayerId::new(format!("p{index}")))
                    .unwrap()
                    .iter()
                    .any(|card| card.rank == Rank::WildSquareRoot)
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
            assert_eq!(
                hand.iter()
                    .filter(|card| card.rank == Rank::WildSquareRoot)
                    .count(),
                2
            );
            assert_eq!(
                hand.iter()
                    .filter(|card| card.rank == Rank::WildFactorial)
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
        assert_eq!(
            next_seven
                .iter()
                .filter(|card| card.rank == Rank::WildSquareRoot)
                .count(),
            2
        );
        assert_eq!(
            next_seven
                .iter()
                .filter(|card| card.rank == Rank::WildFactorial)
                .count(),
            1
        );
    }

    #[test]
    fn easy_human_gets_both_members_of_each_guaranteed_tier() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(
            human.clone(),
            PlayerDrawRule::GuaranteeDrawEightPerFiveAndSixteenPerTen,
        )]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Holiday, rules, 15).unwrap();

        assert_eq!(game.hand_for(&human).unwrap()[3].rank, Rank::WildSquareRoot);
        assert_eq!(game.hand_for(&human).unwrap()[4].rank, Rank::DrawEight);
        let cards: Vec<Card> = (8..=20)
            .map(|_| game.draw_card_for(&human).unwrap())
            .collect();
        assert_eq!(cards[1].rank, Rank::WildFactorial);
        assert_eq!(cards[2].rank, Rank::WildDrawSixteen);
        assert_eq!(cards[6].rank, Rank::WildSquareRoot);
        assert_eq!(cards[7].rank, Rank::DrawEight);
        assert_eq!(cards[11].rank, Rank::WildFactorial);
        assert_eq!(cards[12].rank, Rank::WildDrawSixteen);
    }

    #[test]
    fn normal_human_gets_square_root_and_draw_eight_every_twenty_cards() {
        let human = PlayerId::new("p0");
        let rules = BTreeMap::from([(human.clone(), PlayerDrawRule::GuaranteeDrawEightPerTwenty)]);
        let mut game =
            Game::new_seeded_with_draw_rules(players(2), DeckVariant::Holiday, rules, 16).unwrap();

        let cards: Vec<Card> = (8..=40)
            .map(|_| game.draw_card_for(&human).unwrap())
            .collect();
        assert_eq!(cards[11].rank, Rank::WildSquareRoot);
        assert_eq!(cards[12].rank, Rank::DrawEight);
        assert_eq!(cards[31].rank, Rank::WildSquareRoot);
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
                    | Rank::WildFactorial
                    | Rank::WildSquareRoot
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
    fn plus_batch_planner_uses_wild_as_a_color_bridge_and_excludes_plus_four() {
        let mut game = game();
        let current = game.current_player().clone();
        let plus_four = Card::wild(Rank::WildDrawFour);
        game.set_test_turn(
            &current,
            vec![
                Card::new(Color::Blue, Rank::DrawTwo),
                Card::new(Color::Red, Rank::DrawEight),
                Card::wild(Rank::WildDrawSixteen),
                Card::new(Color::Green, Rank::DrawTwo),
                plus_four,
            ],
            Card::new(Color::Red, Rank::Number(5)),
        );

        let plays = game.best_plus_batch(&current).unwrap();

        assert_eq!(plays.len(), 4);
        assert!(!plays.iter().any(|play| play.card == plus_four));
        let wild_index = plays
            .iter()
            .position(|play| play.card.rank == Rank::WildDrawSixteen)
            .unwrap();
        assert!(wild_index + 1 < plays.len());
        assert_eq!(
            plays[wild_index].chosen_color,
            plays[wild_index + 1].card.color
        );
    }

    #[test]
    fn plus_batch_planner_handles_thirty_two_wild_draw_sixteens() {
        let mut game = game();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let wild = Card::wild(Rank::WildDrawSixteen);
        game.set_test_turn(
            &current,
            vec![wild; 32],
            Card::new(Color::Red, Rank::Number(5)),
        );
        let target_before = game.hand_len_for(&target).unwrap();

        let mut plays = game.best_plus_batch(&current).unwrap();

        assert_eq!(plays.len(), 32);
        assert!(plays[..31].iter().all(|play| play.chosen_color.is_some()));
        assert_eq!(plays[31].chosen_color, None);
        plays[31].chosen_color = Some(Color::Blue);

        let event = game.apply_plus_batch(&current, plays).unwrap();

        assert_eq!(game.hand_len_for(&target).unwrap(), target_before + 512);
        assert_eq!(game.hand_for(&target).unwrap().len(), HAND_BATCH_SIZE);
        assert!(matches!(
            event.kind,
            EventKind::PlusBatchPlayed {
                cards,
                penalty: 512,
                drawn: 512,
                final_color: Color::Blue,
                ..
            } if cards.len() == 32
        ));
    }

    #[test]
    fn plus_batch_after_drawing_can_only_plan_one_matching_card_value() {
        let mut game = game();
        let current = game.current_player().clone();
        let drawn = Card::new(Color::Red, Rank::DrawTwo);
        game.set_test_turn(
            &current,
            vec![drawn, drawn, Card::new(Color::Blue, Rank::DrawTwo)],
            Card::new(Color::Red, Rank::Number(5)),
        );
        game.phase = TurnPhase::Drew(drawn);

        let plays = game.best_plus_batch(&current).unwrap();

        assert_eq!(plays.len(), 1);
        assert_eq!(plays[0].card, drawn);
    }

    #[test]
    fn plus_batch_draws_the_sum_skips_once_and_still_penalizes_on_a_win() {
        let mut game = Game::new_seeded(players(3), DeckVariant::Standard, 70).unwrap();
        let current = game.current_player().clone();
        let target = game.players[1].id.clone();
        let after_target = game.players[2].id.clone();
        let before = game.hand_for(&target).unwrap().len();
        let red = Card::new(Color::Red, Rank::DrawTwo);
        let blue = Card::new(Color::Blue, Rank::DrawTwo);
        game.set_test_turn(
            &current,
            vec![red, blue],
            Card::new(Color::Red, Rank::Number(5)),
        );

        let event = game
            .apply_plus_batch(
                &current,
                vec![
                    PlusPlay {
                        card: red,
                        chosen_color: None,
                    },
                    PlusPlay {
                        card: blue,
                        chosen_color: None,
                    },
                ],
            )
            .unwrap();

        assert_eq!(game.hand_for(&target).unwrap().len(), before + 4);
        assert_eq!(game.current_player(), &after_target);
        assert_eq!(game.public_state().winner, Some(current.clone()));
        assert!(matches!(
            event.kind,
            EventKind::PlusBatchPlayed {
                player,
                target: event_target,
                penalty: 4,
                drawn: 4,
                final_color: Color::Blue,
                ..
            } if player == current && event_target == target
        ));
    }

    #[test]
    fn invalid_plus_batch_is_atomic_and_final_wild_sets_the_selected_color() {
        let mut game = game();
        let current = game.current_player().clone();
        let red = Card::new(Color::Red, Rank::DrawTwo);
        let blue = Card::new(Color::Blue, Rank::DrawEight);
        let wild = Card::wild(Rank::WildDrawSixteen);
        let remaining = Card::new(Color::Yellow, Rank::Number(1));
        game.set_test_turn(
            &current,
            vec![red, blue, wild, remaining],
            Card::new(Color::Red, Rank::Number(5)),
        );
        let before_hand = game.hand_for(&current).unwrap().to_vec();
        let before_state = game.public_state();

        assert_eq!(
            game.apply_plus_batch(
                &current,
                vec![PlusPlay {
                    card: blue,
                    chosen_color: None,
                }],
            )
            .unwrap_err(),
            GameError::InvalidPlusBatch
        );
        assert_eq!(game.hand_for(&current).unwrap(), before_hand);
        assert_eq!(game.public_state(), before_state);

        game.apply_plus_batch(
            &current,
            vec![
                PlusPlay {
                    card: red,
                    chosen_color: None,
                },
                PlusPlay {
                    card: wild,
                    chosen_color: Some(Color::Green),
                },
            ],
        )
        .unwrap();
        assert_eq!(game.public_state().active_color, Color::Green);
        assert_eq!(game.hand_for(&current).unwrap(), &[blue, remaining]);
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
    fn number_batch_requires_a_present_color_and_puts_it_on_top() {
        let mut game = game();
        let current = game.current_player().clone();
        let selected = Card::new(Color::Blue, Rank::Number(3));
        let other_number = Card::new(Color::Red, Rank::Number(3));
        let remaining = Card::new(Color::Green, Rank::Number(8));
        game.active_color = Color::Blue;
        game.discard_pile = vec![Card::new(Color::Blue, Rank::Number(6))];
        game.players[0].hand = vec![other_number, remaining, selected];

        let before_state = game.public_state();
        let before_hand = game.players[0].hand.clone();
        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: selected,
                    chosen_color: None,
                    swap_target: None,
                },
            )
            .unwrap_err(),
            GameError::MissingColorChoice
        );
        assert_eq!(game.public_state(), before_state);
        assert_eq!(game.players[0].hand, before_hand);
        assert_eq!(
            game.apply_action(
                &current,
                Action::Play {
                    card: selected,
                    chosen_color: Some(Color::Yellow),
                    swap_target: None,
                },
            )
            .unwrap_err(),
            GameError::InvalidNumberBatchColor(Color::Yellow)
        );
        assert_eq!(game.public_state(), before_state);
        assert_eq!(game.players[0].hand, before_hand);

        let event = game
            .apply_action(
                &current,
                Action::Play {
                    card: selected,
                    chosen_color: Some(Color::Red),
                    swap_target: None,
                },
            )
            .unwrap();

        assert_eq!(game.players[0].hand, vec![remaining]);
        assert_eq!(
            game.discard_pile,
            vec![
                Card::new(Color::Blue, Rank::Number(6)),
                selected,
                other_number,
            ]
        );
        assert_eq!(game.active_color, Color::Red);
        assert!(matches!(
            event.kind,
            EventKind::CardPlayed {
                card: event_card,
                chosen_color: Some(Color::Red),
                ..
            } if event_card == other_number
        ));
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
                chosen_color: Some(Color::Red),
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
                    chosen_color: Some(Color::Blue),
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
                    | Rank::WildFactorial
                    | Rank::WildSquareRoot
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

    #[test]
    fn legal_actions_are_bounded_for_a_million_duplicate_cards() {
        let mut game = game();
        let current = game.current_player().clone();
        let repeated = Card::new(Color::Red, Rank::Number(5));
        game.set_test_turn(
            &current,
            std::iter::repeat_n(repeated, MAX_FACTORIAL_HAND_SIZE).collect(),
            Card::new(Color::Red, Rank::Number(1)),
        );

        let legal = game.legal_actions(&current).unwrap();

        assert_eq!(legal.len(), 2);
        assert!(legal.iter().any(|action| matches!(
            action,
            Action::Play { card, .. } if *card == repeated
        )));
        assert!(legal.contains(&Action::Draw));
    }

    #[test]
    fn virtual_pages_keep_total_count_and_bound_concrete_cards() {
        let mut game = Game::new_seeded(players(2), DeckVariant::Standard, 90).unwrap();
        let current = game.current_player().clone();
        game.set_test_turn(
            &current,
            vec![Card::new(Color::Red, Rank::Number(5)); 401],
            Card::new(Color::Red, Rank::Number(1)),
        );

        assert_eq!(game.hand_len_for(&current), Ok(401));
        assert_eq!(game.hand_page_for(&current), Ok((0, 3, 200)));

        game.materialize_hand_page(&current, 2).unwrap();
        assert_eq!(game.hand_len_for(&current), Ok(401));
        assert_eq!(game.hand_page_for(&current), Ok((2, 3, 1)));

        game.materialize_hand_page(&current, 0).unwrap();
        assert_eq!(game.hand_page_for(&current), Ok((0, 3, 200)));
    }

    #[test]
    fn virtual_generation_uses_rules_without_consuming_the_shared_pile() {
        let bot = PlayerId::new("p0");
        let mut easy = Game::new_seeded_with_draw_rules(
            players(2),
            DeckVariant::Holiday,
            BTreeMap::from([(bot.clone(), PlayerDrawRule::ExcludeDrawEightAndSixteen)]),
            91,
        )
        .unwrap();
        easy.set_test_turn(
            &bot,
            vec![Card::new(Color::Red, Rank::Number(5)); 400],
            Card::new(Color::Red, Rank::Number(1)),
        );
        let pile_len = easy.draw_pile.len();
        easy.materialize_hand_page(&bot, 1).unwrap();
        assert_eq!(easy.draw_pile.len(), pile_len);
        assert!(easy.hand_for(&bot).unwrap().iter().all(|card| !matches!(
            card.rank,
            Rank::DrawEight
                | Rank::WildSquareRoot
                | Rank::WildDrawSixteen
                | Rank::WildFactorial
                | Rank::WildDiscardThirtyTwo
                | Rank::WildDiscardSixtyFour
        )));

        let mut hard = Game::new_seeded_with_draw_rules(
            players(2),
            DeckVariant::Holiday,
            BTreeMap::from([(bot.clone(), PlayerDrawRule::GuaranteeDrawEightPerSeven)]),
            92,
        )
        .unwrap();
        hard.set_test_turn(
            &bot,
            vec![Card::new(Color::Red, Rank::Number(5)); 400],
            Card::new(Color::Red, Rank::Number(1)),
        );
        hard.materialize_hand_page(&bot, 1).unwrap();
        assert_eq!(hard.hand_for(&bot).unwrap()[0].rank, Rank::DrawEight);
        assert_eq!(hard.hand_for(&bot).unwrap()[1].rank, Rank::WildSquareRoot);
    }

    #[test]
    fn a_drawn_card_stays_concrete_when_the_active_batch_is_full() {
        let mut game = Game::new_seeded(players(2), DeckVariant::Standard, 93).unwrap();
        let current = game.current_player().clone();
        let drawn = Card::new(Color::Blue, Rank::Number(9));
        game.set_test_turn(
            &current,
            vec![Card::new(Color::Red, Rank::Number(5)); 201],
            Card::new(Color::Red, Rank::Number(1)),
        );
        game.draw_pile.push(drawn);

        game.apply_action(&current, Action::Draw).unwrap();

        assert_eq!(game.hand_len_for(&current), Ok(202));
        assert_eq!(game.hand_for(&current).unwrap().len(), HAND_BATCH_SIZE);
        assert!(game.hand_for(&current).unwrap().contains(&drawn));
        assert_eq!(game.phase, TurnPhase::Drew(drawn));
    }

    #[test]
    fn an_exhausted_active_batch_materializes_the_remaining_virtual_cards() {
        let mut game = Game::new_seeded(players(2), DeckVariant::Standard, 96).unwrap();
        let current = game.current_player().clone();
        game.set_test_turn(
            &current,
            vec![Card::new(Color::Red, Rank::Number(5)); 201],
            Card::new(Color::Red, Rank::Number(1)),
        );
        game.players[0].hand.clear();

        assert_eq!(game.hand_len_for(&current), Ok(1));
        assert_eq!(game.materialize_next_batch_if_empty(&current), Ok(true));
        assert_eq!(game.hand_len_for(&current), Ok(1));
        assert_eq!(game.hand_for(&current).unwrap().len(), 1);
    }

    #[test]
    fn virtual_square_root_and_discard_wilds_use_total_counts() {
        let mut square = Game::new_seeded(players(3), DeckVariant::Holiday, 94).unwrap();
        let actor = square.current_player().clone();
        let mut hand = vec![Card::wild(Rank::WildSquareRoot)];
        hand.extend(vec![Card::new(Color::Red, Rank::Number(5)); 999]);
        square.set_test_turn(&actor, hand, Card::new(Color::Red, Rank::Number(1)));
        square
            .apply_action(
                &actor,
                Action::Play {
                    card: Card::wild(Rank::WildSquareRoot),
                    chosen_color: Some(Color::Blue),
                    swap_target: None,
                },
            )
            .unwrap();
        assert_eq!(square.hand_len_for(&actor), Ok(31));
        assert_eq!(square.hand_for(&actor).unwrap().len(), 31);

        let mut discard = Game::new_seeded(players(3), DeckVariant::Holiday, 95).unwrap();
        let actor = discard.current_player().clone();
        let recipients = [discard.players[1].id.clone(), discard.players[2].id.clone()];
        let before = recipients
            .iter()
            .map(|id| discard.hand_len_for(id).unwrap())
            .collect::<Vec<_>>();
        let mut hand = vec![Card::wild(Rank::WildDiscardThirtyTwo)];
        hand.extend(vec![Card::new(Color::Red, Rank::Number(5)); 300]);
        discard.set_test_turn(&actor, hand, Card::new(Color::Red, Rank::Number(1)));
        discard
            .apply_action(
                &actor,
                Action::Play {
                    card: Card::wild(Rank::WildDiscardThirtyTwo),
                    chosen_color: Some(Color::Blue),
                    swap_target: None,
                },
            )
            .unwrap();
        assert_eq!(discard.hand_len_for(&actor), Ok(256));
        for (id, previous) in recipients.iter().zip(before) {
            assert_eq!(discard.hand_len_for(id), Ok(previous + 6));
        }
        assert!(
            discard
                .players
                .iter()
                .all(|player| player.hand.len() <= HAND_BATCH_SIZE)
        );
    }
}
