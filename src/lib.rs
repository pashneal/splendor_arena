#![allow(unused)]

pub mod arena;
pub mod card;
pub mod client;
pub mod game_logic;
pub mod gem;
pub mod nobles;
pub mod player;
pub mod gems;

pub use crate::arena::*;
pub use crate::card::*;
pub use crate::client::*;
pub use crate::game_logic::*;
pub use crate::gem::*;
pub use crate::nobles::*;
pub use crate::player::*;
pub use crate::protocol::*;
pub use crate::gems::*;

pub trait JSONable: serde::Serialize + serde::de::DeserializeOwned {
    fn from_json(json: &str) -> Self {
        serde_json::from_str(json).expect("Should be able to deserialize")
    }
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Should be able to serialize")
    }
}
