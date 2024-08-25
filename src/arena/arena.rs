use crate::card::Card;
use crate::game_logic::*;
use crate::player::*;
use crate::JSONable;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::arena::protocol::*;
use crate::arena::replay::*;
use crate::arena::clock::*;

/// A module for running games across multiple clients. Can be fed binaries
/// and run them in a tournament style. The protocol for communication is
/// given by JSON messages across local websockets that update the game state.
pub struct Arena {
    game: Game,
    pub clients: Vec<String>,
    clock: Clock,
    game_started: bool,
    replay: Either<Replay<Initialized>, FinalizedReplay>,
}

/// A struct given to each client that contains all public information and private
/// information known only to that client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub board: Board,
    pub history: GameHistory,
    pub players: Vec<PlayerPublicInfo>,
    pub current_player: Player,
    pub current_player_num: usize,
    pub legal_actions: Vec<Action>,
    pub time_endpoint_url: String,
}

impl JSONable for ClientInfo {}

impl Arena {
    pub fn new(players: u8, binaries: Vec<String>, initial_time: Duration, increment: Duration) -> Arena {
        let card_lookup = Arc::new(Card::all());
        let game = Game::new(players, card_lookup);
        let clients = binaries;
        let num_players = players as usize;

        Arena {
            game: game.clone(),
            replay: Either::Initialized(Replay::new(game)),
            clients,
            game_started: false,
            clock: Clock::new(num_players, initial_time, increment),
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.game.game_over()
    }

    pub fn client_info(&self) -> ClientInfo {
        let players = self.game.players().iter().map(|p| p.to_public()).collect();
        let legal_actions = self
            .game
            .get_legal_actions()
            .expect("Cannot get legal actions");

        ClientInfo {
            board: Board::from_game(&self.game),
            history: self.game.history(),
            players,
            current_player: self.game.current_player(),
            current_player_num: self.game.current_player_num(),
            legal_actions,
            time_endpoint_url: "http://127.0.0.1:3030/time".to_string(), // TODO: not hardcoded
        }
    }

    pub fn finalize_game(&mut self) {
        let replay = self.replay.clone();
        match replay {
            Either::Initialized(replay) => {
                let history = self.game.history();
                let replay = replay.finalize_with(history);
                let replay = Arc::new(RwLock::new(replay));
                self.replay = Either::Finalized(replay);
            }
            _ => panic!("Cannot finalize game that is already finalized"),
        }
    }

    pub fn get_replay(&self) -> Option<FinalizedReplay> {
        match &self.replay {
            Either::Finalized(replay) => Some(replay.clone()),
            _ => None,
        }
    }

    /// Play an action in the game. If the action is to continue, the clock will
    /// be updated to the next player
    pub fn play_action(&mut self, action : Action) {
        self.game.play_action(action.clone());
        match action {
            Action::Continue => {
                self.clock.end();
                self.clock.next_player();
                self.clock.start();
            }
            _ => {}
        }
    }

    pub fn get_legal_actions(&self) -> Option<Vec<Action>> {
        self.game.get_legal_actions()
    }

    pub fn current_player_num(&self) -> Option<usize> {
        if self.game_started {
            Some(self.game.current_player_num())
        } else {
            None
        }
    }

    pub fn get_winner(&self) -> Option<usize> {
        self.game.get_winner()
    }

    pub fn board(&self) -> Board {
        Board::from_game(&self.game)
    }

    pub fn players(&self) -> &Vec<Player> {
        self.game.players()
    }

    pub fn is_timed_out(&self) -> bool {
        self.clock.time_remaining() <= Duration::from_secs(0)
    }

    pub fn time_remaining(&self) -> Duration {
        self.clock.time_remaining()
    }

    pub fn start_game(&mut self) {
        self.game_started = true;
        self.clock.start();
    }
}

pub struct GameResults {}
