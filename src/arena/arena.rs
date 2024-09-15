use crate::card::Card;
use crate::game_logic::*;
use crate::player::*;
use crate::JSONable;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use super::*;

use crate::arena::protocol::*;
use crate::arena::replay::*;
use crate::arena::clock::*;

use log::{debug, error, info, trace};
use warp::Filter;

/// Builder for creating an arena,
/// allows clients to flexibly include sane defaults or override them 
/// with given parameters
pub struct ArenaBuilder {
    binaries: Vec<String>,
    python_interpreter : String,
    initial_time: Duration,
    increment: Duration,
    static_files: String,
    port : u16,
    send_to_web: bool,
    api_key: Option<String>,
}


impl ArenaBuilder {
    pub fn new() -> Self  {
        ArenaBuilder {
            binaries: Vec::new(),
            python_interpreter : "python3".to_string(),
            initial_time: Duration::from_secs(60),
            increment: Duration::from_secs(0),
            port : 3030,
            send_to_web: false,
            static_files: "splendor".to_string(),
            api_key: None,
        }
    }

    pub fn binaries(mut self, binaries: Vec<String>) -> Self {
        if binaries.len() < 2 {
            panic!("Must have at least two players");
        }
        if binaries.len() > 4 {
            panic!("Cannot have more than 4 players");
        }
        self.binaries = binaries;
        self
    }

    pub fn python_interpreter(mut self, python_interpreter : &str) -> Self {
        self.python_interpreter = python_interpreter.to_owned();
        self
    }

    pub fn initial_time(mut self, initial_time: Duration) -> Self {
        self.initial_time = initial_time;
        self
    } 

    pub fn increment(mut self, increment: Duration) -> Self {
        self.increment = increment;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn static_files(mut self, static_files: &str) -> Self {
        self.static_files = static_files.to_owned();
        self
    }

    pub fn send_to_web(mut self, active: bool, api_key: &str) -> Self {
        self.send_to_web = active;
        self.api_key = Some(api_key.to_owned());
        self
    }

    pub fn build(self) -> Arena {
        let card_lookup = Arc::new(Card::all());
        let num_players = self.binaries.len();
        let game = Game::new(num_players as u8, card_lookup);
        let clients = self.binaries;
        let python_interpreter = self.python_interpreter;
        let initial_time = self.initial_time;
        let increment = self.increment;
        let static_files = self.static_files;
        let port = self.port;
        let send_to_web = self.send_to_web;
        let api_key = self.api_key;

        Arena {
            game: game.clone(),
            replay: Either::Initialized(Replay::new(game)),
            clients,
            game_started: false,
            clock: Clock::new(num_players, initial_time, increment),
            python_interpreter : python_interpreter.to_owned(),
            static_files: static_files.to_owned(),
            port,
            send_to_web,
            api_key,
        }
    }
}

/// A module for running games across multiple clients. Can be fed binaries
/// and run them in a tournament style. The protocol for communication is
/// given by JSON messages across local websockets that update the game state.
pub struct Arena {
    game: Game,  // The Splendor game state
    pub clients: Vec<String>, // The binaries or python files to be run as clients
    clock: Clock, // The clock for keeping track of each player's time
    game_started: bool, // Whether the game has started
    replay: Either<Replay<Initialized>, FinalizedReplay>, // A representation of the game including
                                                          // the ability to walk through all
                                                          // previous moves
    python_interpreter : String, // The python interpreter to use
    static_files: String, // The location of the static files for the local web server
                          // visualization
    port : u16,           // The port to run the local web server on
    send_to_web: bool,  // Whether to send the game state to the global server
    api_key: Option<String>, // The api key to use for sending the game state to the global server
}


impl Arena {
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

impl Arena {
    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }
    pub async fn launch(self) {
        let init_binaries = self.clients.clone();
        let python_interpreter = self.python_interpreter.clone();
        let port = self.port;
        let static_files_loc = self.static_files.clone();

        let arena = self;
        // Keep track of the game state
        let arena = Arc::new(RwLock::new(arena));
        // Turn our arena state into a new Filter
        let arena = warp::any().map(move || arena.clone());

        // Keep track of all connected players
        let clients = Clients::default();
        // Turn our "clients" state into a new Filter...
        let clients = warp::any().map(move || clients.clone());

        let replay_post = warp::post()
            .and(warp::path("replay"));

        let replay_get = warp::get()
            .and(warp::path("replay"));

        let replay_next = replay_post
            .and(warp::path("next"))
            .and(arena.clone())
            .and_then(replay::next_move);

        let replay_prev = replay_post 
            .and(warp::path("previous"))
            .and(arena.clone())
            .and_then(replay::previous_move);

        let replay_goto = replay_post 
            .and(warp::path("goto"))
            .and(replay::json_body())
            .and(arena.clone())
            .and_then(replay::go_to_move);

        let replay_board_nobles = replay_get 
            .and(warp::path("nobles"))
            .and(arena.clone())
            .and_then(replay::board_nobles);

        let replay_board_cards = replay_get 
            .and(warp::path("cards"))
            .and(arena.clone())
            .and_then(replay::board_cards);

        let replay_board_decks = replay_get 
            .and(warp::path("decks"))
            .and(arena.clone())
            .and_then(replay::board_decks);

        let replay_board_bank = replay_get 
            .and(warp::path("bank"))
            .and(arena.clone())
            .and_then(replay::board_bank);

        let replay_board_players = replay_get
            .and(warp::path("players"))
            .and(arena.clone())
            .and_then(replay::board_players);

        let replay = replay_next
            .or(replay_prev)
            .or(replay_goto)
            .or(replay_board_nobles)
            .or(replay_board_cards)
            .or(replay_board_decks)
            .or(replay_board_bank)
            .or(replay_board_players);

        let time = warp::get()
            .and(warp::path("time"))
            .and(arena.clone())
            .and_then(clock::current_time_remaining);

        let game = warp::path("game")
            .and(warp::ws())
            .and(clients)
            .and(arena.clone())
            .map(|ws: warp::ws::Ws, clients, arena| {
                ws.on_upgrade(move |socket| user_connected(socket, clients, arena))
            });

        let log = warp::path("log")
            .and(warp::ws())
            .map(|ws: warp::ws::Ws| ws.on_upgrade(move |socket| log_stream_connected(socket)));

        let splendor = warp::path("splendor").and(warp::fs::dir(static_files_loc.clone()));
        let static_files = warp::path("static_files").and(warp::fs::dir(static_files_loc));

        let routes = game.or(log).or(replay).or(time).or(splendor).or(static_files);

        tokio::spawn(async move {
            // TODO: use a handshake protocol instead of timing
            for binary in init_binaries {
                tokio::time::sleep(Duration::from_secs(1)).await;
                // Launches without stdout, we rely on the logs for that
                if binary.ends_with(".py") {
                    match std::process::Command::new(&python_interpreter)
                        .arg(binary.clone())
                        .arg(format!("--port={}", port))
                        .stdout(std::process::Stdio::null())
                        .spawn()
                    {
                        Ok(_) => info!("Launched python3 script {}", binary),
                        Err(e) => error!("Failed to launch python3 script {}: {}", binary, e),
                    }
                } else {
                    match std::process::Command::new(binary.clone())
                        .arg(format!("--port={}", port))
                        .stdout(std::process::Stdio::null())
                        .spawn()
                    {
                        Ok(_) => info!("Launched binary {}", binary),
                        Err(e) => error!("Failed to launch binary {}: {}", binary, e),
                    }
                }
            }
        });
        // TODO: tokio::spawn a task that sends the game state to the global server

        // Start the server on localhost at the specified port
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }
}

pub struct GameResults {}
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
