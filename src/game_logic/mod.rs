use crate::card::CardId;
use crate::gem::Gem;
use crate::nobles::*;
use crate::player::Player;
use crate::gems::Gems;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod board;
pub mod game;
pub mod history;

pub use self::board::*;
pub use self::game::*;
pub use self::history::*;

#[derive(Debug, Clone)]
enum Phase {
    PlayerStart,            // Take some player action
    PlayerGemCapExceeded,   // [Optional] Player has > 10 gems
    NobleAction,            // See if any nobles get attracted (multiple may be attracted)
    PlayerActionEnd,        // Finish the turn and see if the round should continue
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    TakeDouble(Gem),
    TakeDistinct(HashSet<Gem>),
    Reserve(CardId),
    ReserveHidden(usize),
    Purchase((CardId, Gems)),

    Discard(Gems),

    AttractNoble(NobleId),

    /// Marker for the rare case when a player is unable to take
    /// an action, but the game isn't yet over
    Pass,

    /// Marker for passing the turn to the next player
    /// Unavailable if the game is over
    Continue,
}

pub fn choose_distinct_gems(
    gems: &mut Gems,
    running: &mut Gems,
    num_chosen: u32,
) -> HashSet<Gems> {
    let mut total_choices = HashSet::new();
    if num_chosen == 0 {
        total_choices.insert(running.clone());
        return total_choices;
    }
    // Pick one to discard and recurse
    for color in Gem::all_expect_gold() {
        if gems[color] > 0 {
            if running[color] > 0 {
                continue;
            }

            gems[color] -= 1;
            running[color] += 1;

            let choices = choose_distinct_gems(gems, running, num_chosen - 1);
            total_choices.extend(choices);

            running[color] -= 1;
            gems[color] += 1;
        }
    }

    total_choices
}

pub fn choose_gems(gems: &mut Gems, running: &mut Gems, num_chosen: u32) -> HashSet<Gems> {
    let mut total_choices = HashSet::new();
    if num_chosen == 0 {
        total_choices.insert(running.clone());
        return total_choices;
    }
    // Pick one to discard and recurse
    for color in Gem::all() {
        if gems[color] > 0 {
            gems[color] -= 1;
            running[color] += 1;

            let choices = choose_gems(gems, running, num_chosen - 1);
            total_choices.extend(choices);

            running[color] -= 1;
            gems[color] += 1;
        }
    }

    total_choices
}
