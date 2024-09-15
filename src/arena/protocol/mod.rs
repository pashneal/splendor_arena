/// Helper functions local game server that interacts with the game logic, validates moves
/// from the clients, and send the game state back to the clients after each move
pub mod local;
/// Helper functions for connecting to a Stourney web server which  
/// is running on a different machine from the client. Sends
/// game state updates to the web server, so that the game
/// can be viewed in a web browser from anywhere on the web.
pub mod web;

pub use local::*;

pub use web::*;
pub use super::*;
