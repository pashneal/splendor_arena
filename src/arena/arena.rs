use super::*;
use crate::card::Card;
use crate::game_logic::*;
use crate::player::*;
use crate::JSONable;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::arena::clock::*;
use crate::arena::protocol::*;
use crate::arena::replay::*;

use log::{debug, error, info, trace};
use warp::Filter;

/// TODO: Remove responsibility for launching clients from the Arena
/// TODO: Remove replay

/// Builder for creating an arena,
/// allows clients to flexibly include sane defaults or override them
/// with given parameters
pub struct ArenaBuilder {
    initial_time: Duration,
    increment: Duration,
    port: u16,
    send_to_web: bool,
    api_key: Option<String>,
    num_players: Option<usize>,
}

impl ArenaBuilder {
    pub fn new() -> Self {
        ArenaBuilder {
            initial_time: Duration::from_secs(60),
            increment: Duration::from_secs(0),
            port: 3030,
            send_to_web: false,
            api_key: None,
            num_players: None,
        }
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

    pub fn send_to_web(mut self, active: bool, api_key: &str) -> Self {
        self.send_to_web = active;
        self.api_key = Some(api_key.to_owned());
        self
    }

    pub fn num_players(mut self, num_players: usize) -> Self {
        self.num_players = Some(num_players);
        self
    }

    pub fn build(self) -> Arena {
        assert!(self.num_players.is_some(), "Number of players must be set");

        let card_lookup = Arc::new(Card::all());
        let num_players = self.num_players.unwrap();
        let game = Game::new(num_players as u8, card_lookup);
        let initial_time = self.initial_time;
        let increment = self.increment;
        let port = self.port;
        let send_to_web = self.send_to_web;
        let api_key = self.api_key;
        let mut clients = Vec::new();
        for _ in 0..num_players {
            clients.push(ClientId(rand::random()));
        }

        Arena {
            game: game.clone(),
            replay: Either::Initialized(Replay::new(game)),
            game_started: false,
            clock: Clock::new(num_players, initial_time, increment),
            port,
            clients,
            send_to_web,
            api_key,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy)]
pub struct GameId(pub u64);
impl GameId {
    pub fn new() -> Self {
        GameId(rand::random())
    }
}
#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy)]
pub struct ClientId(pub u64);
impl ClientId {
    pub fn new() -> Self {
        ClientId(rand::random())
    }
}

/// A module for running games across multiple clients. Can be fed binaries
/// and run them in a tournament style. The protocol for communication is
/// given by JSON messages across local websockets that update the game state.
pub struct Arena {
    game: Game,                                           // The Splendor game state
    clock: Clock,       // The clock for keeping track of each player's time
    game_started: bool, // Whether the game has started
    replay: Either<Replay<Initialized>, FinalizedReplay>, // A representation of the game including
    // the ability to walk through all
    // previous moves
    clients: Vec<ClientId>,  // The clients connected to the game
    port: u16,               // The port to run the local web server on
    send_to_web: bool,       // Whether to send the game state to the global server
    api_key: Option<String>, // The api key to use for sending the game state to the global server
}

impl Arena {
    pub fn is_game_over(&self) -> bool {
        self.game.game_over()
    }

    pub fn small_client_info(&self) -> SmallClientInfo {
        let client_info = self.client_info();
        SmallClientInfo {
            board: client_info.board,
            players: client_info.players,
            current_player_num: client_info.current_player_num,
        }
    }

    pub fn client_info(&self) -> ClientInfo {
        let players = self.game.players().iter().map(|p| p.to_public()).collect();
        let legal_actions = self
            .game
            .get_legal_actions()
            .expect("Cannot get legal actions");

        let time_endpoint_url = format!("http://127.0.0.1:{}/time", self.port);

        ClientInfo {
            board: Board::from_game(&self.game),
            history: self.game.history(),
            players,
            current_player: self.game.current_player(),
            current_player_num: self.game.current_player_num(),
            legal_actions,
            time_endpoint_url,
            phase: self.game.phase(),
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
    pub fn play_action(&mut self, action: Action) {
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

    pub fn current_player_id(&self) -> Option<ClientId> {
        if self.game_started {
            Some(self.clients[self.game.current_player_num()])
        } else {
            None
        }
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

    pub fn num_moves(&self) -> usize {
        self.game.history().num_moves() as usize
    }

    pub fn allowed_clients(&self) -> Vec<ClientId> {
        self.clients.clone()
    }
}

impl Arena {
    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }

    pub async fn launch(self) {
        let port = self.port;
        let send_to_web = self.send_to_web;

        let arena = self;
        // Keep track of the game state
        let arena = Arc::new(RwLock::new(arena));
        let arena_clone = arena.clone();
        let arena_filter = warp::any().map(move || arena.clone());

        // Keep track of all connected players
        let clients = Clients::default();
        let clients_filter = warp::any().map(move || clients.clone());

        let time = warp::get()
            .and(warp::path("time"))
            .and(arena_filter.clone())
            .and_then(clock::current_time_remaining);

        let write_to_file = send_to_web.clone();
        let write_to_file = warp::any().map(move || write_to_file.clone());

        let log = warp::path!("log" / u64)
            .and(warp::ws())
            .and(write_to_file)
            .map(|clientid, ws: warp::ws::Ws, write_to_file| {
                ws.on_upgrade(move |socket| {
                    log_stream_connected(ClientId(clientid), socket, write_to_file)
                })
            });

        let mut web_stream: Option<Outgoing> = None;

        // Send to stourney.com if send_to_web is true
        if send_to_web {
            debug!("Connecting to global server...");
            let outgoing = match web::start(arena_clone).await {
                Ok((outgoing, _)) => outgoing,
                Err(e) => {
                    error!("Failed to connect to global server: {}", e);
                    return;
                }
            };
            web_stream = Some(outgoing);
        }

        debug!("Starting local server on port {}", port);

        let web_stream_filter = warp::any().map(move || web_stream.clone());
        let game = warp::path!("game" / u64 / u64)
            .and(warp::ws())
            .and(clients_filter)
            .and(arena_filter.clone())
            .and(web_stream_filter)
            .map(
                |_gameid, clientid, ws: warp::ws::Ws, clients, arena, web_stream| {
                    ws.on_upgrade(move |socket| {
                        user_connected(ClientId(clientid), socket, clients, arena, web_stream)
                    })
                },
            );

        let routes = game.or(log).or(time);
        // Start the server on localhost at the specified port
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(self.launch())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Broadcast(BroadcastInfo),
    PlayerActionRequest(ClientInfo),
}

pub struct GameResults {}
/// A struct given to each client that contains all public information and private
/// information known only to that client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub board: Board,
    pub history: GameHistory,
    pub phase: Phase,
    pub players: Vec<PlayerPublicInfo>,
    pub current_player: Player,
    pub current_player_num: usize,
    pub legal_actions: Vec<Action>,
    pub time_endpoint_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastInfo {
    pub board: Board,
    pub history: GameHistory,
    pub players: Vec<PlayerPublicInfo>,
    pub current_player_num: usize,
    pub phase: Phase,
}

impl From<ClientInfo> for BroadcastInfo {
    fn from(info: ClientInfo) -> Self {
        BroadcastInfo {
            board: info.board,
            history: info.history,
            players: info.players,
            current_player_num: info.current_player_num,
            phase: info.phase,
        }
    }
}

/// A struct given to each client that contains all public information and private
/// information known only to that client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallClientInfo {
    pub board: Board,
    pub players: Vec<PlayerPublicInfo>,
    pub current_player_num: usize,
}

impl JSONable for ClientInfo {}
