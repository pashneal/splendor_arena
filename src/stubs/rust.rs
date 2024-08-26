#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

/// This module repackages the splendor_tourney module into a
/// more convenient form
///
/// While users can certainly use the splendor_tourney module directly, it is
/// discouraged. The splendor_tourney module is designed API-first, and is not
/// designed to be user-friendly.
///
/// This module is also an attempt to have a consistent feel
/// for the user interface across all supported languages.
///
/// Changing this may break compatibility with the engine!

use derive_more::{Display, Error};
use std::time::Duration;
use serde::Deserialize;

pub use crate::{
    run_bot, CardId, Cost, GameResults, Gem, Gems, Log, Noble, NobleId, Runnable,
};

pub use crate as splendor_tourney;

pub const CARD_LOOKUP: [splendor_tourney::Card; 90] = splendor_tourney::Card::all_const();

pub type Tier = usize;

#[derive(Debug, Display, Error)]
pub enum ModelError {
    #[display(fmt = "Unable to convert from Action to splendor_tourney::Action")]
    IllegalAction,
}

/// Re-export the splendor_tourney module Action
/// into one that has a more user-friendly interface
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Take gem tokens from the bank
    TakeGems(Gems),
    /// Reserve a card from the board (face up)
    ReserveFaceUp(CardId),
    /// Reserve a card from the board from tier 0 - 2 (face down)
    ReserveFaceDown(Tier),
    /// Purchase a card from the board (face up) or from reserved cards
    Purchase(CardId, Gems),
    /// Discard gems from your hand if > 10
    DiscardGems(Gems),
    /// Attract an available noble from the board
    AttractNoble(Noble),
    /// Pass your turn (no action available)
    Pass,
    /// Continue play to the next player
    Continue,
}

impl Action {
    /// Convert a splendor_tourney::Action into an Action
    fn from(action: splendor_tourney::Action) -> Self {
        match action {
            splendor_tourney::Action::TakeDouble(gem) => Action::TakeGems(Gems::one(gem) + Gems::one(gem)),
            splendor_tourney::Action::TakeDistinct(gems) => Action::TakeGems(Gems::from_set(&gems)),
            splendor_tourney::Action::Reserve(card_id) => Action::ReserveFaceUp(card_id),
            splendor_tourney::Action::ReserveHidden(tier) => Action::ReserveFaceDown(tier),
            splendor_tourney::Action::Purchase((card_id, gems)) => Action::Purchase(card_id, gems),
            splendor_tourney::Action::Discard(gems) => Action::DiscardGems(gems),
            splendor_tourney::Action::AttractNoble(noble_id) => Action::AttractNoble(Noble::from_id(noble_id)),
            splendor_tourney::Action::Pass => Action::Pass,
            splendor_tourney::Action::Continue => Action::Continue,
        }
    }

    /// Convert an Action into a splendor_tourney::Action
    fn to_splendor_tourney(&self) -> Result<splendor_tourney::Action, ModelError> {
        match self {
            Action::TakeGems(gems) => {
                // TODO: could validate gems here
                let is_double = Gems::all().iter().any(|&gem| gems[gem] == 2);
                if is_double {
                    if gems.total() != 2 {
                        return Err(ModelError::IllegalAction);
                    }
                    let doubled_gem = Gems::all().into_iter().find(|&gem| gems[gem] == 2).unwrap();
                    Ok(splendor_tourney::Action::TakeDouble(doubled_gem))
                } else {
                    if gems.total() > 3 {
                        return Err(ModelError::IllegalAction);
                    }
                    let set = gems.to_set();
                    Ok(splendor_tourney::Action::TakeDistinct(set))
                }
            }
            Action::ReserveFaceUp(card_id) => {
                let reserve = splendor_tourney::Action::Reserve(*card_id);
                Ok(reserve)
            }
            Action::ReserveFaceDown(tier) => {
                let reserve_hidden = splendor_tourney::Action::ReserveHidden(*tier);
                Ok(reserve_hidden)
            }
            Action::Purchase(card_id, gems) => {
                let purchase = splendor_tourney::Action::Purchase((*card_id, *gems));
                Ok(purchase)
            }
            Action::DiscardGems(gems) => {
                let discard = splendor_tourney::Action::Discard(*gems);
                Ok(discard)
            }
            Action::AttractNoble(noble) => {
                let attract_noble = splendor_tourney::Action::AttractNoble(noble.id);
                Ok(attract_noble)
            }
            Action::Pass => {
                let pass = splendor_tourney::Action::Pass;
                Ok(pass)
            }
            Action::Continue => {
                let continue_action = splendor_tourney::Action::Continue;
                Ok(continue_action)
            }
        }
    }
}

impl Into<splendor_tourney::Action> for Action {
    fn into(self) -> splendor_tourney::Action {
        self.to_splendor_tourney().expect("Invalid action")
    }
}

/// Re-export the splendor_tourney module Board
/// into one that has a more user-friendly interface
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub deck_counts: [usize; 3],
    pub nobles: Vec<Noble>,
    pub gems: Gems,
    available_cards: Vec<Vec<CardId>>,
}

impl Board {
    /// Convert a splendor_tourney::Board into a Board
    fn from(board: splendor_tourney::Board) -> Self {
        let mut nobles = Vec::new();
        let all_nobles = Noble::all();
        for &noble_id in &board.nobles {
            let noble = all_nobles[noble_id as usize].clone();
            nobles.push(noble);
        }
        Board {
            deck_counts: board.deck_counts,
            nobles,
            gems: board.gems,
            available_cards: board.available_cards,
        }
    }

    /// Return all face up cards on the board
    /// in no particular order
    pub fn all_face_up_cards(&self) -> Vec<Card> {
        let mut cards = Vec::new();
        self.available_cards.iter().flatten().for_each(|id| {
            let card = Card::from_id(*id);
            cards.push(card);
        });
        cards
    }

    /// Return all face up cards on the board
    /// in a given tier in no particular order
    pub fn face_up_cards(&self, tier: usize) -> Vec<Card> {
        let mut cards = Vec::new();
        self.available_cards[tier].iter().for_each(|id| {
            let card = Card::from_id(*id);
            cards.push(card);
        });
        cards
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub points: u8,
    pub cost: Cost,
    pub gem: Gem,
    pub id: CardId,
    pub tier: u8,
}

impl Card {
    /// Convert a splendor_tourney::Card into a Card
    fn from(card: splendor_tourney::Card) -> Self {
        Card {
            points: card.points(),
            cost: card.cost(),
            gem: card.gem(),
            id: card.id(),
            tier: card.tier(),
        }
    }

    /// Given a CardId, return the corresponding Card
    pub fn from_id(id: CardId) -> Self {
        let card = CARD_LOOKUP[id as usize];
        Card {
            points: card.points(),
            cost: card.cost(),
            gem: card.gem(),
            id: card.id(),
            tier: card.tier(),
        }
    }

    /// Return all cards in the game
    pub fn all() -> [Card; 90] {
        CARD_LOOKUP
            .iter()
            .map(|&card| Card::from(card))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

/// Re-export the splendor_tourney module GameHistory
/// into one that has a more user-friendly interface
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHistory {
    pub turns: Vec<(usize, Vec<Action>)>,
}

impl GameHistory {
    fn from(game_history: splendor_tourney::GameHistory) -> Self {
        let mut turns = Vec::new();
        for group in game_history.group_by_player() {
            let mut actions = Vec::new();
            let mut player_index = 5;
            for (p, action) in group {
                player_index = p;
                actions.push(Action::from(action));
            }
            turns.push((player_index, actions));
        }

        GameHistory { turns }
    }
}

/// Re-export the splendor_tourney module Player
/// into one that has a more user-friendly interface
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub index: usize,
    pub total_points: u8,
    pub num_reserved_cards: usize,
    pub gems: Gems,
    pub developments: Gems,
    pub reserved_cards: Option<Vec<Card>>,
}

impl Player {
    pub fn from(player: &splendor_tourney::Player, index: usize) -> Self {
        Player {
            index,
            total_points: player.total_points(),
            reserved_cards: Some(
                player
                    .all_reserved()
                    .into_iter()
                    .map(Card::from_id)
                    .collect(),
            ),
            num_reserved_cards: player.num_reserved_cards(),
            gems: Gems::from(*player.gems()),
            developments: Gems::from(*player.developments()),
        }
    }

    pub fn from_public(player: &splendor_tourney::PlayerPublicInfo, index: usize) -> Self {
        Player {
            index,
            total_points: player.points,
            reserved_cards: None,
            num_reserved_cards: player.num_reserved,
            gems: Gems::from(player.gems),
            developments: Gems::from(player.developments.to_gems()),
        }
    }
}

/// Re-export the splendor_tourney module ClientInfo
/// into one that has a more user-friendly interface
#[derive(Debug, Clone)]
pub struct GameInfo {
    pub board: Board,
    pub history: GameHistory,
    pub players: Vec<Player>,
    pub current_player: Player,
    pub player_index: usize,
    pub legal_actions: Vec<Action>,
    pub num_players: usize,
    time_endpoint_url: String,
}

impl From<splendor_tourney::ClientInfo> for GameInfo {
    fn from(client_info: splendor_tourney::ClientInfo) -> Self {
        GameInfo::from_splendor_tourney(client_info)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeResponse {
    pub time_remaining: Duration,
}

impl GameInfo {
    pub fn from_splendor_tourney(client_info: splendor_tourney::ClientInfo) -> Self {
        let legal_actions = client_info.legal_actions;
        let legal_actions = legal_actions.into_iter().map(Action::from).collect();
        let current_player =
            Player::from(&client_info.current_player, client_info.current_player_num);
        let board = Board::from(client_info.board);
        let game_history = GameHistory::from(client_info.history);
        let mut players: Vec<Player> = client_info
            .players
            .iter()
            .enumerate()
            .map(|(index, player)| Player::from_public(player, index))
            .collect();

        players[current_player.index] = current_player.clone();
        let num_players = players.len();

        GameInfo {
            board,
            history: game_history,
            players,
            current_player,
            player_index: client_info.current_player_num,
            legal_actions,
            num_players,
            time_endpoint_url: client_info.time_endpoint_url,
        }
    }

    pub fn me(&self) -> &Player {
        &self.players[self.player_index]
    }

    pub fn num_players(&self) -> usize {
        self.num_players
    }

    pub fn time_remaining(&self) -> Duration {
        let url = &self.time_endpoint_url;
        let response = reqwest::blocking::get(url).expect("Could not contact game server");
        let response: TimeResponse = response.json().expect("Could not parse time response");
        response.time_remaining
    }
}

