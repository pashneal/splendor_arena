use crate::gem::Gem;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::collections::HashSet;
use std::ops::{Add, AddAssign, Index, IndexMut, Sub, SubAssign};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Gems {
    pub onyx: i8,
    pub sapphire: i8,
    pub emerald: i8,
    pub ruby: i8,
    pub diamond: i8,
    pub gold: i8,
}

impl Gems {

    pub fn all() -> Vec<Gem> {
        Gem::all()
    }

    pub fn all_expect_gold() -> Vec<Gem> {
        Gem::all_expect_gold()
    }

    pub fn to_set(&self) -> HashSet<Gem> {
        let mut set = HashSet::new();
        if self.onyx > 0 {
            set.insert(Gem::Onyx);
        }
        if self.sapphire > 0 {
            set.insert(Gem::Sapphire);
        }
        if self.emerald > 0 {
            set.insert(Gem::Emerald);
        }
        if self.ruby > 0 {
            set.insert(Gem::Ruby);
        }
        if self.diamond > 0 {
            set.insert(Gem::Diamond);
        }
        set
    }

    pub fn from_vec(vec: &Vec<Gem>) -> Gems {
        let mut gems = Gems::empty();
        for &color in vec {
            gems[color] += 1;
        }
        gems
    }

    pub fn from_set(set: &HashSet<Gem>) -> Gems {
        let mut gems = Gems::empty();
        for color in set {
            gems[*color] += 1;
        }
        gems
    }

    pub fn total(&self) -> u32 {
        debug_assert!(self.legal(), "Illegal token state: {:?}", self);
        self.onyx as u32
            + self.sapphire as u32
            + self.emerald as u32
            + self.ruby as u32
            + self.diamond as u32
            + self.gold as u32
    }
    pub fn legal(&self) -> bool {
        self.onyx >= 0
            && self.sapphire >= 0
            && self.emerald >= 0
            && self.ruby >= 0
            && self.diamond >= 0
            && self.gold >= 0
    }

    pub fn empty() -> Gems {
        Gems {
            onyx: 0,
            sapphire: 0,
            emerald: 0,
            ruby: 0,
            diamond: 0,
            gold: 0,
        }
    }

    pub fn start(players: u8) -> Gems {
        match players {
            2 => Gems {
                onyx: 4,
                sapphire: 4,
                emerald: 4,
                ruby: 4,
                diamond: 4,
                gold: 5,
            },
            3 => Gems {
                onyx: 5,
                sapphire: 5,
                emerald: 5,
                ruby: 5,
                diamond: 5,
                gold: 5,
            },
            4 => Gems {
                onyx: 7,
                sapphire: 7,
                emerald: 7,
                ruby: 7,
                diamond: 7,
                gold: 5,
            },
            _ => panic!("Invalid number of players"),
        }
    }

    pub fn max(&self, other: &Gems) -> Gems {
        Gems {
            onyx: max(self.onyx, other.onyx),
            sapphire: max(self.sapphire, other.sapphire),
            emerald: max(self.emerald, other.emerald),
            ruby: max(self.ruby, other.ruby),
            diamond: max(self.diamond, other.diamond),
            gold: max(self.gold, other.gold),
        }
    }

    pub fn one(color: Gem) -> Gems {
        let mut gems = Gems::empty();
        gems[color] = 1;
        gems
    }

    pub fn distinct(&self) -> usize {
        let mut count = 0;
        if self.onyx > 0 {
            count += 1;
        }
        if self.sapphire > 0 {
            count += 1
        }
        if self.emerald > 0 {
            count += 1
        }
        if self.ruby > 0 {
            count += 1
        }
        if self.diamond > 0 {
            count += 1
        }
        count
    }
    pub fn can_buy(&self, other: &Gems) -> bool {
        unimplemented!()
    }
}

impl Index<Gem> for Gems {
    type Output = i8;

    fn index<'a>(&'a self, color: Gem) -> &'a i8 {
        match color {
            Gem::Onyx => &self.onyx,
            Gem::Sapphire => &self.sapphire,
            Gem::Emerald => &self.emerald,
            Gem::Ruby => &self.ruby,
            Gem::Diamond => &self.diamond,
            Gem::Gold => &self.gold,
        }
    }
}

impl IndexMut<Gem> for Gems {
    fn index_mut<'a>(&'a mut self, color: Gem) -> &'a mut i8 {
        match color {
            Gem::Onyx => &mut self.onyx,
            Gem::Sapphire => &mut self.sapphire,
            Gem::Emerald => &mut self.emerald,
            Gem::Ruby => &mut self.ruby,
            Gem::Diamond => &mut self.diamond,
            Gem::Gold => &mut self.gold,
        }
    }
}

impl AddAssign for Gems {
    fn add_assign(&mut self, other: Gems) {
        self.onyx += other.onyx;
        self.sapphire += other.sapphire;
        self.emerald += other.emerald;
        self.ruby += other.ruby;
        self.diamond += other.diamond;
        self.gold += other.gold;
        debug_assert!(self.legal());
    }
}

impl SubAssign for Gems {
    fn sub_assign(&mut self, other: Gems) {
        self.onyx -= other.onyx;
        self.sapphire -= other.sapphire;
        self.emerald -= other.emerald;
        self.ruby -= other.ruby;
        self.diamond -= other.diamond;
        self.gold -= other.gold;
        debug_assert!(self.legal());
    }
}

impl Add for Gems {
    type Output = Gems;

    fn add(self, other: Gems) -> Gems {
        let gems = Gems {
            onyx: self.onyx + other.onyx,
            sapphire: self.sapphire + other.sapphire,
            emerald: self.emerald + other.emerald,
            ruby: self.ruby + other.ruby,
            diamond: self.diamond + other.diamond,
            gold: self.gold + other.gold,
        };
        debug_assert!(self.legal());
        gems
    }
}

impl Sub for Gems {
    type Output = Gems;

    fn sub(self, other: Gems) -> Gems {
        let gems = Gems {
            onyx: self.onyx - other.onyx,
            sapphire: self.sapphire - other.sapphire,
            emerald: self.emerald - other.emerald,
            ruby: self.ruby - other.ruby,
            diamond: self.diamond - other.diamond,
            gold: self.gold - other.gold,
        };
        debug_assert!(self.legal());
        gems
    }
}
