use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use crate::core::{Action, Card, Color, Direction, PublicGameState, Rank};
use rand::Rng;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        })
    }
}

impl FromStr for Difficulty {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "easy" => Ok(Self::Easy),
            "normal" | "medium" => Ok(Self::Normal),
            "hard" => Ok(Self::Hard),
            _ => Err(format!("invalid difficulty '{value}'")),
        }
    }
}

pub fn choose_action<R: Rng + ?Sized>(
    difficulty: Difficulty,
    state: &PublicGameState,
    hand: &[Card],
    legal_actions: &[Action],
    rng: &mut R,
) -> Action {
    let playable_cards: Vec<Card> = legal_actions
        .iter()
        .filter_map(|action| match action {
            Action::Play { card, .. } => Some(*card),
            Action::Draw | Action::Pass => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if playable_cards.is_empty() {
        return legal_actions
            .iter()
            .find(|action| matches!(action, Action::Draw | Action::Pass))
            .cloned()
            .expect("a live turn always has draw or pass");
    }

    let selected = match difficulty {
        Difficulty::Easy => playable_cards[rng.gen_range(0..playable_cards.len())],
        Difficulty::Normal | Difficulty::Hard => {
            choose_scored(difficulty, state, hand, &playable_cards, rng)
        }
    };
    let chosen_color = if selected.is_wild() {
        Some(match difficulty {
            Difficulty::Easy => Color::ALL[rng.gen_range(0..Color::ALL.len())],
            Difficulty::Normal | Difficulty::Hard => dominant_color(hand, rng),
        })
    } else {
        None
    };
    Action::Play {
        card: selected,
        chosen_color,
    }
}

fn choose_scored<R: Rng + ?Sized>(
    difficulty: Difficulty,
    state: &PublicGameState,
    hand: &[Card],
    playable: &[Card],
    rng: &mut R,
) -> Card {
    let has_colored_play = playable.iter().any(|card| !card.is_wild());
    let next_hand = next_opponent_hand(state).unwrap_or(usize::MAX);
    let mut best_score = i32::MIN;
    let mut best = Vec::new();
    for card in playable {
        let remaining_same_color = card.color.map_or(0, |color| {
            hand.iter()
                .filter(|candidate| candidate.color == Some(color) && **candidate != *card)
                .count() as i32
        });
        let mut score = remaining_same_color * 3
            + match card.rank {
                Rank::Number(number) => i32::from(number) / 3,
                Rank::Skip | Rank::Reverse => 4,
                Rank::DrawTwo => 6,
                Rank::Wild => 1,
                Rank::WildDrawFour => 7,
            };
        if card.is_wild() && has_colored_play {
            score -= 8;
        }
        if difficulty == Difficulty::Hard && next_hand <= 2 {
            score += match card.rank {
                Rank::WildDrawFour => 14,
                Rank::DrawTwo => 12,
                Rank::Skip | Rank::Reverse => 10,
                Rank::Number(_) | Rank::Wild => 0,
            };
        }
        match score.cmp(&best_score) {
            std::cmp::Ordering::Greater => {
                best_score = score;
                best.clear();
                best.push(*card);
            }
            std::cmp::Ordering::Equal => best.push(*card),
            std::cmp::Ordering::Less => {}
        }
    }
    best[rng.gen_range(0..best.len())]
}

fn dominant_color<R: Rng + ?Sized>(hand: &[Card], rng: &mut R) -> Color {
    let mut counts = BTreeMap::new();
    for card in hand {
        if let Some(color) = card.color {
            *counts.entry(color).or_insert(0_usize) += 1;
        }
    }
    let max = counts.values().copied().max().unwrap_or(0);
    let choices: Vec<Color> = Color::ALL
        .into_iter()
        .filter(|color| counts.get(color).copied().unwrap_or(0) == max)
        .collect();
    choices[rng.gen_range(0..choices.len())]
}

fn next_opponent_hand(state: &PublicGameState) -> Option<usize> {
    let current = state
        .players
        .iter()
        .position(|player| player.id == state.current_player)?;
    let next = match state.direction {
        Direction::Clockwise => (current + 1) % state.players.len(),
        Direction::CounterClockwise => (current + state.players.len() - 1) % state.players.len(),
    };
    Some(state.players[next].hand_len)
}

#[cfg(test)]
mod tests {
    use crate::core::{PlayerId, PublicPlayerState};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    fn state(next_hand: usize) -> PublicGameState {
        PublicGameState {
            players: vec![
                PublicPlayerState {
                    id: PlayerId::new("bot"),
                    name: "Bot".to_owned(),
                    hand_len: 3,
                },
                PublicPlayerState {
                    id: PlayerId::new("human"),
                    name: "Human".to_owned(),
                    hand_len: next_hand,
                },
            ],
            discard_top: Card::new(Color::Red, Rank::Number(3)),
            active_color: Color::Red,
            current_player: PlayerId::new("bot"),
            direction: Direction::Clockwise,
            has_drawn: false,
            winner: None,
            next_sequence: 1,
        }
    }

    fn play(card: Card) -> Action {
        Action::Play {
            card,
            chosen_color: card.is_wild().then_some(Color::Red),
        }
    }

    #[test]
    fn no_play_chooses_draw_or_pass() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            choose_action(Difficulty::Easy, &state(7), &[], &[Action::Draw], &mut rng),
            Action::Draw
        );
        assert_eq!(
            choose_action(Difficulty::Hard, &state(7), &[], &[Action::Pass], &mut rng),
            Action::Pass
        );
    }

    #[test]
    fn seeded_easy_choice_is_reproducible_and_legal() {
        let cards = [
            Card::new(Color::Red, Rank::Number(1)),
            Card::new(Color::Red, Rank::Skip),
        ];
        let legal = [play(cards[0]), play(cards[1]), Action::Draw];
        let mut first = StdRng::seed_from_u64(9);
        let mut second = StdRng::seed_from_u64(9);
        let a = choose_action(Difficulty::Easy, &state(7), &cards, &legal, &mut first);
        let b = choose_action(Difficulty::Easy, &state(7), &cards, &legal, &mut second);
        assert_eq!(a, b);
        assert!(legal.contains(&a));
    }

    #[test]
    fn normal_preserves_wild_when_colored_play_exists() {
        let colored = Card::new(Color::Red, Rank::DrawTwo);
        let wild = Card::wild(Rank::Wild);
        let hand = [colored, Card::new(Color::Red, Rank::Number(8)), wild];
        let legal = [
            play(colored),
            play(wild),
            Action::Play {
                card: wild,
                chosen_color: Some(Color::Blue),
            },
            Action::Draw,
        ];
        let mut rng = StdRng::seed_from_u64(2);
        assert_eq!(
            choose_action(Difficulty::Normal, &state(5), &hand, &legal, &mut rng),
            play(colored)
        );
    }

    #[test]
    fn hard_prioritizes_disruption_when_opponent_is_close() {
        let number = Card::new(Color::Red, Rank::Number(9));
        let skip = Card::new(Color::Red, Rank::Skip);
        let hand = [number, skip, Card::new(Color::Blue, Rank::Number(1))];
        let legal = [play(number), play(skip), Action::Draw];
        let mut rng = StdRng::seed_from_u64(3);
        assert_eq!(
            choose_action(Difficulty::Hard, &state(2), &hand, &legal, &mut rng),
            play(skip)
        );
    }

    #[test]
    fn normal_wild_chooses_dominant_color() {
        let wild = Card::wild(Rank::Wild);
        let hand = [
            wild,
            Card::new(Color::Blue, Rank::Number(1)),
            Card::new(Color::Blue, Rank::Number(4)),
            Card::new(Color::Red, Rank::Number(7)),
        ];
        let legal = Color::ALL.map(|color| Action::Play {
            card: wild,
            chosen_color: Some(color),
        });
        let mut rng = StdRng::seed_from_u64(4);
        assert_eq!(
            choose_action(Difficulty::Normal, &state(6), &hand, &legal, &mut rng),
            Action::Play {
                card: wild,
                chosen_color: Some(Color::Blue)
            }
        );
    }
}
