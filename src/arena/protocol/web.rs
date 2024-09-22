use crate::models::*;
use crate::constants;
use super::{Arena, GlobalArena};
use futures_util::{stream::SplitSink, stream::SplitStream,  SinkExt, StreamExt};
use log::{debug, info, trace, error, warn};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tokio::sync::RwLock;
use std::sync::Arc;

pub type Outgoing = Arc<RwLock<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>;
pub type Incoming = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub fn handle_info(message : &str) {
    info!("stourney.com says: {}", message);
}

pub fn handle_error(message : &str) {
    error!("stourney.com says: {}", message);
}

pub fn handle_warning(message : &str) {
    warn!("stourney.com says: {}", message);
}

pub fn handle_failure() {
    error!("Failed to communicate with stourney.com");
}

pub fn handle_timeout() {
}

/// Given a stream to the global server, sends a heartbeat message every 60 seconds
/// to keep the connection alive
pub async fn maintain_heartbeat(outgoing_stream : Outgoing) {
    loop {
        {
            let mut outgoing_stream = outgoing_stream.write().await;
            let heartbeat = serde_json::to_string(&ArenaRequest::Heartbeat).unwrap();
            let message = Message::text(heartbeat);
            debug!("Sending heartbeat to global server...");
            let _ = outgoing_stream.send(message).await;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

pub fn push_reconnect() {
}

/// Pushes the current game state to the global server,
/// TODO: wait for response confirming the update was successful
/// TODO: if necessary, batch updates
pub async fn push_game_update(
    outgoing_stream : Outgoing,
    arena: GlobalArena,
) {
    let mut outgoing_stream = outgoing_stream.write().await;
    let arena = arena.read().await;
    let game_update = get_game_update(&arena).await.expect("Failed to get game update");
    let message = serde_json::to_string(&game_update).expect("Failed to serialize game update");
    let message = Message::text(message);
    debug!("Sending game update to global server...");
    outgoing_stream.send(message).await.expect("Failed to send game update");
}


pub async fn get_game_update(arena : &Arena) -> Result<ArenaRequest, ()> {
    let game_state = arena.small_client_info();
    match arena.client_info().history.num_actions() {
        0 => {
            return Ok(ArenaRequest::InitializeGame{ info: game_state })
        }
        _ => {
            let num_moves = arena.client_info().history.num_moves();
            let game_update = GameUpdate {
                info: game_state,
                update_num: num_moves as usize + 1,
            };
            return Ok(ArenaRequest::GameUpdates(vec![game_update]))
        }
    }

    todo!();
}

/// Pushes an initial game state to the global server
/// and waits for a response, returning the id of the game if the initialization
/// was successful, and None otherwise
///
/// Assumes no moves were made in the game yet
pub async fn push_initial_game(
    outgoing_stream : Outgoing,
    incoming_stream : &mut Incoming,
    arena: GlobalArena,
) -> Option<String> {
    debug!("Pushing initial game state to global server...");

    let arena = arena.read().await;
    assert!(arena.client_info().history.num_moves() == 0);

    let game_update = get_game_update(&arena).await.expect("Failed to get game update");
    let game_update = serde_json::to_string(&game_update).expect("Failed to serialize game update");
    let message = Message::text(game_update);

    debug!("Sending initial game state to global server...");
    debug!("message: {:?}", message);
    let mut outgoing_stream = outgoing_stream.write().await;
    let result = match outgoing_stream.send(message).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to send initial game state to global server: {}", e);
            handle_failure();
            return None
        }
    };

    //TODO: add timeout?
    while let Some(msg) = incoming_stream.next().await {
        debug!("Received message from global server...");
        let msg = msg.expect("Failed to receive message from global server");
        let msg = msg.to_string();

        debug!("Received message from global server: {}", msg);
        
        let msg = serde_json::from_str::<GlobalServerResponse>(&msg);
        debug!("Deserialized message from global server: {:?}", msg);

        if msg.is_err() {
            error!("Failed to deserialize message from global server: {}", msg.err().unwrap());
            handle_failure();
            return None
        }
        let msg = msg.unwrap();


        match msg {
            GlobalServerResponse::Initialized(Initialized::Success{ id, url }) => {
                debug!("Successfully initialized with stourney.com");
                return Some(url)
            },
            GlobalServerResponse::Initialized(Initialized::Failure{ reason }) => {
                error!("Failed to initialize with stourney.com: {}", reason);
                handle_failure();
                return None
            }

            GlobalServerResponse::Warning(msg) => handle_warning(&msg),
            GlobalServerResponse::Error(msg) => handle_error(&msg),
            GlobalServerResponse::Info(msg) => handle_info(&msg),
            _ => {
                error!("Unexpected response from global server: {:?}", msg);
                handle_failure();
                return None
            }
        }
    };

    return None;
}

/// Pushes an authentication request to the global server,
/// and waits for a authenticated response, returning true if the authentication
/// was successful, and false otherwise
/// TODO: add error handling
pub async fn push_authentication(outgoing_stream : Outgoing, incoming_stream : &mut Incoming, arena: GlobalArena) -> bool {
    let arena = arena.read().await;
    let api_key = arena.api_key().clone();
    let api_key = api_key.expect("Should be connecting to global server without key");

    let auth_req = ArenaRequest::Authenticate{ secret: api_key};
    let auth_req = serde_json::to_string(&auth_req);
    let auth_req = auth_req.expect("Failed to serialize authentication request");


    let message = Message::text(auth_req);
    debug!("Sending authentication request to global server...");
    info!("Contacting stourney.com...");
    {
        let mut outgoing_stream = outgoing_stream.write().await;
        outgoing_stream.send(message).await.expect("Failed to send authentication request");

    }
    debug!("Contacted stourney.com...");
    //TODO: add timeout?
    while let Some(msg) = incoming_stream.next().await {
        debug!("Received message from global server...");
        let msg = msg.expect("Failed to receive message from global server");
        let msg = msg.to_string();
        let msg : GlobalServerResponse = serde_json::from_str(&msg).expect("Failed to deserialize message from global server");
        debug!("Received message from global server: {:?}", msg);

        match msg {
            GlobalServerResponse::Authenticated(Authenticated::Success) => {
                debug!("Successfully authenticated with stourney.com");
                return true
            },
            GlobalServerResponse::Authenticated(Authenticated::Failure{ reason }) => {
                error!("Failed to authenticate with stourney.com: {}", reason);
                return false
            }

            GlobalServerResponse::Warning(msg) => handle_warning(&msg),
            GlobalServerResponse::Error(msg) => handle_error(&msg),
            GlobalServerResponse::Info(msg) => handle_info(&msg),
            _ => {
                error!("Unexpected response from global server: {:?}", msg);
                handle_failure();
                return false
            }
        }
    }
    debug!("Failed to receive message from global server...");
    return false
}

pub fn push_game_over() {
}

pub fn push_debug_message() {
}

/// Depending on the state of the global server,
/// updates a queue of actions to be sent to the server,
/// so as the minimize the number of messages sent
pub fn update_queue(arena : GlobalArena) {
}


/// Run and manage the connection to the global server
pub async fn start(arena : GlobalArena) -> Result<(Outgoing, Incoming), String >{
    let websocket = match connect_async(constants::STOURNEY_WEBSOCKET_URL).await {
       Ok((websocket, _)) => websocket,
       Err(e) => {
           error!("Failed to connect to stourney.com: {}", e);
           return Err("Failed to connect to stourney.com".to_owned())
       }
    };

    debug!("Connected to stourney.com");

    let (outgoing_stream, mut incoming_stream) = websocket.split();
    let outgoing_stream = Arc::new(RwLock::new(outgoing_stream));

    let auth = push_authentication(outgoing_stream.clone(), &mut incoming_stream, arena.clone()).await;
    if !auth {
        return Err("Failed to authenticate with stourney.com".to_owned())
    }

    let url = push_initial_game(outgoing_stream.clone(), &mut incoming_stream, arena).await;

    if url.is_none() {
        return Err("Failed to initialize game with stourney.com".to_owned())
    }
    println!("Game connected with stourney.com!");
    println!("You can view the ongoing game at: {}", url.unwrap());
    println!("The game is running in the background and all logs will go to ./{}", constants::DEFAULT_LOG_FILENAME);
    
    let outgoing_clone = outgoing_stream.clone();
    tokio::spawn( async move {
        maintain_heartbeat(outgoing_clone).await;
    });

    Ok((outgoing_stream, incoming_stream))
}
