use super::*;
use serde::{Deserialize, Serialize};

/// Contains public board information that all players have
/// access to such as card counts, nobles available, and gems available in
/// the piles. Removes any hidden information (such as the order that cards
/// will be drawn from the deck).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub deck_counts: [usize; 3],
    pub available_cards: Vec<Vec<CardId>>,
    pub nobles: Vec<NobleId>,
    pub gems: Gems,
}

impl Board {
    pub fn from_game(game: &Game) -> Self {
        Board {
            deck_counts: game.deck_counts(),
            available_cards: game.cards(),
            nobles: game.nobles().iter().map(|n| n.id()).collect(),
            gems: game.bank().clone(),
        }
    }
}
