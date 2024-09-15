use serde::{Deserialize, Serialize};
use super::*;

/// Updates the game state with the newest client info 
/// and the number of the updates starting from 0 and incrementing by 1 for each
/// sequential change in the game state
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameUpdate{
    pub info: ClientInfo,
    pub update_num: usize
}

/// Represents the information requests that the client can send to
/// the global stourney server to visualize a running or completed splendor game
#[derive(Serialize, Deserialize, Debug)]
pub enum ArenaRequest {
    /// Authenticate the arena 
    Authenticate{ secret: String },

    /// Reconnect current game with a given id to the global server,
    /// so updates can be resumed
    Reconnect{ id : String },

    /// Request the global server to initialize the game state
    InitializeGame{ info: ClientInfo },

    /// Request the global server to update the game state
    GameUpdates(Vec<GameUpdate>),

    /// Announce to server that the game is over,
    /// therefore indicating that the last successful update was the final update
    GameOver{ total_updates : usize },

    /// Reports a debug message to the global server 
    DebugMessage(String)
}

/// A response from the global stourney server to a client request
/// concerning authentication of the arena
#[derive(Serialize, Deserialize)]
pub enum Authenticated {
    Success,
    Failure{ reason: String }
}

/// A game state update response from the server
#[derive(Serialize, Deserialize)]
pub enum Updated {
    /// Indicates that the server has updated the game state, and returns
    /// the number of successful updates that have been processed since the initialization
    Success{ num_lifetime_updates: usize },

    /// Indicates that the server was unable to update the game state,
    /// and returns the number of previously successfully processed updates
    Failure{ reason: String, num_lifetime_updates: usize },

    /// Acknowledges that the game is over
    GameOverAck
}


/// A response from the global stourney server to a client request
/// concerning initialization of game state
#[derive(Serialize, Deserialize)]
pub enum Initialized {
    Success{ id : String },
    Failure{ reason: String }
}

/// A response from the global stourney server concerning whether
/// a client was able to reconnect to a game
#[derive(Serialize, Deserialize)]
pub enum Reconnected {
    Success,
    Failure{ reason: String }
}


/// Represents the information that the global stourney server
/// can send in response to a client request or as a broadcast
#[derive(Serialize, Deserialize)]
pub enum GlobalServerResponse {
    Authenticated(Authenticated),
    Updated(Updated),
    Initialized(Initialized),
    Reconnected(Reconnected),
    Warning(String),
    Error(String),
    Info(String),
}

