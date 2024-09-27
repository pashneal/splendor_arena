use super::*;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use warp::ws::WebSocket;
use warp::Filter;
use log::{info, error, warn, debug};

type ArenaMap = HashMap<GameId, GlobalArena>;
type RwArenaMap = Arc<RwLock<ArenaMap>>;
type ClientsMap = HashMap<GameId, Clients>;
type RwClientsMap = Arc<RwLock<ClientsMap>>;

/// A structure for running multiple games in parallel. Each game is run in an Arena
pub struct ArenaPool {
    port: u16,
    arenas: RwArenaMap,
    clients: RwClientsMap,
}

impl ArenaPool {
    pub fn new(port: u16) -> Self {
        ArenaPool {
            port,
            arenas: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_arena(&mut self, num_players: usize, arena: Arena) -> (GameId, Vec<ClientId>) {
        let game_id = GameId::new();
        let client_ids = arena.allowed_clients();
        let arena = Arc::new(RwLock::new(arena));
        self.arenas.write().await.insert(game_id, arena);
        (game_id, client_ids)
    }

    async fn get_arena(&self, game_id: GameId) -> Option<GlobalArena> {
        self.arenas
            .read()
            .await
            .get(&game_id)
            .map(|arena| arena.clone())
    }

    async fn handle_upgrade(
        game_id: u64,
        client_id: u64,
        ws: WebSocket,
        arenas: RwArenaMap,
        clients: RwClientsMap,
    ) {
        let game_id = GameId(game_id);
        let client_id = ClientId(client_id);
        let arenas = arenas.read().await.get(&game_id).cloned();
        let clients = clients.read().await.get(&game_id).cloned();
        let web_stream = None;

        match (arenas, clients) {
            (Some(arena), Some(clients)) => {
                info!("User {} connected to game {}", client_id.0, game_id.0);
                user_connected(client_id, ws, clients, arena, web_stream);
            }
            (None, _) => {
                error!("Game {} does not exist, or is not ongoing", game_id.0);
            }
            (_, None) => {
                panic!("Clients map does not exist for game {}", game_id.0);
            }
            _ => {
            }
        }
    }

    pub async fn run(&self) {
        //TODO: return some handler that can be used to stop the server

        let arenas = self.arenas.clone();
        let clients = self.clients.clone();
        let arenas_filter = warp::any().map(move || arenas.clone());
        let clients_filter = warp::any().map(move || clients.clone());

        // GET /game/{game_id}/{client_id}
        let websocket = warp::path!("game" / u64 / u64)
            .and(warp::ws())
            .and(arenas_filter)
            .and(clients_filter)
            .map(
                |game_id: u64,
                 client_id: u64,
                 ws: warp::ws::Ws,
                 arenas: RwArenaMap,
                 clients: RwClientsMap| {
                    ws.on_upgrade(move |socket| {
                        ArenaPool::handle_upgrade(game_id, client_id, socket, arenas, clients)
                    })
                },
            );

        let routes = websocket;
        tokio::spawn(warp::serve(routes).run(([127, 0, 0, 1], self.port)));
    }

    async fn save_to_database() {
        todo!("feature comming soon");
    }

    async fn load_from_database() {
        todo!("feature comming soon");
    }
}
