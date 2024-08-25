use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameHistory {
    pub history: Vec<(usize, Action)>,
}

type PlayerActions = Vec<(usize, Action)>;

impl GameHistory {
    pub fn new() -> Self {
        GameHistory {
            history: Vec::new(),
        }
    }

    pub fn from(history: Vec<(usize, Action)>) -> Self {
        GameHistory { history }
    }

    fn undo(&mut self) {
        self.history.pop();
    }

    fn history_since_player(&self, player_num: usize) -> GameHistory {
        let mut new_history = Vec::new();
        for (p, a) in self.history.iter().rev() {
            if *p == player_num {
                new_history.push((*p, a.clone()));
            } else {
                break;
            }
        }
        new_history.reverse();
        GameHistory::from(new_history)
    }

    pub fn add(&mut self, player_num: usize, action: Action) {
        self.history.push((player_num, action));
    }

    // A move is defined a bit weirdly here,
    // it's all the actions taken by a single player in a turn
    pub fn num_moves(&self) -> i32 {
        let mut moves = 0;
        // Group the actions by player and count every transition
        self.history.iter().fold(None, |acc, (p, _)| {
            if let Some(last_p) = acc {
                if last_p != *p {
                    moves += 1;
                }
            }
            Some(*p)
        });
        moves
    }

    /// Group all items in history by each player such that
    /// all actions taken by a single player (from PlayerStart through
    /// to Continue) are grouped together
    pub fn group_by_player(&self) -> Vec<PlayerActions> {
        let mut turn_sequences = vec![];
        let mut current_turn = vec![];
        let mut last_player = None;
        for (player_num, action) in self.history.iter() {
            if last_player != Some(*player_num) {
                if !current_turn.is_empty() {
                    turn_sequences.push(current_turn);
                }
                current_turn = vec![];
            }
            current_turn.push((*player_num, action.clone()));
            last_player = Some(*player_num);
        }

        if !current_turn.is_empty() {
            turn_sequences.push(current_turn);
        }

        turn_sequences
    }

    // Inclusively traverse the history until the given move index
    // and return a new history with only the actions taken until that point
    pub fn take_until_move(&self, move_index_target: i32) -> GameHistory {
        let move_index_target = (move_index_target + 1) as usize;
        let actions = self
            .group_by_player()
            .into_iter()
            .take(move_index_target)
            .flatten()
            .collect();
        GameHistory::from(actions)
    }
}

impl IntoIterator for GameHistory {
    type Item = (usize, Action);
    type IntoIter = std::vec::IntoIter<(usize, Action)>;

    fn into_iter(self) -> Self::IntoIter {
        self.history.into_iter()
    }
}
