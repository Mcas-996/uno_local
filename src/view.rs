//! Semantic view data and layout helpers shared by both frontends.

use unicode_width::UnicodeWidthStr;

use crate::app::{App, Screen};
use crate::core::Card;
use crate::i18n::Language;

pub const MIN_COLUMNS: u16 = 70;
pub const MIN_ROWS: u16 = 26;

pub struct AppView<'a> {
    _app: &'a App,
    pub screen: Screen,
    pub images_allowed: bool,
}

impl<'a> AppView<'a> {
    pub fn new(app: &'a App, images_allowed: bool) -> Self {
        let overlays = app.pending_wild.is_some()
            || matches!(
                app.screen,
                Screen::Help | Screen::Result | Screen::QuitConfirm
            );
        Self {
            _app: app,
            screen: app.screen,
            images_allowed: images_allowed && app.screen == Screen::Game && !overlays,
        }
    }
}

pub fn card_entry(language: Language, index: usize, card: Card) -> String {
    format!(" {}:[{}]  ", index + 1, language.card(card))
}

pub fn wrap_hand(language: Language, hand: &[Card], width: usize) -> Vec<Vec<(usize, String)>> {
    let mut rows = Vec::<Vec<(usize, String)>>::new();
    let mut row = Vec::new();
    let mut used = 0;
    for (index, card) in hand.iter().copied().enumerate() {
        let entry = card_entry(language, index, card);
        let entry_width = UnicodeWidthStr::width(entry.as_str());
        if !row.is_empty() && used + entry_width > width {
            rows.push(row);
            row = Vec::new();
            used = 0;
        }
        used += entry_width;
        row.push((index, entry));
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

pub fn adjacent_hand_card(
    language: Language,
    hand: &[Card],
    selected_card: usize,
    width: usize,
    row_delta: isize,
) -> usize {
    let rows = wrap_hand(language, hand, width.max(1));
    let Some((row_index, item_index)) = rows.iter().enumerate().find_map(|(row_index, row)| {
        row.iter()
            .position(|(index, _)| *index == selected_card)
            .map(|item_index| (row_index, item_index))
    }) else {
        return selected_card;
    };
    let Some(target_row) = row_index.checked_add_signed(row_delta) else {
        return selected_card;
    };
    rows.get(target_row)
        .and_then(|row| row.get(item_index.min(row.len().saturating_sub(1))))
        .map_or(selected_card, |(index, _)| *index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Color, Rank};

    #[test]
    fn hand_wrapping_and_vertical_navigation_share_geometry() {
        let hand = vec![
            Card::new(Color::Red, Rank::Number(1)),
            Card::new(Color::Blue, Rank::Number(2)),
            Card::new(Color::Green, Rank::Number(3)),
        ];
        let rows = wrap_hand(Language::English, &hand, 16);
        assert!(rows.len() > 1);
        assert_eq!(adjacent_hand_card(Language::English, &hand, 1, 16, -1), 0);
    }
}
