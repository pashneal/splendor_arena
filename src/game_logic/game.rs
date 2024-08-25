use crate::card::{Card, CardId};
use crate::gem::Gem;
use crate::nobles::*;
use crate::player::Player;
use crate::gems::Gems;

use rand::seq::SliceRandom;
use rand::thread_rng;

use super::{Action::*, *};

use std::collections::HashSet;
use std::sync::Arc;

use cached::proc_macro::cached;

use log::{debug, error, info, trace};

#[derive(Debug, Clone)]
pub struct Game {
    players: Vec<Player>,
    bank: Gems,
    decks: Vec<Vec<Card>>,
    current_player: usize,
    nobles: Vec<Noble>,
    dealt_cards: Vec<Vec<CardId>>,
    current_phase: Phase,
    card_lookup: Arc<Vec<Card>>,
    history: GameHistory,
    deadlock_count: u8,
}

impl Game {
    /// Initialize the game with given nobles
    fn with_nobles(&mut self, nobles: Vec<NobleId>) {
        let noble_lookup = Noble::all();
        self.nobles = nobles
            .iter()
            .map(|id| noble_lookup[*id as usize].clone())
            .collect();
    }

    /// Initialize the game with given cards
    fn with_initial_cards(&mut self, initial_cards: Vec<Vec<Card>>) {
        // Undeal the initial cards
        self.decks[0].extend(
            self.dealt_cards[0]
                .drain(..)
                .map(|id| self.card_lookup[id as usize]),
        );
        self.decks[1].extend(
            self.dealt_cards[1]
                .drain(..)
                .map(|id| self.card_lookup[id as usize]),
        );
        self.decks[2].extend(
            self.dealt_cards[2]
                .drain(..)
                .map(|id| self.card_lookup[id as usize]),
        );
        // Filter out the initial cards from the decks
        self.decks[0].retain(|card| !initial_cards[0].contains(card));
        self.decks[1].retain(|card| !initial_cards[1].contains(card));
        self.decks[2].retain(|card| !initial_cards[2].contains(card));

        self.dealt_cards[0] = initial_cards[0].iter().map(|card| card.id()).collect();
        self.dealt_cards[1] = initial_cards[1].iter().map(|card| card.id()).collect();
        self.dealt_cards[2] = initial_cards[2].iter().map(|card| card.id()).collect();
    }

    /// Get the number of cards in each deck from tier 1 to 3
    pub fn deck_counts(&self) -> [usize; 3] {
        self.decks
            .iter()
            .map(|deck| deck.len())
            .collect::<Vec<_>>()
            .try_into()
            .expect("Deck size is != 3")
    }

    /// Get the array that maps card ids to cards
    pub fn card_lookup(&self) -> Arc<Vec<Card>> {
        self.card_lookup.clone()
    }

    /// Get the cards that have been dealt to the board
    /// and are face up
    pub fn cards(&self) -> Vec<Vec<CardId>> {
        self.dealt_cards.clone()
    }

    /// Get the gems that are currently available for taking
    pub fn bank(&self) -> &Gems {
        &self.bank
    }

    /// Get the nobles that are currently available
    pub fn nobles(&self) -> &Vec<Noble> {
        &self.nobles
    }

    /// Get the players in the game
    pub fn players(&self) -> &Vec<Player> {
        &self.players
    }

    /// Get the index of the current player
    pub fn current_player_num(&self) -> usize {
        self.current_player
    }

    /// Get the Player object of the current player
    pub fn current_player(&self) -> Player {
        self.players[self.current_player].clone()
    }

    pub fn history(&self) -> GameHistory {
        self.history.clone()
    }

    /// Initialize a new game with the given number of players 
    /// and a global array of cards where indices are card ids
    pub fn new(players: u8, card_lookup: Arc<Vec<Card>>) -> Game {
        let mut decks = Vec::new();
        for tier in 1..=3 {
            let mut deck = Vec::new();
            for card in Card::all() {
                if card.tier() == tier {
                    deck.push(card);
                }
            }
            decks.push(deck);
        }

        let mut nobles = Noble::all();
        nobles.shuffle(&mut thread_rng());
        nobles.truncate(players as usize + 1);

        let mut dealt_cards = Vec::<Vec<CardId>>::new();

        decks[0].shuffle(&mut thread_rng());
        decks[1].shuffle(&mut thread_rng());
        decks[2].shuffle(&mut thread_rng());

        // Deal 4 cards to start
        dealt_cards.push(decks[0].drain(0..4).map(|card| card.id()).collect());
        dealt_cards.push(decks[1].drain(0..4).map(|card| card.id()).collect());
        dealt_cards.push(decks[2].drain(0..4).map(|card| card.id()).collect());

        Game {
            players: (0..players).map(|_| Player::new()).collect(),
            bank: Gems::start(players),
            decks,
            current_player: 0,
            nobles,
            current_phase: Phase::PlayerStart,
            dealt_cards,
            card_lookup,
            history: GameHistory::new(),
            deadlock_count: 0,
        }
    }

    /// Given a game state return all 
    /// legal actions that can be taken
    ///
    /// returns None if the game is deadlocked or over 
    pub fn get_legal_actions(&self) -> Option<Vec<Action>> {
        if self.deadlock_count == 2 * self.players.len() as u8 {
            return None;
        }

        match self.current_phase {
            Phase::NobleAction => {
                let mut available_nobles = Vec::new();
                let player = &self.players[self.current_player];
                for noble in &self.nobles {
                    if noble.is_attracted_to(player.developments()) {
                        available_nobles.push(noble);
                    }
                }
                let nobles: Vec<Action> = available_nobles
                    .into_iter()
                    .map(|n| AttractNoble(n.id()))
                    .collect();
                if nobles.len() == 0 {
                    Some(vec![Pass])
                } else {
                    Some(nobles)
                }
            }
            Phase::PlayerActionEnd => {
                // There are no legal actions remaining if
                // there's a player with >= 15 points and we are on the last player's
                // turn
                if self.current_player == self.players.len() - 1
                    && self.players.iter().any(|p| p.total_points() >= 15)
                {
                    None
                } else {
                    Some(vec![Continue])
                }
            }

            Phase::PlayerGemCapExceeded => {
                let mut running = Gems::empty();
                let player = &self.players[self.current_player];
                let mut gems = player.gems().clone();

                let discard_num = player.gems().total() - 10;
                let choices = choose_gems(&mut gems, &mut running, discard_num);
                let discard_actions = choices.iter().map(|d| Discard(*d)).collect();
                Some(discard_actions)
            }

            Phase::PlayerStart => {
                let mut actions = Vec::<Action>::new();
                let player = &self.players[self.current_player];

                // If num reserved cards < 3:
                // -> Can reserve a card from board
                // -> Can reserve a card from decks that are not empty
                if player.num_reserved_cards() < 3 {
                    for tier in 0..3 {
                        if self.decks[tier].len() > 0 {
                            actions.push(ReserveHidden(tier));
                        }
                        self.dealt_cards[tier].iter().for_each(|card| {
                            actions.push(Reserve(*card));
                        });
                    }
                }

                // If has prerequisites:
                // -> Can purchase a card from board
                // -> Can purchase a card from hand
                for card_index in self
                    .dealt_cards
                    .iter()
                    .flatten()
                    .chain(player.all_reserved().iter())
                {
                    let card = &self.card_lookup[*card_index as usize];
                    if let Some(payments) = player.payment_options_for(&card) {
                        for payment in payments {
                            actions.push(Purchase((*card_index, payment)));
                        }
                    }
                }

                // If there are >= 3 distinct token piles:
                // -> Can take 3 distinct tokens
                // If there are x < 3 distinct token piles:
                // -> Can take x distinct tokens
                let distinct_tokens = self.bank.distinct();
                let take_max = distinct_tokens.min(3) as u32;
                let choices =
                    choose_distinct_gems(&mut self.bank.clone(), &mut Gems::empty(), take_max);

                if take_max > 0 {
                    for choice in choices {
                        actions.push(TakeDistinct(choice.to_set()));
                    }
                }

                // If there are 4 tokens of the same color:
                // -> Can take the two tokens of that color
                for color in Gem::all_expect_gold() {
                    if self.bank[color] >= 4 {
                        actions.push(TakeDouble(color));
                    }
                }

                // In the event of no legal actions, passing is the only
                // legal action
                if actions.len() == 0 {
                    Some(vec![Pass])
                } else {
                    Some(actions)
                }
            }
        }
    }

    /// Given an action and the current phase, determine if the action is legal
    fn is_phase_correct_for(&self, action: Action) -> bool {
        match self.current_phase {
            Phase::PlayerStart => match action {
                TakeDouble(_) => true,
                TakeDistinct(_) => true,
                Reserve(_) => true,
                ReserveHidden(_) => true,
                Purchase(_) => true,
                Pass => true,
                _ => false,
            },
            Phase::PlayerGemCapExceeded => match action {
                Discard(_) => true,
                _ => false,
            },
            Phase::NobleAction => match action {
                AttractNoble(_) => true,
                Pass => true,
                _ => false,
            },
            Phase::PlayerActionEnd => match action {
                Continue => true,
                _ => false,
            },
        }
    }

    /// Deals a card to a certain tier and return the id
    /// Deals no card if the deck for that tier is exhausted
    fn deal_to(&mut self, tier: usize) -> Option<CardId> {
        if self.decks[tier].len() == 0 {
            return None;
        }
        let new_card = self.decks[tier].pop().unwrap();
        self.dealt_cards[tier].push(new_card.id());
        Some(new_card.id())
    }

    fn has_card(&self, card_id: CardId) -> bool {
        for tier in &self.dealt_cards {
            if tier.contains(&card_id) {
                return true;
            }
        }
        false
    }

    /// Removes a faceup card from the board
    /// and return the tier it was removed from
    fn remove_card(&mut self, card_id: CardId) -> usize {
        debug_assert!(self.has_card(card_id));

        let mut remove_index = (5, 5);
        for (tier, tiers) in self.dealt_cards.iter().enumerate() {
            for (index, id) in tiers.iter().enumerate() {
                if *id == card_id {
                    remove_index = (tier, index);
                }
            }
        }

        let (i, j) = remove_index;
        self.dealt_cards[i].remove(j);
        i
    }

    pub fn advance_history_with(&mut self, history: GameHistory) {
        for (p, a) in history {
            self.history.add(p, a.clone());
            self.play_action(a);
        }
    }

    /// Takes an action and updates the game state accordingly
    /// Preconditions:
    ///     the action is a legal action for the current phase as dictated
    ///     by the game state and the rules of the game of Splendor
    ///
    /// Note: this function makes judicious use of debug_assert! to check many
    /// preconditions. I'm experimenting with this style of error checking
    /// alongside TDD to see if developer productivity is improved
    pub fn play_action(&mut self, action: Action) {
        debug_assert!(self.is_phase_correct_for(action.clone()));

        // If there are enough passes in a row, the game is over (deadlocked)
        match action {
            Pass => {
                self.deadlock_count += 1;
            }
            Continue => {}
            _ => {
                self.deadlock_count = 0;
            }
        }

        self.history.add(self.current_player, action.clone());

        let next_phase = match action {
            TakeDouble(color) => {
                // Preconditions:
                // -> Must be from a pile that has >= 4
                // -> Cannot take a wild token with this action
                debug_assert!(self.bank[color] >= 4);
                debug_assert!(!matches!(color, Gem::Gold));

                // TODO: this is a little weird but we can change later
                // right now it's using debug asserts on the
                // Sub operations to check preconditions
                self.bank -= Gems::one(color);
                self.bank -= Gems::one(color);

                let player = &mut self.players[self.current_player];
                player.add_gems(Gems::one(color));
                player.add_gems(Gems::one(color));

                if player.gems().total() > 10 {
                    Phase::PlayerGemCapExceeded
                } else {
                    Phase::NobleAction
                }
            }

            TakeDistinct(colors) => {
                // Preconditions
                // -> Can take 1,2, or 3 distinct colors
                debug_assert!(colors.len() <= 3 && colors.len() > 0);
                // -> Which all exist on the board
                debug_assert!(colors.iter().all(|c| self.bank[*c] >= 1));
                // -> And you can only choose 2 or 1 tokens if all other
                // piles are depleted (See Splendor FAQ)
                debug_assert!(if colors.len() < 3 {
                    self.bank.distinct() == colors.len()
                } else {
                    true
                });
                // -> Cannot take a wild token with this action
                debug_assert!(colors.iter().all(|c| !matches!(c, Gem::Gold)));

                let player = &mut self.players[self.current_player];
                player.add_gems(Gems::from_set(&colors));

                for color in colors {
                    self.bank -= Gems::one(color);
                }

                if player.gems().total() > 10 {
                    Phase::PlayerGemCapExceeded
                } else {
                    Phase::NobleAction
                }
            }

            Reserve(card_id) => {
                // Preconditions
                // -> Card with id:card_id is on the board
                debug_assert!(self.dealt_cards.iter().flatten().any(|id| card_id == *id));

                let tier = self.remove_card(card_id);
                self.deal_to(tier);

                // See if the player gets an wild/gold gem
                let gets_gold = self.bank[Gem::Gold] > 0;
                let player = &mut self.players[self.current_player];
                player.reserve_card(card_id);

                if gets_gold {
                    player.add_gems(Gems::one(Gem::Gold));
                    self.bank -= Gems::one(Gem::Gold);
                }

                if player.gems().total() > 10 {
                    Phase::PlayerGemCapExceeded
                } else {
                    Phase::NobleAction
                }
            }

            ReserveHidden(tier) => {
                let new_card_id = self.deal_to(tier).expect("Cannot reserve from empty deck");
                self.remove_card(new_card_id);

                let gets_gold = self.bank[Gem::Gold] > 0;
                let player = &mut self.players[self.current_player];

                if gets_gold {
                    player.add_gems(Gems::one(Gem::Gold));
                    self.bank -= Gems::one(Gem::Gold);
                }

                player.blind_reserve_card(new_card_id);

                if player.gems().total() > 10 {
                    Phase::PlayerGemCapExceeded
                } else {
                    Phase::NobleAction
                }
            }

            Purchase((card_id, payment)) => {
                let card = self.card_lookup[card_id as usize];
                let player = &self.players[self.current_player];
                // Preconditions:
                // -> The tokens being used is one of the legal ways to purchase this card
                debug_assert!({
                    let payment_options = player.payment_options_for(&card);
                    let payments = payment_options.unwrap_or(HashSet::new());
                    payments.iter().any(|&p| p == payment)
                });
                // -> Must have been on the board or in the player's reserved cards
                debug_assert!(self.has_card(card_id) || player.has_reserved_card(card_id));

                let player = &mut self.players[self.current_player];
                player.purchase_card(&card, &payment);

                // Put the payment back on the board
                self.bank += payment;

                if self.has_card(card_id) {
                    let tier = self.remove_card(card_id);
                    self.deal_to(tier);
                }

                Phase::NobleAction
            }

            Discard(discards) => {
                // Preconditions:
                // -> Must have greater than 10 tokens
                // -> Must discard enough tokens to be == 10
                // -> Must be discarding tokens already present in the player's gems
                let player = &mut self.players[self.current_player];
                debug_assert!(player.gems().total() > 10);
                debug_assert!(player.gems().total() - discards.total() == 10);
                debug_assert!((*player.gems() - discards).legal());

                player.remove_gems(discards);
                self.bank += discards;

                Phase::NobleAction
            }

            AttractNoble(noble_id) => {
                // Preconditions:
                // -> The player has enough development cards to attract the noble
                let player = &mut self.players[self.current_player];
                let noble_index = self.nobles.iter().position(|n| n.id() == noble_id).unwrap();
                let noble = &self.nobles[noble_index];
                debug_assert!(noble.is_attracted_to(player.developments()));

                player.add_noble_points();
                self.nobles.remove(noble_index);

                Phase::PlayerActionEnd
            }

            Continue => {
                self.current_player = (self.current_player + 1) % self.players.len();
                Phase::PlayerStart
            }

            Pass => {
                // Preconditions:
                // -> The player has no other legal actions in this phase
                debug_assert!({
                    let legal_actions = self.get_legal_actions();
                    let legal_actions = legal_actions.unwrap();
                    legal_actions.len() == 1 && legal_actions.contains(&Action::Pass)
                });

                match self.current_phase {
                    Phase::PlayerStart => Phase::NobleAction,
                    Phase::NobleAction => Phase::PlayerActionEnd,
                    _ => panic!("Cannot pass in this phase"),
                }
            }
        };

        debug_assert!(
            Gems::start(self.players.len() as u8)
                == self.bank
                    + self
                        .players
                        .iter()
                        .map(|p| p.gems())
                        .fold(Gems::empty(), |a, b| a + *b),
            "Tokens should be conserved"
        );
        self.current_phase = next_phase;
    }

    pub fn game_over(&self) -> bool {
        self.get_legal_actions().is_none()
    }

    /// Given a terminal game state, determine the winner
    pub fn get_winner(&self) -> Option<usize> {
        // The winner of a splendor game is the player with the most points
        // and fewest development cards in the event of a point tie
        // Note: there is no indication of what to do in the event of a cards + point tie
        // TODO: return None instead of whatever we do now

        // Preconditions:
        // -> The game is over
        // -> Someone has at least >= 15 points or the game is deadlocked
        debug_assert!(self.get_legal_actions().is_none());
        debug_assert!(
            self.players.iter().any(|p| p.total_points() >= 15)
                || self.deadlock_count >= (2 * self.players.len() as u8)
        );

        let mut max_points = 15;
        let mut min_developments = u32::MAX;
        let mut winner = None;
        for (i, player) in self.players.iter().enumerate() {
            if player.total_points() > max_points {
                max_points = player.total_points();
                min_developments = player.developments().total();
                winner = Some(i);
            } else if player.total_points() == max_points {
                if player.developments().total() < min_developments as u32 {
                    min_developments = player.developments().total();
                    winner = Some(i);
                }
            }
        }

        winner
    }

    /// Given a game state, play random legal moves until the game is over
    /// Returns the winner of the game
    /// Returns None if there is no clear winner 
    pub fn rollout(&mut self) -> Option<usize> {
        loop {
            let actions = self.get_legal_actions();
            // If there are no legal actions, the game is over
            // and we should break out of the loop
            if actions.is_none() {
                break;
            }

            let actions = actions.unwrap();

            let action = actions
                .choose(&mut thread_rng())
                .expect("List should not be empty");
            self.play_action(action.clone());
        }

        self.get_winner()
    }
}

#[cfg(test)]
pub mod test {
    use super::Gem::*;
    pub use super::*;
    #[test]

    pub fn test_choose_tokens_1() {
        let mut gems = Gems::from_vec(&vec![
            Gem::Ruby,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Emerald,
        ]);
        let mut running = Gems::empty();
        let choices = choose_gems(&mut gems, &mut running, 1);
        assert_eq!(
            choices,
            HashSet::from_iter(vec![
                Gems::from_vec(&vec![Gem::Ruby]),
                Gems::from_vec(&vec![Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Emerald]),
            ])
        );
    }

    #[test]
    pub fn test_choose_tokens_2() {
        let mut gems = Gems::from_vec(&vec![
            Gem::Ruby,
            Gem::Ruby,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Emerald,
        ]);
        let mut running = Gems::empty();
        let choices = choose_gems(&mut gems, &mut running, 2);
        assert_eq!(
            choices,
            HashSet::from_iter(vec![
                Gems::from_vec(&vec![Gem::Ruby, Gem::Ruby]),
                Gems::from_vec(&vec![Gem::Sapphire, Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Emerald, Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Emerald]),
            ])
        );
    }

    #[test]
    pub fn test_choose_3_distinct_tokens() {
        let mut gems = Gems::start(2);
        let mut running = Gems::empty();
        let choices = choose_distinct_gems(&mut gems, &mut running, 3);
        assert_eq!(
            choices,
            HashSet::from_iter(vec![
                Gems::from_vec(&vec![Gem::Ruby, Gem::Sapphire, Gem::Emerald]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Sapphire, Gem::Diamond]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Sapphire, Gem::Onyx]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Emerald, Gem::Diamond]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Emerald, Gem::Onyx]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Diamond, Gem::Onyx]),
                Gems::from_vec(&vec![Gem::Sapphire, Gem::Emerald, Gem::Diamond]),
                Gems::from_vec(&vec![Gem::Sapphire, Gem::Emerald, Gem::Onyx]),
                Gems::from_vec(&vec![Gem::Sapphire, Gem::Diamond, Gem::Onyx]),
                Gems::from_vec(&vec![Gem::Emerald, Gem::Diamond, Gem::Onyx]),
            ])
        );
    }

    #[test]
    pub fn test_choose_distinct_tokens() {
        let mut gems = Gems::from_vec(&vec![
            Gem::Ruby,
            Gem::Ruby,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Sapphire,
            Gem::Emerald,
        ]);
        let mut running = Gems::empty();
        let choices = choose_distinct_gems(&mut gems, &mut running, 2);
        assert_eq!(
            choices,
            HashSet::from_iter(vec![
                Gems::from_vec(&vec![Gem::Emerald, Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Sapphire]),
                Gems::from_vec(&vec![Gem::Ruby, Gem::Emerald]),
            ])
        );
    }

    #[test]
    pub fn test_init_legal_actions() {
        let card_lookup = Arc::new(Card::all());
        let game = Game::new(2, card_lookup);
        let actions = game.get_legal_actions().unwrap();

        // 3 hiddens decks to choose from (ReserveHidden)
        // 12 cards to choose from (Reserve)
        // 5 colors to choose from (TakeDouble)
        // 5 choose 3 = 10 colors to choose from (TakeDistinct)
        // 0 cards able to be purchased
        // sum = 30
        assert_eq!(actions.len(), 30);
    }

    #[test]
    pub fn test_init_winners() {
        // Note: this is manually inspected as a test for now due to time contraints
    }

    #[test]
    pub fn test_init_legal_rounds_specific_board_state() {
        let mut game = Game::new(3, Arc::new(Card::all()));
        let cards = Card::all();
        game.with_nobles(vec![2, 3, 0, 9]);
        game.with_initial_cards(vec![
            vec![cards[31], cards[10], cards[8], cards[17]],
            vec![cards[43], cards[66], cards[47], cards[67]],
            vec![cards[89], cards[80], cards[86], cards[74]],
        ]);
        game.play_action(TakeDouble(Gem::Onyx));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 29);
        assert_eq!(!actions.contains(&TakeDouble(Gem::Onyx)), true);

        game.play_action(TakeDistinct(HashSet::from_iter(vec![
            Gem::Diamond,
            Gem::Emerald,
            Gem::Ruby,
        ])));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 29);
        assert_eq!(!actions.contains(&TakeDouble(Gem::Onyx)), true);

        game.play_action(TakeDouble(Gem::Diamond));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 28);

        game.play_action(TakeDistinct(HashSet::from_iter(vec![
            Gem::Diamond,
            Gem::Emerald,
            Gem::Ruby,
        ])));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 26);

        game.play_action(TakeDistinct(HashSet::from_iter(vec![
            Gem::Diamond,
            Gem::Emerald,
            Gem::Ruby,
        ])));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 30 - 4 - 6);

        game.play_action(TakeDouble(Gem::Sapphire));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 30 - 5 - 6 + 1);

        game.play_action(Purchase((
            8,
            Gems::from_vec(&vec![
                Gem::Diamond,
                Gem::Emerald,
                Gem::Ruby,
                Gem::Onyx,
            ]),
        )));
        game.play_action(Pass);
        game.play_action(Continue);

        let actions = game.get_legal_actions().unwrap();
        assert!((actions.len() == 30 - 4 + 1) || (actions.len() == 30 - 4 + 2));
    }

    #[test]
    pub fn test_init_legal_round() {
        let card_lookup = Arc::new(Card::all());
        let mut game = Game::new(4, card_lookup);
        let actions = game.get_legal_actions().unwrap();

        // 3 hiddens decks to choose from (ReserveHidden)
        // 12 cards to choose from (Reserve)
        // 5 colors to choose from (TakeDouble)
        // 5 choose 3 = 10 colors to choose from (TakeDistinct)
        // 0 cards able to be purchased
        // sum = 30

        assert_eq!(actions.len(), 30);
        game.play_action(Action::ReserveHidden(0));
        game.play_action(Pass);
        let actions = game.get_legal_actions().unwrap();
        assert_eq!(Action::Continue, actions[0].clone());

        game.play_action(Action::Continue);
        let actions = game.get_legal_actions().unwrap();
        assert_eq!(actions.len(), 30);
    }

    #[test]
    pub fn test_randomized_rollout() {
        let card_lookup = Arc::new(Card::all());
        for _ in 0..20000 {
            let mut game = Game::new(4, card_lookup.clone());
            game.rollout();
        }
    }
}
