// Helper functions local game server that interacts with the game logic, validates moves
// from the clients, and send the game state back to the clients after each move

use super::*;
use crate::constants::DEFAULT_LOG_FILENAME;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use derive_more::{Display, Error};
use futures_util::{stream::SplitSink, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use warp::ws::{Message, WebSocket};
use warp::Filter;

use log::{debug, error, info, trace};

pub type Clients = Arc<RwLock<HashMap<ClientId, SplitSink<WebSocket, Message>>>>;
pub type GlobalArena = Arc<RwLock<Arena>>;
pub type GlobalGameHistory = Arc<RwLock<GameHistory>>;

type StdError = Box<dyn std::error::Error>;

const TIMEOUT: Duration = Duration::from_secs(4);

static CLIENT_ID: AtomicUsize = AtomicUsize::new(0);
static TURN_COUNTER: AtomicUsize = AtomicUsize::new(0);

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

pub async fn validate_action(action: &Action, client_id: ClientId, arena: GlobalArena) -> bool {
    // -> The current player is not timed out
    if arena.read().await.is_timed_out() {
        error!("Player {:?} is timed out!", client_id);
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
    if arena.read().await.current_player_id() != Some(client_id) {
        error!("Not player {:?}'s turn!", client_id);
        return false;
    }

    return true;
}

pub async fn log_stream_connected(client_id : ClientId, socket: WebSocket, write_to_file: bool) {
    let id = client_id;

    let mut file = if write_to_file {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(DEFAULT_LOG_FILENAME)
            .await;
        if let Err(e) = file {
            error!("error opening file! {:?}", e);
            return;
        }
        Some(file.unwrap())
    } else {
        None
    };

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
                let message = format!(
                    "[Turn : {}] [Player {:?}]: {}",
                    TURN_COUNTER.load(Ordering::SeqCst),
                    id,
                    log
                );

                if write_to_file {
                    let file = file.as_mut().unwrap();
                    if let Err(e) =
                        tokio::io::AsyncWriteExt::write_all(file, (message + "\n").as_bytes()).await
                    {
                        error!("error writing to file! {:?}", e);
                        break;
                    }
                } else {
                    println!("{}", message);
                }
            }
        }
    }
}

/// Setup a new client to play the game
pub async fn user_connected(
    client_id: ClientId,
    ws: WebSocket,
    clients: Clients,
    arena: GlobalArena,
    web_stream: Option<Outgoing>,
) {
    let (client_tx, mut client_rx) = ws.split();
    let my_id = client_id;

    let allowed = arena.read().await.allowed_clients();
    if !allowed.contains(&my_id) {
        error!("Player {:?} not allowed to play!", my_id);
        error!("Exiting...");
        return;
    }

    if clients.read().await.get(&my_id).is_some() {
        error!("Player {:?} already connected!", my_id);
        error!("Exiting...");
        return;
    }

    clients.write().await.insert(my_id, client_tx);

    let init_clients = clients.clone();
    let init_arena = arena.clone();
    let num_players = init_arena.read().await.players().len();

    let outgoing = web_stream.clone();
    let outgoing_clone = outgoing.clone();

    // Convert messages from the client into a stream of actions
    // So we play them in the game as soon as they come in
    tokio::spawn(async move {
        loop {
            // Wait until all players are connected
            // and it is the current player's turn
            while (arena.read().await.current_player_id() != Some(my_id)
                && !arena.read().await.is_game_over())
            {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            if arena.read().await.is_game_over() {
                break;
            }

            let time_remaining = arena.read().await.time_remaining();

            match timeout(time_remaining, client_rx.next()).await {
                Ok(Some(msg)) => {
                    trace!("Received message: {:?}", msg);
                    if let Err(e) = msg {
                        play_default_action(
                            my_id,
                            clients.clone(),
                            arena.clone(),
                            outgoing_clone.clone(),
                        )
                        .await;
                        continue;
                    }
                    let msg = msg.unwrap();

                    let client_msg = parse_message(&msg);
                    if let Err(e) = client_msg {
                        error!("error parsing message from json string! {:?}", e);
                        play_default_action(
                            my_id,
                            clients.clone(),
                            arena.clone(),
                            outgoing_clone.clone(),
                        )
                        .await;
                        continue;
                    }

                    match client_msg.unwrap() {
                        ClientMessage::Action(action) => {
                            if !validate_action(&action, my_id, arena.clone()).await {
                                error!("Invalid action: {:?}", action);
                                play_default_action(
                                    my_id,
                                    clients.clone(),
                                    arena.clone(),
                                    outgoing_clone.clone(),
                                )
                                .await;
                                continue;
                            }

                            trace!("{:?} played {:?}", my_id, action);
                            arena.write().await.play_action(action);
                            action_played(clients.clone(), arena.clone(), outgoing_clone.clone())
                                .await;
                        }
                        ClientMessage::Log(log) => {
                            error!("Logs sent to the wrong endpoint! {:?}", log);
                            continue;
                        }
                    }
                }
                Ok(_) => panic!("unexpected None"),
                Err(e) => {
                    play_default_action(
                        my_id,
                        clients.clone(),
                        arena.clone(),
                        outgoing_clone.clone(),
                    )
                    .await;
                }
            }
        }
        info!("Player {:?} disconnected", my_id);
        user_disconnected(my_id, clients, arena).await;
    });

    let num_players = init_arena.read().await.players().len();
    user_initialized(my_id, init_clients.clone(), init_arena.clone()).await;

    // All users are connected, start the game
    if init_clients.read().await.len() == num_players {
        game_initialized(init_clients, init_arena, outgoing.clone()).await;
    }
}
pub async fn play_default_action(
    my_id: ClientId,
    clients: Clients,
    arena: GlobalArena,
    web_stream: Option<Outgoing>,
) {
    if arena.read().await.is_game_over() {
        return;
    }

    println!(
        "[Turn : {}] [Player {:?} (crashed/timed out)] Playing a random move...",
        TURN_COUNTER.load(Ordering::SeqCst),
        my_id
    );
    let action = arena.read().await.get_legal_actions().unwrap()[0].clone();
    arena.write().await.play_action(action);
    action_played(clients.clone(), arena.clone(), web_stream.clone()).await;
}

pub async fn game_initialized(clients: Clients, arena: GlobalArena, web_stream: Option<Outgoing>) {
    info!("All users locked and loaded! Game starting!");
    arena.write().await.start_game();
    action_played(clients, arena, web_stream).await;
}

pub async fn user_initialized(my_id: ClientId, clients: Clients, arena: GlobalArena) {
    info!("{:?} connected", my_id);
}

pub async fn user_disconnected(my_id: ClientId, clients: Clients, arena: GlobalArena) {
    clients.write().await.remove(&my_id);
}


/// Sends a broadcast message of the current game state to all clients
pub async fn broadcast(clients: Clients, arena: GlobalArena) {
    let client_info = arena.read().await.client_info();
    let broadcast_info = BroadcastInfo::from(client_info);
    let broadcast_info = ServerMessage::Broadcast(broadcast_info);
    let broadcast_info = serde_json::to_string(&broadcast_info).unwrap();
    let broadcast = Message::text(broadcast_info);
    for (client_id, tx) in clients.write().await.iter_mut() {
        tx.send(broadcast.clone()).await.unwrap();
    }

}


/// Play actions automatically for a player until they have more than
/// one legal action, also updates a connected web server with the game state
pub async fn auto_play(clients: Clients, arena: GlobalArena, web_stream: Option<Outgoing>){
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
        broadcast(clients.clone(), arena.clone()).await;
        let stream = web_stream.clone();

        // An action was played, be sure to send the game state to the web server
        if stream.is_some() {
            web::push_game_update(stream.unwrap(), arena.clone()).await;
        }
    }
}

/// Is called whenever an action is played
pub async fn action_played(clients: Clients, arena: GlobalArena, web_stream: Option<Outgoing>) {
    broadcast(clients.clone(), arena.clone()).await;
    //  An action was played, be sure to send the game state to the web server
    //  if it is connected
    let stream = web_stream.clone();
    if stream.is_some() {
        web::push_game_update(stream.unwrap(), arena.clone()).await;
    }

    // TODO: reconsider usage of auto_play, as this complicates
    // the mental model of the game server
    // auto_play(clients.clone(), arena.clone(), web_stream.clone()).await;

    let last_player = arena
        .read()
        .await
        .current_player_id()
        .expect("No current player, is the game started?");

    let num_moves = arena.read().await.num_moves();
    TURN_COUNTER.swap(num_moves, Ordering::SeqCst);

    trace!("Sending game state to clients...");

    // Determine which client to send the next game state to
    let player_num = arena.read().await.current_player_id().expect("No current player, but the game has already started");
    let client_info = arena.read().await.client_info();
    let action_request = ServerMessage::PlayerActionRequest(client_info);

    // Wait up to TIMEOUT for the player to come online and make a move
    // TODO: This is a hacky way to wait for the player to come online
    if let None = clients.read().await.get(&player_num) {
        tokio::time::sleep(TIMEOUT).await;
    }

    trace!("Sending game state to player {:?}", player_num);
    if let Some(tx) = clients.write().await.get_mut(&player_num) {
        let info_str = serde_json::to_string(&action_request).unwrap();
        let info = Message::text(info_str);
        tx.send(info).await.unwrap();
        trace!("Sent game state!");
    } else {
        panic!("no tx for client with id {:?}", player_num);
    }
}
