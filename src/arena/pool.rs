use super::*;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};

/// A structure for running multiple games in parallel. Each game is run in an Arena
pub struct ArenaPool {
    port: u16,
    arenas: Arc<Mutex<HashMap<GameId, Arena>>>,
    clients: Arc<RwLock<HashMap<GameId, Vec<ClientId>>>>,
}

impl ArenaPool {
    pub fn new(port: u16) -> Self {
        ArenaPool {
            port,
            arenas: Arc::new(Mutex::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_arena(&mut self, num_players: usize, arena : Arena) -> (GameId, Vec<ClientId>) {
        let game_id = GameId::new();
        let mut clients = Vec::new();
        for _ in 0..num_players {
            let client_id = ClientId::new();
            clients.push(client_id);
        }
        self.clients.write().await.insert(game_id, clients.clone());
        self.arenas.lock().await.insert(game_id, arena);
        (game_id, clients)
    }

    pub async fn run(&self) {
        let port = self.port;

        // TODO: modify user actions so that it's updated after every action
        // but doesn't accept any messages from the user until it is their turn?
        // will need to consume differently on client side

        // perhaps split it into two messages
        //  Broadcast <- just a message that goes out to all clients indicating 
        //                the current game state after every action
        //
        // PlayerActionRequest <- a message that goes out to a specific player
        //                    indicating that it is their turn and they need to
        //                    send an action
    }
}
