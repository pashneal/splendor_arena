// Helper functions local game server that interacts with the game logic, validates moves
// from the clients, and send the game state back to the clients after each move

use super::*;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use derive_more::{Display, Error};
use futures_util::{stream::SplitSink, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
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

pub async fn validate_action(action: &Action, player_id: usize, arena: GlobalArena) -> bool {
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

pub async fn log_stream_connected(socket: WebSocket) {
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
pub async fn user_connected(ws: WebSocket, clients: Clients, arena: GlobalArena, web_stream : Option<Outgoing>) {
    let (client_tx, mut client_rx) = ws.split();
    let my_id = CLIENT_ID.fetch_add(1, Ordering::Relaxed);
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
                        play_default_action(my_id, clients.clone(), arena.clone(), outgoing_clone.clone()).await;
                        continue;
                    }
                    let msg = msg.unwrap();

                    let client_msg = parse_message(&msg);
                    if let Err(e) = client_msg {
                        error!("error parsing message from json string! {:?}", e);
                        play_default_action(my_id, clients.clone(), arena.clone(), outgoing_clone.clone()).await;
                        continue;
                    }

                    match client_msg.unwrap() {
                        ClientMessage::Action(action) => {
                            if !validate_action(&action, my_id, arena.clone()).await {
                                play_default_action(my_id, clients.clone(), arena.clone(), outgoing_clone.clone()).await;
                                continue;
                            }

                            trace!("{} played {:?}", my_id, action);
                            arena.write().await.play_action(action);
                            action_played(clients.clone(), arena.clone(), outgoing_clone.clone()).await;
                        }
                        ClientMessage::Log(log) => {
                            error!("Logs sent to the wrong endpoint! {:?}", log);
                            continue;
                        }
                    }
                }
                Ok(_) => panic!("unexpected None"),
                Err(e) => {
                    play_default_action(my_id, clients.clone(), arena.clone(), outgoing_clone.clone()).await;
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
        game_initialized(init_clients, init_arena, outgoing.clone()).await;
    }
}
pub async fn play_default_action(my_id : usize, clients: Clients, arena: GlobalArena, web_stream : Option<Outgoing>) {
    if arena.read().await.is_game_over() {
        return;
    }
    
    println!("[Turn : {}] [Player {} (crashed/timed out)] Playing a random move...", TURN_COUNTER.load(Ordering::SeqCst), my_id);
    let action = arena.read().await.get_legal_actions().unwrap()[0].clone();
    arena.write().await.play_action(action);
    action_played(clients.clone(), arena.clone(), web_stream.clone()).await;
}

pub async fn game_initialized(clients: Clients, arena: GlobalArena, web_stream : Option<Outgoing>) {
    info!("All users locked and loaded! Game starting!");
    arena.write().await.start_game();
    action_played(clients, arena, web_stream).await;
}

pub async fn user_initialized(my_id: usize, clients: Clients, arena: GlobalArena) {
    info!("{} connected", my_id);
}

pub async fn user_disconnected(my_id: usize, clients: Clients, arena: GlobalArena) {
    clients.write().await.remove(&my_id);
}

pub async fn action_played(clients: Clients, arena: GlobalArena, web_stream : Option<Outgoing>) {
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

    if  web_stream.is_some() {
        web::push_game_update(web_stream.unwrap(), arena.clone()).await;
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
