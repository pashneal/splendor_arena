use crate::gem::Gem;
use crate::gems::Gems;
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cost {
    pub onyx: i8,
    pub sapphire: i8,
    pub emerald: i8,
    pub ruby: i8,
    pub diamond: i8,
}

impl Index<Gem> for Cost {
    type Output = i8;

    fn index<'a>(&'a self, color: Gem) -> &'a i8 {
        match color {
            Gem::Onyx => &self.onyx,
            Gem::Sapphire => &self.sapphire,
            Gem::Emerald => &self.emerald,
            Gem::Ruby => &self.ruby,
            Gem::Diamond => &self.diamond,
            _ => panic!("Invalid color in Cost object"),
        }
    }
}
impl IndexMut<Gem> for Cost {
    fn index_mut<'a>(&'a mut self, color: Gem) -> &'a mut i8 {
        match color {
            Gem::Onyx => &mut self.onyx,
            Gem::Sapphire => &mut self.sapphire,
            Gem::Emerald => &mut self.emerald,
            Gem::Ruby => &mut self.ruby,
            Gem::Diamond => &mut self.diamond,
            _ => panic!("Invalid color in Cost object"),
        }
    }
}

impl Cost {

    /// Create a new cost object that removes all gems
    /// passed in from the cost, if that would result in
    /// a negative cost, the cost is instead set to 0
    pub fn discounted_with(&self, gems: &Gems) -> Cost {
        Cost {
            onyx: 0.max(self.onyx - gems.onyx),
            sapphire: 0.max(self.sapphire - gems.sapphire),
            emerald: 0.max(self.emerald - gems.emerald),
            ruby: 0.max(self.ruby - gems.ruby),
            diamond: 0.max(self.diamond - gems.diamond),
        }
    }
    /// Convert to raw gems (granting ability to access gold)
    pub fn to_gems(&self) -> Gems {
        Gems {
            onyx: self.onyx,
            sapphire: self.sapphire,
            emerald: self.emerald,
            ruby: self.ruby,
            diamond: self.diamond,
            gold: 0,
        }
    }
    /// Convert from raw gems (removing ability to access gold)
    pub fn from_gems(gems: &Gems) -> Cost {
        debug_assert!(gems.gold == 0, "Cannot convert gems to cost with gold");
        Cost {
            onyx: gems.onyx,
            sapphire: gems.sapphire,
            emerald: gems.emerald,
            ruby: gems.ruby,
            diamond: gems.diamond,
        }
    }
}

pub type CardId = u8;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Card {
    points: u8,
    cost: Cost,
    gem: Gem,
    id: CardId,
    tier: u8,
}

impl Card {
    pub fn cost(&self) -> Cost {
        self.cost
    }

    pub fn points(&self) -> u8 {
        self.points
    }

    pub fn gem(&self) -> Gem {
        self.gem
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn tier(&self) -> u8 {
        self.tier
    }

    /// Create a static card array 
    /// which maps indices to Card objects
    /// Represents all cards in a game of Splendor
    pub const fn all_const() -> [Card; 90] {
        [
            Card {
                id: 0,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{diamond:1,sapphire:1,emerald:1,ruby:1,onyx:0}
            },
            Card {
                id: 1,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{diamond:1,sapphire:2,emerald:1,ruby:1,onyx:0}
            },
            Card {
                id: 2,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{diamond:2,sapphire:2,ruby:1,emerald:0,onyx:0}
            },
            Card {
                id: 3,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{emerald:1,ruby:3,onyx:1,sapphire:0,diamond:0}
            },
            Card {
                id: 4,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{emerald:2,ruby:1,sapphire:0,diamond:0,onyx:0}
            },
            Card {
                id: 5,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{diamond:2,emerald:2,sapphire:0,onyx:0,ruby:0}
            },
            Card {
                id: 6,
                tier: 1,
                gem: Gem::Onyx,
                points: 0,
                cost: Cost{emerald:3,sapphire:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 7,
                tier: 1,
                gem: Gem::Onyx,
                points: 1,
                cost: Cost{sapphire:4,emerald:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 8,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{diamond:1,emerald:1,ruby:1,onyx:1,sapphire:0}
            },
            Card {
                id: 9,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{diamond:1,emerald:1,ruby:2,onyx:1,sapphire:0}
            },
            Card {
                id: 10,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{diamond:1,emerald:2,ruby:2,sapphire:0,onyx:0}
            },
            Card {
                id: 11,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{sapphire:1,emerald:3,ruby:1,diamond:0,onyx:0}
            },
            Card {
                id: 12,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{diamond:1,onyx:2,sapphire:0,emerald:0,ruby:0}
            },
            Card {
                id: 13,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{emerald:2,onyx:2,sapphire:0,diamond:0,ruby:0}
            },
            Card {
                id: 14,
                tier: 1,
                gem: Gem::Sapphire,
                points: 0,
                cost: Cost{onyx:3,sapphire:0,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 15,
                tier: 1,
                gem: Gem::Sapphire,
                points: 1,
                cost: Cost{ruby:4,sapphire:0,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 16,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{sapphire:1,emerald:1,ruby:1,onyx:1,diamond:0}
            },
            Card {
                id: 17,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{sapphire:1,emerald:2,ruby:1,onyx:1,diamond:0}
            },
            Card {
                id: 18,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{sapphire:2,emerald:2,onyx:1,diamond:0,ruby:0}
            },
            Card {
                id: 19,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{diamond:3,sapphire:1,onyx:1,emerald:0,ruby:0}
            },
            Card {
                id: 20,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{ruby:2,onyx:1,sapphire:0,emerald:0,diamond:0}
            },
            Card {
                id: 21,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{sapphire:2,onyx:2,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 22,
                tier: 1,
                gem: Gem::Diamond,
                points: 0,
                cost: Cost{sapphire:3,emerald:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 23,
                tier: 1,
                gem: Gem::Diamond,
                points: 1,
                cost: Cost{emerald:4,sapphire:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 24,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{diamond:1,sapphire:1,ruby:1,onyx:1,emerald:0}
            },
            Card {
                id: 25,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{diamond:1,sapphire:1,ruby:1,onyx:2,emerald:0}
            },
            Card {
                id: 26,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{sapphire:1,ruby:2,onyx:2,emerald:0,diamond:0}
            },
            Card {
                id: 27,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{diamond:1,sapphire:3,emerald:1,onyx:0,ruby:0}
            },
            Card {
                id: 28,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{diamond:2,sapphire:1,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 29,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{sapphire:2,ruby:2,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 30,
                tier: 1,
                gem: Gem::Emerald,
                points: 0,
                cost: Cost{ruby:3,sapphire:0,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 31,
                tier: 1,
                gem: Gem::Emerald,
                points: 1,
                cost: Cost{onyx:4,sapphire:0,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 32,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:1,sapphire:1,emerald:1,onyx:1,ruby:0}
            },
            Card {
                id: 33,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:2,sapphire:1,emerald:1,onyx:1,ruby:0}
            },
            Card {
                id: 34,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:2,emerald:1,onyx:2,sapphire:0,ruby:0}
            },
            Card {
                id: 35,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:1,ruby:1,onyx:3,sapphire:0,emerald:0}
            },
            Card {
                id: 36,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{sapphire:2,emerald:1,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 37,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:2,ruby:2,sapphire:0,emerald:0,onyx:0}
            },
            Card {
                id: 38,
                tier: 1,
                gem: Gem::Ruby,
                points: 0,
                cost: Cost{diamond:3,sapphire:0,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 39,
                tier: 1,
                gem: Gem::Ruby,
                points: 1,
                cost: Cost{diamond:4,sapphire:0,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 40,
                tier: 2,
                gem: Gem::Onyx,
                points: 1,
                cost: Cost{diamond:3,sapphire:2,emerald:2,onyx:0,ruby:0}
            },
            Card {
                id: 41,
                tier: 2,
                gem: Gem::Onyx,
                points: 1,
                cost: Cost{diamond:3,emerald:3,onyx:2,sapphire:0,ruby:0}
            },
            Card {
                id: 42,
                tier: 2,
                gem: Gem::Onyx,
                points: 2,
                cost: Cost{sapphire:1,emerald:4,ruby:2,diamond:0,onyx:0}
            },
            Card {
                id: 43,
                tier: 2,
                gem: Gem::Onyx,
                points: 2,
                cost: Cost{emerald:5,ruby:3,sapphire:0,diamond:0,onyx:0}
            },
            Card {
                id: 44,
                tier: 2,
                gem: Gem::Onyx,
                points: 2,
                cost: Cost{diamond:5,sapphire:0,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 45,
                tier: 2,
                gem: Gem::Onyx,
                points: 3,
                cost: Cost{onyx:6,sapphire:0,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 46,
                tier: 2,
                gem: Gem::Sapphire,
                points: 1,
                cost: Cost{sapphire:2,emerald:2,ruby:3,diamond:0,onyx:0}
            },
            Card {
                id: 47,
                tier: 2,
                gem: Gem::Sapphire,
                points: 1,
                cost: Cost{sapphire:2,emerald:3,onyx:3,diamond:0,ruby:0}
            },
            Card {
                id: 48,
                tier: 2,
                gem: Gem::Sapphire,
                points: 2,
                cost: Cost{diamond:5,sapphire:3,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 49,
                tier: 2,
                gem: Gem::Sapphire,
                points: 2,
                cost: Cost{diamond:2,ruby:1,onyx:4,sapphire:0,emerald:0}
            },
            Card {
                id: 50,
                tier: 2,
                gem: Gem::Sapphire,
                points: 2,
                cost: Cost{sapphire:5,emerald:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 51,
                tier: 2,
                gem: Gem::Sapphire,
                points: 3,
                cost: Cost{sapphire:6,emerald:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 52,
                tier: 2,
                gem: Gem::Diamond,
                points: 1,
                cost: Cost{emerald:3,ruby:2,onyx:2,sapphire:0,diamond:0}
            },
            Card {
                id: 53,
                tier: 2,
                gem: Gem::Diamond,
                points: 1,
                cost: Cost{diamond:2,sapphire:3,ruby:3,emerald:0,onyx:0}
            },
            Card {
                id: 54,
                tier: 2,
                gem: Gem::Diamond,
                points: 2,
                cost: Cost{emerald:1,ruby:4,onyx:2,sapphire:0,diamond:0}
            },
            Card {
                id: 55,
                tier: 2,
                gem: Gem::Diamond,
                points: 2,
                cost: Cost{ruby:5,onyx:3,sapphire:0,emerald:0,diamond:0}
            },
            Card {
                id: 56,
                tier: 2,
                gem: Gem::Diamond,
                points: 2,
                cost: Cost{ruby:5,sapphire:0,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 57,
                tier: 2,
                gem: Gem::Diamond,
                points: 3,
                cost: Cost{diamond:6,sapphire:0,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 58,
                tier: 2,
                gem: Gem::Emerald,
                points: 1,
                cost: Cost{diamond:3,emerald:2,ruby:3,sapphire:0,onyx:0}
            },
            Card {
                id: 59,
                tier: 2,
                gem: Gem::Emerald,
                points: 1,
                cost: Cost{diamond:2,sapphire:3,onyx:2,emerald:0,ruby:0}
            },
            Card {
                id: 60,
                tier: 2,
                gem: Gem::Emerald,
                points: 2,
                cost: Cost{diamond:4,sapphire:2,onyx:1,emerald:0,ruby:0}
            },
            Card {
                id: 61,
                tier: 2,
                gem: Gem::Emerald,
                points: 2,
                cost: Cost{sapphire:5,emerald:3,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 62,
                tier: 2,
                gem: Gem::Emerald,
                points: 2,
                cost: Cost{emerald:5,sapphire:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 63,
                tier: 2,
                gem: Gem::Emerald,
                points: 3,
                cost: Cost{emerald:6,sapphire:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 64,
                tier: 2,
                gem: Gem::Ruby,
                points: 1,
                cost: Cost{diamond:2,ruby:2,onyx:3,sapphire:0,emerald:0}
            },
            Card {
                id: 65,
                tier: 2,
                gem: Gem::Ruby,
                points: 1,
                cost: Cost{sapphire:3,ruby:2,onyx:3,emerald:0,diamond:0}
            },
            Card {
                id: 66,
                tier: 2,
                gem: Gem::Ruby,
                points: 2,
                cost: Cost{diamond:1,sapphire:4,emerald:2,onyx:0,ruby:0}
            },
            Card {
                id: 67,
                tier: 2,
                gem: Gem::Ruby,
                points: 2,
                cost: Cost{diamond:3,onyx:5,sapphire:0,emerald:0,ruby:0}
            },
            Card {
                id: 68,
                tier: 2,
                gem: Gem::Ruby,
                points: 2,
                cost: Cost{onyx:5,sapphire:0,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 69,
                tier: 2,
                gem: Gem::Ruby,
                points: 3,
                cost: Cost{ruby:6,sapphire:0,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 70,
                tier: 3,
                gem: Gem::Onyx,
                points: 3,
                cost: Cost{diamond:3,sapphire:3,emerald:5,ruby:3,onyx:0}
            },
            Card {
                id: 71,
                tier: 3,
                gem: Gem::Onyx,
                points: 4,
                cost: Cost{ruby:7,sapphire:0,emerald:0,diamond:0,onyx:0}
            },
            Card {
                id: 72,
                tier: 3,
                gem: Gem::Onyx,
                points: 4,
                cost: Cost{emerald:3,ruby:6,onyx:3,sapphire:0,diamond:0}
            },
            Card {
                id: 73,
                tier: 3,
                gem: Gem::Onyx,
                points: 5,
                cost: Cost{ruby:7,onyx:3,sapphire:0,emerald:0,diamond:0}
            },
            Card {
                id: 74,
                tier: 3,
                gem: Gem::Sapphire,
                points: 3,
                cost: Cost{diamond:3,emerald:3,ruby:3,onyx:5,sapphire:0}
            },
            Card {
                id: 75,
                tier: 3,
                gem: Gem::Sapphire,
                points: 4,
                cost: Cost{diamond:7,sapphire:0,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 76,
                tier: 3,
                gem: Gem::Sapphire,
                points: 4,
                cost: Cost{diamond:6,sapphire:3,onyx:3,emerald:0,ruby:0}
            },
            Card {
                id: 77,
                tier: 3,
                gem: Gem::Sapphire,
                points: 5,
                cost: Cost{diamond:7,sapphire:3,emerald:0,onyx:0,ruby:0}
            },
            Card {
                id: 78,
                tier: 3,
                gem: Gem::Diamond,
                points: 3,
                cost: Cost{sapphire:3,emerald:3,ruby:5,onyx:3,diamond:0}
            },
            Card {
                id: 79,
                tier: 3,
                gem: Gem::Diamond,
                points: 4,
                cost: Cost{onyx:7,sapphire:0,emerald:0,diamond:0,ruby:0}
            },
            Card {
                id: 80,
                tier: 3,
                gem: Gem::Diamond,
                points: 4,
                cost: Cost{diamond:3,ruby:3,onyx:6,sapphire:0,emerald:0}
            },
            Card {
                id: 81,
                tier: 3,
                gem: Gem::Diamond,
                points: 5,
                cost: Cost{diamond:3,onyx:7,sapphire:0,emerald:0,ruby:0}
            },
            Card {
                id: 82,
                tier: 3,
                gem: Gem::Emerald,
                points: 3,
                cost: Cost{diamond:5,sapphire:3,ruby:3,onyx:3,emerald:0}
            },
            Card {
                id: 83,
                tier: 3,
                gem: Gem::Emerald,
                points: 4,
                cost: Cost{sapphire:7,emerald:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 84,
                tier: 3,
                gem: Gem::Emerald,
                points: 4,
                cost: Cost{diamond:3,sapphire:6,emerald:3,onyx:0,ruby:0}
            },
            Card {
                id: 85,
                tier: 3,
                gem: Gem::Emerald,
                points: 5,
                cost: Cost{sapphire:7,emerald:3,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 86,
                tier: 3,
                gem: Gem::Ruby,
                points: 3,
                cost: Cost{diamond:3,sapphire:5,emerald:3,onyx:3,ruby:0}
            },
            Card {
                id: 87,
                tier: 3,
                gem: Gem::Ruby,
                points: 4,
                cost: Cost{emerald:7,sapphire:0,diamond:0,onyx:0,ruby:0}
            },
            Card {
                id: 88,
                tier: 3,
                gem: Gem::Ruby,
                points: 4,
                cost: Cost{sapphire:3,emerald:6,ruby:3,diamond:0,onyx:0}
            },
            Card {
                id: 89,
                tier: 3,
                gem: Gem::Ruby,
                points: 5,
                cost: Cost{emerald:7,ruby:3,sapphire:0,diamond:0,onyx:0}
            },
        ]
    }

    pub fn all() -> Vec<Card> {
        Card::all_const().to_vec()
    }
}
