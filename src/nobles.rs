use crate::card::Cost;
use crate::gems::Gems;

pub type NobleId = u8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Noble {
    pub points: u8,
    pub id: NobleId,
    requirements: Gems,
}

impl Noble {
    pub fn from_id(id: NobleId) -> Noble{
        Noble::all()[id as usize].clone()
    }

    pub fn all() -> Vec<Noble> {
        vec![
            Noble::new(
                3,
                0,
                Gems {
                    onyx: 0,
                    sapphire: 0,
                    emerald: 4,
                    ruby: 4,
                    diamond: 0,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                1,
                Gems {
                    onyx: 3,
                    sapphire: 0,
                    emerald: 0,
                    ruby: 3,
                    diamond: 3,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                2,
                Gems {
                    onyx: 3,
                    sapphire: 0,
                    emerald: 3,
                    ruby: 3,
                    diamond: 0,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                3,
                Gems {
                    onyx: 0,
                    sapphire: 4,
                    emerald: 0,
                    ruby: 0,
                    diamond: 4,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                4,
                Gems {
                    onyx: 4,
                    sapphire: 0,
                    emerald: 0,
                    ruby: 0,
                    diamond: 4,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                5,
                Gems {
                    onyx: 0,
                    sapphire: 4,
                    emerald: 4,
                    ruby: 0,
                    diamond: 0,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                6,
                Gems {
                    onyx: 0,
                    sapphire: 3,
                    emerald: 3,
                    ruby: 3,
                    diamond: 0,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                7,
                Gems {
                    onyx: 0,
                    sapphire: 3,
                    emerald: 3,
                    ruby: 0,
                    diamond: 3,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                8,
                Gems {
                    onyx: 4,
                    sapphire: 0,
                    emerald: 4,
                    ruby: 0,
                    diamond: 0,
                    gold: 0,
                },
            ),
            Noble::new(
                3,
                9,
                Gems {
                    onyx: 3,
                    sapphire: 3,
                    emerald: 0,
                    ruby: 0,
                    diamond: 3,
                    gold: 0,
                },
            ),
        ]
    }
    fn new(points: u8, id: NobleId, requirements: Gems) -> Noble {
        Noble {
            points,
            id,
            requirements,
        }
    }
    pub fn is_attracted_to(&self, developments: &Gems) -> bool {
        let can_attract = (developments.onyx >= self.requirements.onyx)
            && (developments.sapphire >= self.requirements.sapphire)
            && (developments.emerald >= self.requirements.emerald)
            && (developments.ruby >= self.requirements.ruby)
            && (developments.diamond >= self.requirements.diamond);

        can_attract
    }
    pub fn id(&self) -> NobleId {
        self.id
    }
    pub fn points(&self) -> u8 {
        self.points
    }
    pub fn requirements(&self) -> &Gems {
        &self.requirements
    }
}
