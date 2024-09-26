use super::*;
use std::collections::HashMap;

/// A structure for running multiple games in parallel. Each game is run in an Arena
pub struct ArenaPool {
    port: u16,
    arenas: HashMap<GameId, Arena>,
    clients: HashMap<GameId, Vec<ClientId>>,
}

impl ArenaPool {
    pub fn new(port: u16) -> Self {
        ArenaPool {
            port,
            arenas: HashMap::new(),
            clients: HashMap::new(),
        }
    }
}
