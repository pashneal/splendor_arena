use serde::{Deserialize, Serialize};
#[derive(PartialEq, Copy, Clone, Debug, Eq, Hash, Serialize, Deserialize)]
pub enum Gem {
    Onyx,
    Sapphire,
    Emerald,
    Ruby,
    Diamond,
    Gold,
}

impl Gem {
    pub fn all_expect_gold() -> Vec<Gem> {
        vec![
            Gem::Onyx,
            Gem::Sapphire,
            Gem::Emerald,
            Gem::Ruby,
            Gem::Diamond,
        ]
    }
    pub fn all() -> Vec<Gem> {
        vec![
            Gem::Onyx,
            Gem::Sapphire,
            Gem::Emerald,
            Gem::Ruby,
            Gem::Diamond,
            Gem::Gold,
        ]
    }
}
