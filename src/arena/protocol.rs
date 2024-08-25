use super::*;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use derive_more::{Display, Error};
use futures_util::{stream::SplitSink, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use tokio::time::timeout;

use log::{debug, error, info, trace};

pub type Clients = Arc<RwLock<HashMap<usize, SplitSink<WebSocket, Message>>>>;
pub type GlobalArena = Arc<RwLock<Arena>>;
pub type GlobalGameHistory = Arc<RwLock<GameHistory>>;

type StdError = Box<dyn std::error::Error>;

const TIMEOUT: Duration = Duration::from_secs(4);

static CLIENT_ID: AtomicUsize = AtomicUsize::new(0);
static TURN_COUNTER: AtomicUsize = AtomicUsize::new(0);
static LAST_PLAYER: AtomicUsize = AtomicUsize::new(5);

impl Arena {
    pub async fn launch(port: u16, binaries: Vec<String>, num_players: u8, initial_time: Duration, increment: Duration) {
        let init_binaries = binaries.clone();
        let arena = Arena::new(num_players, binaries, initial_time, increment);
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

        let static_files = warp::path("splendor").and(warp::fs::dir("frontend"));

        let routes = game.or(log).or(replay).or(time).or(static_files);

        tokio::spawn(async move {
            // TODO: use a handshake protocol instead of timing
            for binary in init_binaries {
                tokio::time::sleep(Duration::from_secs(1)).await;
                // Launches without stdout, we rely on the logs for that
                if binary.ends_with(".py") {
                    match std::process::Command::new("python3")
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

        // Start the server on localhost at the specified port
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }
}

#[derive(Debug, Display, Error)]
pub enum ParseError {
    #[display(fmt = "Unknown")]
    Unknown,
    #[display(fmt = "Cannot convert client message to string")]
    CannotConvertToString,
    #[display(fmt = "Cannot convert string to client message")]
    CannotConvertToClientMessage,
    #[display(fmt = "Message too long to display")]
    MessageTooLong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Action(Action),
    Log(String),
}

fn parse_message(message_text: &Message) -> Result<ClientMessage, ParseError> {
    let message_str = message_text
        .to_str()
        .map_err(|_| ParseError::CannotConvertToString)?;
    let client_msg: ClientMessage =
        serde_json::from_str(message_str).map_err(|_| ParseError::CannotConvertToClientMessage)?;
    Ok(client_msg)
}

async fn validate_action(action: &Action, player_id: usize, arena: GlobalArena) -> bool {
    // -> The current player is not timed out  
    if arena.read().await.is_timed_out(){
        error!("Player {} is timed out!", player_id);
        return false;
    }

    // -> Is a legal action
    let actions = arena.read().await.get_legal_actions();
    if actions.is_none() {
        error!("No legal actions found!");
        return false;
    }

    let actions = actions.unwrap();
    if !actions.contains(action) {
        error!("Illegal action: {:?}", action);
        return false;
    }

    // -> Is the correct player's turn
    if arena.read().await.current_player_num() != Some(player_id) {
        error!("Not player {}'s turn!", player_id);
        return false;
    }


    return true;

}

async fn log_stream_connected(socket: WebSocket) {
    // TODO: This makes an assumption that
    // the client that last connected is the one that is logging
    // This may not be a good assumption
    let id = CLIENT_ID.load(Ordering::Relaxed) - 1;

    let (_tx, mut rx) = socket.split();
    while let Some(msg) = rx.next().await {
        if let Err(e) = msg {
            trace!("error receiving message! breaking {:?}", e);
            break;
        }
        let msg = msg.unwrap();

        let client_msg = parse_message(&msg);
        if let Err(e) = client_msg {
            error!("error parsing message! {:?}", e);
            break;
        }
        match client_msg.unwrap() {
            ClientMessage::Action(action) => {
                error!("Actions sent to the wrong endpoint! {:?}", action);
                break;
            }
            ClientMessage::Log(log) => {
                println!(
                    "[Turn : {}] [Player {}]: {}",
                    TURN_COUNTER.load(Ordering::SeqCst),
                    id,
                    log
                );
            }
        }
    }
}

/// Setup a new client to play the game
async fn user_connected(ws: WebSocket, clients: Clients, arena: GlobalArena) {
    let (client_tx, mut client_rx) = ws.split();
    let my_id = CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    clients.write().await.insert(my_id, client_tx);

    let init_clients = clients.clone();
    let init_arena = arena.clone();
    let num_players = init_arena.read().await.players().len();

    // Convert messages from the client into a stream of actions
    // So we play them in the game as soon as they come in
    tokio::spawn(async move {
        loop {
            // Wait until all players are connected
            // and it is the current player's turn
            while (
                arena.read().await.current_player_num() != Some(my_id)
                && !arena.read().await.is_game_over()
            ) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            if arena.read().await.is_game_over() {
                break;
            }

            // Give a little extra time to account for network + server latency
            let time_remaining = arena.read().await.time_remaining();
            let time_remaining = time_remaining + Duration::from_millis(10);

            match timeout(time_remaining, client_rx.next()).await {
                Ok(Some(msg)) => {
                    trace!("Received message: {:?}", msg);
                    if let Err(e) = msg {
                        play_default_action(my_id, clients.clone(), arena.clone()).await;
                        continue;
                    }
                    let msg = msg.unwrap();

                    let client_msg = parse_message(&msg);
                    if let Err(e) = client_msg {
                        error!("error parsing message from json string! {:?}", e);
                        play_default_action(my_id, clients.clone(), arena.clone()).await;
                        continue;
                    }
                    match client_msg.unwrap() {
                        ClientMessage::Action(action) => {
                            if !validate_action(&action, my_id, arena.clone()).await {
                                play_default_action(my_id, clients.clone(), arena.clone()).await;
                                continue;
                            }

                            trace!("{} played {:?}", my_id, action);
                            arena.write().await.play_action(action);
                            action_played(clients.clone(), arena.clone()).await;
                        }
                        ClientMessage::Log(log) => {
                            error!("Logs sent to the wrong endpoint! {:?}", log);
                            continue;
                        }
                    }
                }
                Ok(_) => panic!("unexpected None"),
                Err(e) => {
                    play_default_action(my_id, clients.clone(), arena.clone()).await;
                }
            }
        }
        info!("Player {} disconnected", my_id);
        user_disconnected(my_id, clients, arena).await;
    });

    let num_players = init_arena.read().await.players().len();
    user_initialized(my_id, init_clients.clone(), init_arena.clone()).await;

    // All users are connected, start the game
    if my_id == num_players - 1 {
        game_initialized(init_clients, init_arena).await;
    }
}
async fn play_default_action(my_id : usize, clients: Clients, arena: GlobalArena) {
    if arena.read().await.is_game_over() {
        return;
    }
    
    println!("[Turn : {}] [Player {} (crashed/timed out)] Playing a random move...", TURN_COUNTER.load(Ordering::SeqCst), my_id);
    let action = arena.read().await.get_legal_actions().unwrap()[0].clone();
    arena.write().await.play_action(action);
    action_played(clients.clone(), arena.clone()).await;
}

async fn game_initialized(clients: Clients, arena: GlobalArena) {
    info!("All users locked and loaded! Game starting!");
    arena.write().await.start_game();
    action_played(clients, arena).await;
}

async fn user_initialized(my_id: usize, clients: Clients, arena: GlobalArena) {
    info!("{} connected", my_id);
}

async fn user_disconnected(my_id: usize, clients: Clients, arena: GlobalArena) {
    clients.write().await.remove(&my_id);
}

async fn action_played(clients: Clients, arena: GlobalArena) {
    // Auto play for any given player if there is only 1 legal action
    loop {
        // If the game is over, don't do anything else
        if arena.read().await.is_game_over() {
            info!("Game over!");
            let winner = arena.read().await.get_winner();
            match winner {
                Some(winner) => info!("Winner: Player {:?}", winner),
                None => info!("No winner! Draw!"),
            }
            arena.write().await.finalize_game();

            return;
        }

        let actions = arena
            .read()
            .await
            .get_legal_actions()
            .expect("Cannot get legal actions");
        if actions.len() != 1 {
            break;
        }
        let action = actions[0].clone();
        trace!("Auto played action: {:?}", action);
        arena.write().await.play_action(action);
    }
    let last_player = arena.read().await.current_player_num().expect("No current player, is the game started?");

    if LAST_PLAYER.load(Ordering::SeqCst) != last_player {
        TURN_COUNTER.fetch_add(1, Ordering::SeqCst);
        LAST_PLAYER.store(last_player, Ordering::SeqCst);
    }

    trace!("Sending game state to clients...");
    // Determine which client to send the next game state to
    let client_info = arena.read().await.client_info();
    let player_num = client_info.current_player_num;

    // Wait up to TIMEOUT for the player to come online and make a move
    if let None = clients.read().await.get(&player_num) {
        tokio::time::sleep(TIMEOUT).await;
    }

    trace!("Sending game state to player {}", player_num);
    if let Some(tx) = clients.write().await.get_mut(&player_num) {
        let info_str = serde_json::to_string(&client_info).unwrap();
        let info = Message::text(info_str);
        tx.send(info).await.unwrap();
        trace!("Sent game state!");
    } else {
        panic!("no tx for client with id {}", player_num);
    }
}
