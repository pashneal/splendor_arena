use super::*;
use crate::card::CardId;
use crate::gem::Gem;
use crate::nobles::Noble;
use crate::gems::Gems;
use log::trace;
use std::collections::HashMap;
use std::marker::PhantomData;
use warp::{Filter, Rejection, Reply};

// Note: the following code results from me playing around with
//
// 1) a type stating system to have illegal state be unrepresentable
// 2) an sum type to represent either of two state in the same field
//
// It is not exactly the cleanest code, but I wanted
// to play around with these ergonomics

#[derive(Debug, Clone)]
pub enum Either<A, B> {
    Initialized(A),
    Finalized(B),
}

pub trait ReplayState {}
impl ReplayState for Initialized {}
impl ReplayState for Finalized {}

#[derive(Debug, Clone)]
pub struct Initialized {
    initial_game: Game,
}
#[derive(Debug, Clone)]
pub struct Finalized {
    initial_game: Game,
    viewable_game: Game,
    history: GameHistory,
    move_index: usize,
}

#[derive(Debug, Clone)]
pub struct Replay<T: ReplayState> {
    inner: T,
}

impl Replay<Initialized> {
    pub fn new(game: Game) -> Replay<Initialized> {
        Replay {
            inner: Initialized { initial_game: game },
        }
    }

    pub fn finalize_with(self, history: GameHistory) -> Replay<Finalized> {
        Replay {
            inner: Finalized {
                initial_game: self.inner.initial_game.clone(),
                viewable_game: self.inner.initial_game.clone(),
                history,
                move_index: 0,
            },
        }
    }
}

impl Replay<Finalized> {
    pub fn next_move(&mut self) {
        self.go_to_move(self.inner.move_index as i32 + 1)
    }

    pub fn previous_move(&mut self) {
        self.go_to_move(self.inner.move_index as i32 - 1)
    }

    pub fn go_to_move(&mut self, new_move_index: i32) {
        // Bound between 0 and the number of moves no matter the input
        let new_move_index = new_move_index.max(0);
        let new_move_index = new_move_index.min(self.inner.history.num_moves());

        self.inner.move_index = new_move_index as usize;

        // Replay the game up to the given number
        let history = self.inner.history.take_until_move(new_move_index);
        trace!("Replaying history : {:?}", history);
        let mut init_game = self.inner.initial_game.clone();
        init_game.advance_history_with(history);

        self.inner.viewable_game = init_game;
    }

    pub fn current_game(&self) -> &Game {
        &self.inner.viewable_game
    }
}

pub type FinalizedReplay = Arc<RwLock<Replay<Finalized>>>;

// (color/gem, amount)
type JSTokens = Vec<(usize, i8)>;

#[derive(Debug, Clone, Serialize)]
pub struct JSCard {
    tier: usize,
    points: usize,
    #[serde(rename = "colorIndex")]
    color_index: usize,
    tokens: JSTokens,
}

#[derive(Debug, Clone, Serialize)]
pub struct JSDeck {
    #[serde(rename = "cardCount")]
    card_count: usize,
    #[serde(rename = "tier")]
    tier: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct JSPlayer {
    developments: JSTokens,
    gems : JSTokens,
    #[serde(rename = "totalGems")]
    total_gems: u32,
    #[serde(rename = "reservedCards")]
    reserved_cards: Vec<JSCard>,
    #[serde(rename = "totalPoints")]
    total_points: u8,
    #[serde(rename = "noblePoints")]
    noble_points: u8,
}

#[derive(Debug, Serialize)]
enum Success {
    #[serde(rename = "move_index")]
    Move(usize),
    #[serde(rename = "nobles")]
    Nobles(Vec<JSTokens>),
    #[serde(rename = "cards")]
    Cards(Vec<Vec<JSCard>>),
    #[serde(rename = "decks")]
    Decks(Vec<JSDeck>),
    #[serde(rename = "bank")]
    Bank(JSTokens),
    #[serde(rename = "players")]
    Players(Vec<JSPlayer>),
}

#[derive(Debug, Serialize)]
enum EndpointReply {
    #[serde(rename = "success")]
    Success(Success),
    #[serde(rename = "error")]
    Error(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Move {
    pub move_index: i32,
}

pub fn json_body() -> impl Filter<Extract = (Move,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

pub async fn next_move(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            replay.write().await.next_move();
            let move_index = replay.read().await.inner.move_index;
            Ok(warp::reply::json(&EndpointReply::Success(Success::Move(
                move_index,
            ))))
        }
    }
}

pub async fn previous_move(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            replay.write().await.previous_move();
            let move_index = replay.read().await.inner.move_index;
            Ok(warp::reply::json(&EndpointReply::Success(Success::Move(
                move_index,
            ))))
        }
    }
}

pub async fn go_to_move(move_number: Move, arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            let move_number = move_number.move_index;
            replay.write().await.go_to_move(move_number);
            let move_index = replay.read().await.inner.move_index;
            Ok(warp::reply::json(&EndpointReply::Success(Success::Move(
                move_index,
            ))))
        }
    }
}

// Match the conventions of the frontend gems
//
//          color    : index
//	 white (diamond) : 0
//	 blue (sapphire) : 1
//	 green (emerald) : 2
//	 red (ruby)      : 3
//	 black (onyx)    : 4
//	 yellow (gold)   : 5
fn js_gems_map() -> HashMap<Gem, usize> {
    let mut map = HashMap::new();
    map.insert(Gem::Diamond, 0);
    map.insert(Gem::Sapphire, 1);
    map.insert(Gem::Emerald, 2);
    map.insert(Gem::Ruby, 3);
    map.insert(Gem::Onyx, 4);
    map.insert(Gem::Gold, 5);
    map
}

// Converts a noble to a vector representing the color distribution
// of the cost of the noble as a list of (color_index, number_needed)
fn to_js_noble(noble: &Noble) -> JSTokens {
    let mut map = js_gems_map();
    let mut js_noble = Vec::new();

    let gems = noble.requirements();

    for gem in Gem::all_expect_gold() {
        let index = map.get(&gem).unwrap();
        let count = gems[gem];
        if count > 0 {
            js_noble.push((*index, count));
        }
    }

    js_noble
}

pub async fn board_nobles(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            trace!("Getting nobles");
            let game = &replay.read().await.inner.viewable_game;
            let nobles = game.nobles();
            trace!("Got nobles : {:#?}", nobles);
            let js_nobles = nobles.iter().map(|n| to_js_noble(&n)).collect();
            Ok(warp::reply::json(&EndpointReply::Success(Success::Nobles(
                js_nobles,
            ))))
        }
    }
}

// Converts a list of card ids to a list of JSCards
// using the conventions laid out in the frontend
fn to_js_cards(card_ids: Vec<Vec<CardId>>, card_lookup: Arc<Vec<Card>>) -> Vec<Vec<JSCard>> {
    let cards = card_ids
        .iter()
        .flatten()
        .map(|&c| card_lookup[c as usize].clone())
        .collect::<Vec<Card>>();
    let map = js_gems_map();
    let js_cards: Vec<JSCard> = cards
        .iter()
        .map(|c| {
            let tier = (c.tier() - 1) as usize;
            let points = c.points() as usize;
            let cost = c.cost();
            let mut js_cost = Vec::new();

            for gem in Gem::all_expect_gold() {
                let index = map.get(&gem).unwrap();
                let count = cost[gem];
                if count > 0 {
                    js_cost.push((*index, count));
                }
            }

            let color_index = map.get(&c.gem()).unwrap();

            JSCard {
                tier,
                points,
                color_index: *color_index,
                tokens: js_cost,
            }
        })
        .collect();

    // Group by tier
    let mut grouped = vec![Vec::new(); 3];
    for card in js_cards {
        grouped[card.tier].push(card);
    }

    grouped
}

pub async fn board_cards(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            let card_lookup = replay.read().await.inner.viewable_game.card_lookup();
            let cards = replay.read().await.inner.viewable_game.cards();
            let js_cards = to_js_cards(cards, card_lookup);
            Ok(warp::reply::json(&EndpointReply::Success(Success::Cards(
                js_cards,
            ))))
        }
    }
}

// Converts a list of card counts to a list of JSDeck
// using the conventions laid out in the frontend
pub fn to_js_decks(deck_counts: [usize; 3]) -> Vec<JSDeck> {
    let mut decks = Vec::new();
    for (i, &count) in deck_counts.iter().enumerate() {
        let tier = i;
        let card_count = count;
        decks.push(JSDeck { card_count, tier });
    }
    decks
}

pub async fn board_decks(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            let deck_counts = replay.read().await.inner.viewable_game.deck_counts();
            let js_decks = to_js_decks(deck_counts);
            Ok(warp::reply::json(&EndpointReply::Success(Success::Decks(
                js_decks,
            ))))
        }
    }
}

// Converts a list of gems from the public board area to a list of JSGems
// using the conventions laid out in the frontend
pub fn to_js_bank(gems: &Gems) -> JSTokens {
    let map = js_gems_map();
    let mut js_bank = Vec::new();
    for gem in Gem::all() {
        let index = map.get(&gem).unwrap();
        let count = gems[gem];
        if count > 0 {
            js_bank.push((*index, count));
        }
    }
    js_bank
}

pub async fn board_bank(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            let bank = to_js_bank(replay.read().await.inner.viewable_game.bank());
            Ok(warp::reply::json(&EndpointReply::Success(Success::Bank(
                bank,
            ))))
        }
    }
}

//  Converts metadata about the players to a list of JSPlayer
//  using the conventions laid out in the frontend
pub fn to_js_players(players: &Vec<Player>, card_lookup: Arc<Vec<Card>>) -> Vec<JSPlayer> {
    let mut js_players = Vec::new();
    for player in players {
        let developments = player.developments();
        let map = js_gems_map();

        let mut js_developments = Vec::new();
        let mut js_gems = Vec::new();
        let mut js_cards =  Vec::new();


        for gem in Gem::all_expect_gold() {
            let index = map.get(&gem).unwrap();
            let count = developments[gem];
            js_developments.push((*index, count));

        }

        for gem in Gem::all() {
            let index = map.get(&gem).unwrap();
            let count = player.gems()[gem];
            js_gems.push((*index, count));
        }

        for card_id in player.all_reserved() {
            let card = card_lookup[card_id as usize].clone();
            let tier = (card.tier() - 1) as usize;
            let points = card.points() as usize;
            let cost = card.cost();
            let mut js_cost = Vec::new();

            for gem in Gem::all_expect_gold() {
                let index = map.get(&gem).unwrap();
                let count = cost[gem];
                if count > 0 {
                    js_cost.push((*index, count));
                }
            }

            let color_index = map.get(&card.gem()).unwrap();
            let color_index = *color_index;

            js_cards.push(JSCard {
                tier,
                points,
                color_index,
                tokens: js_cost,
            });
        }

        let total_gems = player.gems().total();
        let total_points = player.total_points();
        let noble_points = player.noble_points();

        js_players.push(JSPlayer { 
            developments : js_developments,
            gems : js_gems,
            reserved_cards : js_cards, 
            total_gems,
            total_points,
            noble_points,
        });
    }
    js_players
}

pub async fn board_players(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let replay = arena.write().await.get_replay();
    match replay {
        None => Ok(warp::reply::json(&EndpointReply::Error(
            "No replay available".to_string(),
        ))),
        Some(replay) => {
            let card_lookup = replay.read().await.inner.viewable_game.card_lookup();
            let players = to_js_players(replay.read().await.inner.viewable_game.players()
                                        , card_lookup);
            Ok(warp::reply::json(&EndpointReply::Success(Success::Players(
                players,
            ))))
        }
    }
}
