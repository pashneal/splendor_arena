use crate::*;
use clap::Parser;
use log::trace;
use std::ops::Deref;
use tungstenite::{connect, stream::MaybeTlsStream, Message};
use url::Url;

pub type WebSocket = tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>;

pub struct Log {
    socket: WebSocket,
}

impl Log {
    pub fn new(url: &str, port: u16, client_id: u64) -> Self {
        let url = format!("{}:{}/log/{}", url, port, client_id);
        let url = Url::parse(&url).unwrap();
        let (socket, _) = connect(url).expect("Can't connect to the log server");
        Self { socket }
    }

    pub fn send(&mut self, message: &str) {
        let message = ClientMessage::Log(message.to_string());
        let message = serde_json::to_string(&message).expect("Error converting message to string");
        self.socket
            .send(Message::Text(message))
            .expect("Error writing message");
    }
}

pub trait Runnable<C: From<PrivateGameState>, A: Into<Action>> {
    fn initialize(&mut self, log: &mut Log);
    fn take_action(&mut self, info: C, log: &mut Log) -> A;
    fn game_over(&self, info: C, results: GameResults) {
        todo!()
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    /// The port to connect to
    #[arg(short, long)]
    port: u16,

    /// The game url to connect to
    #[arg(short, long)]
    url: Option<String>,

    /// The game id to connect to
    #[arg(short, long)]
    game_id: Option<u64>,

    /// The client id to connect as
    #[arg(short, long)]
    client_id: u64,
}

/// Public function to allow Python and Rust users
/// to have the same interface on the command line
pub fn get_args() -> Args {
    let mut args = Args::parse();
    if args.url.is_none() {
        args.url = Some("ws://127.0.0.1".to_string());
    }
    if args.game_id.is_none() {
        args.game_id = Some(0);
    }
    args
}

/// TODO: move to rust stubs
/// The protocol for communication and running the bot between the client and
/// the server. Sends logs and actions to the server when appropriate.
pub fn run_bot<C: From<PrivateGameState>, A: Into<Action>, B: Runnable<C, A> + Default>() {
    let args = get_args();
    let port = args.port;
    let base_url = args.url.unwrap();
    let game_id = args.game_id.unwrap();
    let client_id = args.client_id;

    trace!("Connecting to the game server...");
    trace!("Port: {}", port);
    trace!("Base URL: {}", base_url);
    trace!("Game ID: {}", game_id);
    trace!("Client ID: {}", client_id);

    let url = format!("{}:{}/game/{}/{}", base_url, port, game_id, client_id);
    trace!("Connecting to: {}", url);
    trace!("");
    let url = Url::parse(&url).unwrap();
    trace!("Url: {:?}", url);
    let (mut game_socket, _) = connect(url).expect("Can't connect to the game server");

    // Give the server a chance to start up
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut log = Log::new(&base_url, port, client_id);

    let mut bot = B::default();
    bot.initialize(&mut log);
    trace!("Connected to the game server...");

    loop {
        let msg = game_socket.read();
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                break;
            }
        };
        let msg = msg.to_text().expect("Error converting message to text");
        let message: ServerMessage = serde_json::from_str(msg).expect("Error parsing message");
        if let ServerMessage::PlayerActionRequest(info) = message {
            let info: C = C::from(info);
            let action = bot.take_action(info, &mut log);
            let action = action.into();
            let msg = ClientMessage::Action(action);

            let msg_str = serde_json::to_string(&msg).expect("Error converting action to string");
            let game_socket_result = game_socket.send(Message::Text(msg_str));
            if let Err(_) = game_socket_result {
                break;
            }
        } else if let ServerMessage::LobbyUpdate(LobbyUpdate::GameOver) = message {
            break;
        } else { 
            // TODO: handle game state updates
            // TODO: handle player update events
        }
    }
}
