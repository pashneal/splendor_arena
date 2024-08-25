use crate::card::*;
use crate::gem::*;
use crate::gems::Gems;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use cached::proc_macro::cached;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPublicInfo {
    pub points: u8,
    pub num_reserved: usize,
    pub developments: Cost,
    pub gems: Gems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    points: u8,
    noble_points: u8,
    reserved: Vec<CardId>,
    gems: Gems,
    developments: Gems,
    blind_reserved: Vec<CardId>,
}

#[cached]
fn gem_match(cost: Gems, gems: Gems, running_payment: Gems) -> HashSet<Gems> {
    if cost.total() == 0 {
        return HashSet::from_iter(vec![running_payment]);
    }
    if gems.total() == 0 {
        return HashSet::new();
    }

    // Take one token that satisfies the cost or a wild token and recurse
    let mut result = Vec::new();
    for color in Gem::all() {
        if cost[color] > 0 {
            let new_cost = cost - Gems::one(color);

            if gems[color] > 0 {
                let new_gems = gems - Gems::one(color);
                result.extend(gem_match(
                    new_cost,
                    new_gems,
                    running_payment + Gems::one(color),
                ));
            }

            if gems[Gem::Gold] > 0 {
                let new_gems = gems - Gems::one(Gem::Gold);
                result.extend(gem_match(
                    new_cost,
                    new_gems,
                    running_payment + Gems::one(Gem::Gold),
                ));
            }
        }
    }

    HashSet::from_iter(result)
}

impl Player {
    pub fn new() -> Player {
        Player {
            points: 0,
            noble_points: 0,
            reserved: Vec::new(),
            gems: Gems::empty(),
            developments: Gems::empty(),
            blind_reserved: Vec::new(),
        }
    }

    pub fn to_public(&self) -> PlayerPublicInfo {
        PlayerPublicInfo {
            points: self.points,
            num_reserved: self.reserved.len(),
            developments: Cost::from_gems(&self.developments),
            gems: self.gems.clone(),
        }
    }

    pub fn total_points(&self) -> u8 {
        self.points
    }

    pub fn noble_points(&self) -> u8 {
        self.noble_points
    }
    pub fn add_points(&mut self, points: u8) {
        self.points += points;
    }

    pub fn add_noble_points(&mut self) {
        self.points += 3;
        self.noble_points += 3;
    }

    /// Return the number of reserved cards in total
    pub fn num_reserved_cards(&self) -> usize {
        self.reserved.len()
    }

    /// Gets the list of reserved card ids that all players have perfect information of
    pub fn public_reserved(&self) -> Vec<CardId> {
        self.reserved
            .clone()
            .into_iter()
            .filter(|card_id| !self.blind_reserved.contains(card_id))
            .collect()
    }

    /// Gets the list of all cards currently reserved (whether they were blind reserved or not)
    pub fn all_reserved(&self) -> Vec<CardId> {
        self.reserved.clone()
    }
    /// Gets the list of cards that were blind reserved  
    pub fn blind_reserved(&self) -> Vec<CardId> {
        self.blind_reserved.clone()
    }

    pub fn gems(&self) -> &Gems {
        &self.gems
    }

    fn add_development(&mut self, color: Gem) {
        self.developments += Gems::one(color);
    }

    pub fn developments(&self) -> &Gems {
        &self.developments
    }

    pub fn remove_gems(&mut self, gems: Gems) {
        self.gems -= gems;
    }

    pub fn add_gems(&mut self, gems: Gems) {
        self.gems += gems;
    }

    pub fn has_reserved_card(&self, card_id: CardId) -> bool {
        self.reserved.contains(&card_id)
    }

    pub fn purchase_card(&mut self, card: &Card, payment: &Gems) {
        debug_assert!(payment.legal());
        self.gems -= *payment;
        debug_assert!(self.gems.legal());
        self.add_development(card.gem());
        self.points += card.points();
        self.reserved.retain(|&x| x != card.id());
        self.blind_reserved.retain(|&x| x != card.id());
    }

    pub fn reserve_card(&mut self, card_id: CardId) {
        debug_assert!(self.reserved.len() < 3);
        self.reserved.push(card_id);
    }

    pub fn blind_reserve_card(&mut self, card_id: CardId) {
        debug_assert!(self.reserved.len() < 3);
        self.reserved.push(card_id);
        self.blind_reserved.push(card_id);
    }

    /// Returns the token spread that a player needs to afford
    /// a given card.
    pub fn payment_options_for(&self, card: &Card) -> Option<HashSet<Gems>> {
        let cost = card.cost();
        let cost = cost.discounted_with(&self.developments).to_gems();
        let mut total_deficit = 0;
        for color in Gems::all() {
            let deficit = cost[color] - self.gems[color];
            if deficit > 0 {
                total_deficit += deficit;
            }
        }

        // Cannot pay off deficit with wild gems
        if total_deficit > self.gems[Gem::Gold] {
            return None;
        }
        // Card is free!
        let payments = gem_match(cost, self.gems, Gems::empty());
        if payments.len() == 0 {
            return None;
        }
        Some(payments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use crate::gem::Gem;
    use crate::gems::Gems;

    /// Testing strategy:
    ///     payment_to_afford:
    ///         - has 0, 1, >1 wild (gold) gems
    ///         - can afford, cannot afford
    ///         - specific (unique) payment, ambiguous (multiple) payments
    ///         - development discounts (output):
    ///             discount exact, discount more than cost, discount less than cost

    #[test]
    fn test_cannot_afford_1_wild() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Onyx));

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card);
        assert_eq!(payment, None);
    }

    #[test]
    fn test_cannot_afford_0_wild() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Emerald));

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card);
        assert_eq!(payment, None);
    }
    #[test]
    fn test_payment_specific_0_wild_discount_exact() {
        let mut player = Player::new();
        player.add_development(Gem::Ruby);
        player.add_development(Gem::Emerald);
        player.add_development(Gem::Emerald);

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(
            *payment
                .into_iter()
                .take(1)
                .collect::<Vec<_>>()
                .first()
                .unwrap(),
            Gems {
                ruby: 0,
                emerald: 0,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 0,
            }
        );
    }
    #[test]
    fn test_payment_specific_0_wild_discount_less() {
        let mut player = Player::new();
        player.add_development(Gem::Ruby);
        player.add_gems(Gems::one(Gem::Emerald));
        player.add_gems(Gems::one(Gem::Emerald));

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(
            *payment
                .into_iter()
                .take(1)
                .collect::<Vec<_>>()
                .first()
                .unwrap(),
            Gems {
                ruby: 0,
                emerald: 2,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 0,
            }
        );
    }
    #[test]
    fn test_payment_specific_1_wild_discount_less() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_development(Gem::Ruby);
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Emerald));

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 1, "payment not unique: {:?}", payment);
        assert_eq!(
            *payment
                .into_iter()
                .take(1)
                .collect::<Vec<_>>()
                .first()
                .unwrap(),
            Gems {
                ruby: 0,
                emerald: 1,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 1,
            }
        );
    }

    #[test]
    fn test_payment_ambiguous_1_wild() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Emerald));
        player.add_gems(Gems::one(Gem::Emerald));

        let card = Card::all()[4];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 3);

        let set = payment;
        let target = vec![
            Gems {
                ruby: 1,
                emerald: 1,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 1,
            },
            Gems {
                ruby: 0,
                emerald: 2,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 1,
            },
            Gems {
                ruby: 1,
                emerald: 2,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 0,
            },
        ];
        let target = target.into_iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(set, target);
    }

    #[test]
    fn test_payment_specific_2_wild_discount_more() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Emerald));

        player.add_development(Gem::Ruby);
        player.add_development(Gem::Emerald);
        player.add_development(Gem::Emerald);
        player.add_development(Gem::Emerald);
        player.add_development(Gem::Emerald);

        let card = Card::all()[6];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(
            *payment
                .into_iter()
                .take(1)
                .collect::<Vec<_>>()
                .first()
                .unwrap(),
            Gems {
                ruby: 0,
                emerald: 0,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 0,
            }
        )
    }

    #[test]
    fn test_payment_specific_2_wild() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Ruby));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Emerald));

        let card = Card::all()[6];
        let payment = player.payment_options_for(&card).unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(
            *payment
                .into_iter()
                .take(1)
                .collect::<Vec<_>>()
                .first()
                .unwrap(),
            Gems {
                ruby: 0,
                emerald: 1,
                sapphire: 0,
                diamond: 0,
                onyx: 0,
                gold: 2,
            }
        )
    }

    #[test]
    fn test_payment_ambiguous_3_wild() {
        let mut player = Player::new();
        player.add_gems(Gems::one(Gem::Emerald));
        player.add_gems(Gems::one(Gem::Emerald));
        player.add_gems(Gems::one(Gem::Onyx));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Gold));
        player.add_gems(Gems::one(Gem::Gold));

        let card = Card::all()[13];

        let payment = player.payment_options_for(&card).unwrap();

        //             = 0 ways to pay with 0 golds
        // ee.o        = 1 way to pay with 1 gold
        // .e.o | ee.. = 2 ways to pay with 2 golds
        // .e.. | ..o. = 2 ways to pay with 3 golds

        assert_eq!(payment.len(), 5);
    }
}
