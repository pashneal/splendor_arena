use crate::*;
use lazy_static::lazy_static;
use pyo3::prelude::*;
use serde::Deserialize;
use std::time::Duration;
use tungstenite::{connect, Message};
use url::Url;

lazy_static! {
    static ref CARD_LOOKUP: [Card; 90] = Card::all_const();
}

/// A Python wrapper for the `Card` struct
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyCard {
    pub id: CardId,
    pub tier: u8,
    pub points: u8,
    pub cost: PyGems,
    pub gem: PyGem,
}

impl PyCard {
    pub fn from(card: &Card) -> Self {
        PyCard {
            id: card.id(),
            tier: card.tier(),
            points: card.points(),
            cost: PyGems::from(card.cost().to_gems()),
            gem: PyGem::from(card.gem()),
        }
    }
    pub fn from_id(card_id: CardId) -> Self {
        if card_id >= CARD_LOOKUP.len() as CardId {
            panic!("Invalid card id: [{}], card ids must be from 0-89", card_id);
        }
        let card = CARD_LOOKUP[card_id as usize];
        PyCard::from(&card)
    }
}

#[pymethods]
impl PyCard {
    /// Get a list of all the possible cards
    #[staticmethod]
    pub fn all_possible_cards() -> Vec<PyCard> {
        CARD_LOOKUP.iter().map(PyCard::from).collect()
    }

    /// Initialize a new PyCard from a card id
    #[new]
    pub fn new(id: CardId) -> PyCard {
        if id >= CARD_LOOKUP.len() as CardId {
            panic!("Invalid card id: {}", id);
        }
        PyCard::from_id(id)
    }

    pub fn __str__(&self) -> String {
        format!("Card(id: {})", self.id)
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    #[getter]
    pub fn id(&self) -> CardId {
        self.id
    }

    #[getter]
    pub fn tier(&self) -> u8 {
        self.tier
    }

    #[getter]
    pub fn points(&self) -> u8 {
        self.points
    }

    #[getter]
    pub fn cost(&self) -> PyGems {
        self.cost.clone()
    }

    #[getter]
    pub fn gem(&self) -> PyGem {
        self.gem.clone()
    }

    pub fn __eq__(&self, other: &PyCard) -> bool {
        self == other
    }
}

/// A Python wrapper for the `Gem` enum
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyGem {
    Onyx,
    Sapphire,
    Emerald,
    Ruby,
    Diamond,
    Gold,
}

impl PyGem {
    pub fn from(gem: Gem) -> Self {
        match gem {
            Gem::Onyx => PyGem::Onyx,
            Gem::Sapphire => PyGem::Sapphire,
            Gem::Emerald => PyGem::Emerald,
            Gem::Ruby => PyGem::Ruby,
            Gem::Diamond => PyGem::Diamond,
            Gem::Gold => PyGem::Gold,
        }
    }
}

#[pymethods]
impl PyGem {
    pub fn __str__(&self) -> String {
        match self {
            PyGem::Onyx => "Onyx".to_string(),
            PyGem::Sapphire => "Sapphire".to_string(),
            PyGem::Emerald => "Emerald".to_string(),
            PyGem::Ruby => "Ruby".to_string(),
            PyGem::Diamond => "Diamond".to_string(),
            PyGem::Gold => "Gold".to_string(),
        }
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// A python wrapper for the `Gems` struct
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyGems {
    #[pyo3(get)]
    pub onyx: i8,
    #[pyo3(get)]
    pub sapphire: i8,
    #[pyo3(get)]
    pub emerald: i8,
    #[pyo3(get)]
    pub ruby: i8,
    #[pyo3(get)]
    pub diamond: i8,
    #[pyo3(get)]
    pub gold: i8,
}

impl PyGems {
    pub fn from_cost(cost: Cost) -> Self {
        let gems = cost.to_gems();
        PyGems {
            onyx: gems.onyx,
            sapphire: gems.sapphire,
            emerald: gems.emerald,
            ruby: gems.ruby,
            diamond: gems.diamond,
            gold: gems.gold,
        }
    }
    pub fn from(gems: Gems) -> Self {
        PyGems {
            onyx: gems.onyx,
            sapphire: gems.sapphire,
            emerald: gems.emerald,
            ruby: gems.ruby,
            diamond: gems.diamond,
            gold: gems.gold,
        }
    }

    pub fn into_gems(self) -> Gems {
        Gems {
            onyx: self.onyx,
            sapphire: self.sapphire,
            emerald: self.emerald,
            ruby: self.ruby,
            diamond: self.diamond,
            gold: self.gold,
        }
    }
}

#[pymethods]
impl PyGems {
    #[new]
    pub fn new(
        onyx: Option<i8>,
        sapphire: Option<i8>,
        emerald: Option<i8>,
        ruby: Option<i8>,
        diamond: Option<i8>,
        gold: Option<i8>,
    ) -> Self {
        PyGems {
            onyx: onyx.unwrap_or(0),
            sapphire: sapphire.unwrap_or(0),
            emerald: emerald.unwrap_or(0),
            ruby: ruby.unwrap_or(0),
            diamond: diamond.unwrap_or(0),
            gold: gold.unwrap_or(0),
        }
    }

    pub fn __str__(&self) -> String {
        //TODO : perhaps we ignore the 0 values?
        format!(
            "onyx: {}, sapphire: {}, emerald: {}, ruby: {}, diamond: {}, gold: {}",
            self.onyx, self.sapphire, self.emerald, self.ruby, self.diamond, self.gold
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyActionType {
    TakeGems,
    ReserveFaceUp,
    ReserveFaceDown,
    Discard,
    Purchase,
    AttractNoble,
    Pass,
    Continue,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyAction {
    action_type: PyActionType,
    card_id: Option<CardId>,
    noble_id: Option<NobleId>,
    gems: Option<PyGems>,
    tier: Option<usize>,
}

/// Rust side only functions for the PyAction enum
impl PyAction {
    pub fn from(action: Action) -> Self {
        let action_type = match &action {
            Action::TakeDouble(_) => PyActionType::TakeGems,
            Action::TakeDistinct(_) => PyActionType::TakeGems,
            Action::Reserve(_) => PyActionType::ReserveFaceUp,
            Action::ReserveHidden(_) => PyActionType::ReserveFaceDown,
            Action::Discard(_) => PyActionType::Discard,
            Action::Purchase(_) => PyActionType::Purchase,
            Action::AttractNoble(_) => PyActionType::AttractNoble,
            Action::Pass => PyActionType::Pass,
            Action::Continue => PyActionType::Continue,
        };
        let card_id = match &action {
            Action::Reserve(card_id) => Some(*card_id),
            Action::Purchase((card_id, _)) => Some(*card_id),
            _ => None,
        };

        let noble_id = match &action {
            Action::AttractNoble(noble_id) => Some(*noble_id),
            _ => None,
        };

        let gems = match &action {
            Action::TakeDouble(color) => {
                let color = *color;
                let mut gems = Gems::empty();
                gems += Gems::one(color.clone());
                gems += Gems::one(color.clone());
                Some(PyGems::from(gems))
            }
            Action::TakeDistinct(color_set) => Some(PyGems::from(Gems::from_set(color_set))),
            Action::Discard(gems) => Some(PyGems::from(*gems)),
            Action::Purchase((_, gems)) => Some(PyGems::from(*gems)),
            _ => None,
        };

        let tier = match &action {
            Action::ReserveHidden(tier) => Some(*tier),
            _ => None,
        };

        PyAction {
            action_type,
            card_id,
            noble_id,
            gems,
            tier,
        }
    }

    pub fn into_action(self) -> Action {
        match self.action_type {
            PyActionType::TakeGems => {
                let py_gems = self.gems();
                let gems = py_gems.into_gems();
                let is_double = gems.total() == 2 && gems.to_set().len() == 1;

                match is_double {
                    true => {
                        let mut color = Gem::Gold;
                        for c in gems.to_set() {
                            color = c
                        }
                        Action::TakeDouble(color)
                    }
                    false => Action::TakeDistinct(gems.to_set()),
                }
            }
            PyActionType::ReserveFaceUp => Action::Reserve(self.card_id()),
            PyActionType::ReserveFaceDown => Action::ReserveHidden(self.tier()),
            PyActionType::Discard => Action::Discard(self.gems().into_gems()),
            PyActionType::Purchase => Action::Purchase((self.card_id(), self.gems().into_gems())),
            PyActionType::AttractNoble => Action::AttractNoble(self.noble_id()),
            PyActionType::Pass => Action::Pass,
            PyActionType::Continue => Action::Continue,
        }
    }
}

/// Separate the Rust-only struct enum Action to Python-like objects with PyAction
/// TODO: (if i'm feeling nice) make error messages more helpful?
#[pymethods]
impl PyAction {
    pub fn __str__(&self) -> String {
        match self.action_type.clone() {
            PyActionType::TakeGems => {
                let gems = self.gems();
                format!("TakeGems({})", gems.__str__())
            }
            PyActionType::ReserveFaceUp => {
                let card_id = self.card_id();
                format!("ReserveFaceUp(card_id : {})", card_id)
            }
            PyActionType::ReserveFaceDown => {
                let tier = self.tier();
                format!("ReserveFaceDown(tier : {})", tier)
            }
            PyActionType::Discard => {
                let gems = self.gems();
                format!("Discard({})", gems.__str__())
            }
            PyActionType::Purchase => {
                let card_id = self.card_id();
                let gems = self.gems();
                format!("Purchase({}, {})", card_id, gems.__str__())
            }
            PyActionType::AttractNoble => {
                let noble_id = self.noble_id();
                format!("AttractNoble(noble_id : {})", noble_id)
            }
            PyActionType::Pass => "Pass".to_string(),
            PyActionType::Continue => "Continue".to_string(),
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{}", self.__str__())
    }

    #[getter]
    pub fn action_type(&self) -> PyActionType {
        self.action_type.clone()
    }

    #[getter]
    pub fn card(&self) -> PyCard {
        let error_message = format!(
            "This action type ({:?}) does not have an associated card",
            self.action_type
        );
        PyCard::from_id(self.card_id.expect(&error_message))
    }

    #[getter]
    pub fn card_id(&self) -> CardId {
        let error_message = format!(
            "This action type ({:?}) does not have an associated card_id",
            self.action_type
        );
        self.card_id.expect(&error_message)
    }

    #[getter]
    pub fn noble_id(&self) -> NobleId {
        let error_message = format!(
            "This action type ({:?}) does not have an associated noble_id",
            self.action_type
        );
        self.noble_id.expect(&error_message)
    }

    #[getter]
    pub fn gems(&self) -> PyGems {
        match self.gems.clone() {
            None => panic!(
                "This action type ({:?}) does not have gems",
                self.action_type
            ),
            Some(gems) => gems,
        }
    }

    #[getter]
    pub fn tier(&self) -> usize {
        match self.tier {
            None => panic!(
                "This action type ({:?}) does not have gems",
                self.action_type
            ),
            Some(tier) => tier,
        }
    }

    pub fn __eq__(&self, other: &PyAction) -> bool {
        self.action_type == other.action_type
            && self.card_id == other.card_id
            && self.noble_id == other.noble_id
            && self.gems == other.gems
            && self.tier == other.tier
    }

    #[staticmethod]
    pub fn purchase(
        card: Option<PyCard>,
        card_id: Option<CardId>,
        onyx: Option<i8>,
        sapphire: Option<i8>,
        emerald: Option<i8>,
        ruby: Option<i8>,
        diamond: Option<i8>,
        gold: Option<i8>,
    ) -> Self {
        // Make sure only card or card_id is passed in
        if card.is_some() && card_id.is_some() {
            panic!("Only one of card or card_id should be passed in! Not both");
        }
        if card.is_none() && card_id.is_none() {
            panic!("Either card or card_id should be passed in!");
        }
        let card_id = match card {
            Some(card) => Some(card.id),
            None => card_id,
        };
        PyAction {
            action_type: PyActionType::Purchase,
            card_id,
            noble_id: None,
            gems: Some(PyGems::new(onyx, sapphire, emerald, ruby, diamond, gold)),
            tier: None,
        }
    }

    #[staticmethod]
    pub fn reserve_face_down(tier: Option<usize>) -> Self {
        PyAction {
            action_type: PyActionType::ReserveFaceDown,
            card_id: None,
            noble_id: None,
            gems: None,
            tier,
        }
    }

    #[staticmethod]
    pub fn reserve_face_up(card: Option<PyCard>, card_id: Option<CardId>) -> Self {
        // Make sure only card or card_id is passed in
        if card.is_some() && card_id.is_some() {
            panic!("Only one of card or card_id should be passed in! Not both");
        }
        if card.is_none() && card_id.is_none() {
            panic!("Either card or card_id should be passed in!");
        }
        let card_id = match card {
            Some(card) => Some(card.id),
            None => card_id,
        };
        PyAction {
            action_type: PyActionType::ReserveFaceUp,
            card_id,
            noble_id: None,
            gems: None,
            tier: None,
        }
    }

    #[staticmethod]
    pub fn take_gems(
        onyx: Option<i8>,
        sapphire: Option<i8>,
        emerald: Option<i8>,
        ruby: Option<i8>,
        diamond: Option<i8>,
    ) -> Self {
        // TODO: we can check against legal actions and
        // be sure to only allow legal gem takes, and point out
        // specifically which gems are illegal
        PyAction {
            action_type: PyActionType::TakeGems,
            card_id: None,
            noble_id: None,
            gems: Some(PyGems::new(onyx, sapphire, emerald, ruby, diamond, None)),
            tier: None,
        }
    }

    #[staticmethod]
    pub fn discard(
        onyx: Option<i8>,
        sapphire: Option<i8>,
        emerald: Option<i8>,
        ruby: Option<i8>,
        diamond: Option<i8>,
    ) -> Self {
        PyAction {
            action_type: PyActionType::Discard,
            card_id: None,
            noble_id: None,
            gems: Some(PyGems::new(onyx, sapphire, emerald, ruby, diamond, None)),
            tier: None,
        }
    }

    #[staticmethod]
    pub fn attract_noble(noble_id: Option<NobleId>) -> Self {
        PyAction {
            action_type: PyActionType::AttractNoble,
            card_id: None,
            noble_id,
            gems: None,
            tier: None,
        }
    }
}

/// A Python wrapper for the `ClientInfo` struct
#[pyclass]
pub struct PyClientInfo {
    #[pyo3(get)]
    pub board: PyBoard,
    #[pyo3(get)]
    pub history: PyGameHistory,
    #[pyo3(get)]
    pub players: Vec<PyPlayer>,
    pub current_player: PyPlayer,
    pub player_index: usize,
    #[pyo3(get)]
    pub legal_actions: Vec<PyAction>,
    time_endpoint_url: String,
}

impl PyClientInfo {
    pub fn from_client_info(client_info: ClientInfo) -> Self {
        // TODO: going to need to
        // make sure that the number of players
        // is conveyed to the python side in
        // the __init__ function
        let legal_actions = client_info.legal_actions;
        let py_legal_actions = legal_actions.into_iter().map(PyAction::from).collect();
        let py_current_player =
            PyPlayer::from(&client_info.current_player, client_info.current_player_num);
        let mut py_players: Vec<PyPlayer> = client_info
            .players
            .iter()
            .enumerate()
            .map(|(index, player)| PyPlayer::from_public(player, index))
            .collect();

        py_players[py_current_player.index] = py_current_player.clone();

        let py_board = PyBoard::from(&client_info.board);
        let py_game_history = PyGameHistory::from(client_info.history);

        PyClientInfo {
            board: py_board,
            history: py_game_history,
            players: py_players,
            current_player: py_current_player,
            player_index: client_info.current_player_num,
            legal_actions: py_legal_actions,
            time_endpoint_url: client_info.time_endpoint_url,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeRemaining {
    pub time_remaining: Duration,
}

/// TODO: would an opponents() method be useful??
///
/// API for the Python clients to access the info
/// of the game sent from a connected server
#[pymethods]
impl PyClientInfo {
    pub fn face_up_cards(&self, tier: Option<usize>) -> PyResult<Vec<PyCard>> {
        self.board.face_up_cards(tier)
    }

    #[getter]
    pub fn me(&self) -> PyPlayer {
        self.current_player.clone()
    }

    #[getter]
    pub fn num_players(&self) -> usize {
        self.players.len()
    }

    pub fn time_remaining(&self) -> f64 {
        let response = reqwest::blocking::get(&self.time_endpoint_url)
            .expect("Server did not response with time remaining");
        let response: TimeRemaining = response
            .json()
            .expect("Could not parse time remaining response");
        response.time_remaining.as_millis() as f64
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyPlayer {
    #[pyo3(get)]
    index: usize,
    #[pyo3(get)]
    total_points: u8,
    #[pyo3(get)]
    num_reserved_cards: usize,
    #[pyo3(get)]
    gems: PyGems,
    #[pyo3(get)]
    developments: PyGems,
    reserved_cards: Option<Vec<PyCard>>,
}

impl PyPlayer {
    pub fn from(player: &Player, index: usize) -> Self {
        PyPlayer {
            index,
            total_points: player.total_points(),
            reserved_cards: Some(
                player
                    .all_reserved()
                    .into_iter()
                    .map(PyCard::from_id)
                    .collect(),
            ),
            num_reserved_cards: player.num_reserved_cards(),
            gems: PyGems::from(*player.gems()),
            developments: PyGems::from(*player.developments()),
        }
    }

    pub fn from_public(player: &PlayerPublicInfo, index: usize) -> Self {
        PyPlayer {
            index,
            total_points: player.points,
            reserved_cards: None,
            num_reserved_cards: player.num_reserved,
            gems: PyGems::from(player.gems),
            developments: PyGems::from(player.developments.to_gems()),
        }
    }
}

#[pymethods]
impl PyPlayer {
    #[getter]
    pub fn reserved_cards(&self) -> PyResult<Vec<PyCard>> {
        if self.reserved_cards.is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                "Attempted to peek at the reserved_cards of an opponent!",
            ));
        }
        Ok(self.reserved_cards.clone().unwrap())
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyNoble {
    #[pyo3(get)]
    pub points: u8,
    #[pyo3(get)]
    pub cost: PyGems,
    #[pyo3(get)]
    pub id: NobleId,
}

impl PyNoble {
    pub fn from(noble: &Noble) -> Self {
        PyNoble {
            points: noble.points(),
            cost: PyGems::from(*noble.requirements()),
            id: noble.id(),
        }
    }
}

#[pymethods]
impl PyNoble {
    #[new]
    pub fn new(id: NobleId) -> PyNoble {
        let noble = Noble::all()[id as usize].clone();
        PyNoble::from(&noble)
    }

    pub fn __eq__(&self, other: &PyNoble) -> bool {
        self == other
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyBoard {
    #[pyo3(get)]
    pub deck_counts: [usize; 3],
    pub available_cards: Vec<Vec<CardId>>,
    #[pyo3(get)]
    pub nobles: Vec<PyNoble>,
    #[pyo3(get)]
    pub gems: PyGems,
}

impl PyBoard {
    pub fn from(board: &Board) -> Self {
        let board_nobles = board.nobles.clone().into_iter().map(PyNoble::new).collect();
        PyBoard {
            deck_counts: board.deck_counts,
            available_cards: board.available_cards.clone(),
            nobles: board_nobles,
            gems: PyGems::from(board.gems),
        }
    }
}

#[pymethods]
impl PyBoard {
    pub fn face_up_cards(&self, tier: Option<usize>) -> PyResult<Vec<PyCard>> {
        if tier.is_some() && tier.unwrap() > 2 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Tier must be 0, 1, or 2",
            ));
        }
        let cards = match tier {
            None => self
                .available_cards
                .iter()
                .flatten()
                .map(|&card_id| PyCard::from_id(card_id))
                .collect(),
            Some(tier) => self.available_cards[tier]
                .iter()
                .map(|&card_id| PyCard::from_id(card_id))
                .collect(),
        };
        Ok(cards)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyTurn {
    #[pyo3(get)]
    pub player_index: usize,
    #[pyo3(get)]
    pub actions: Vec<PyAction>,
}
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyGameHistory {
    turns: Vec<PyTurn>,
}

impl PyGameHistory {
    pub fn from(history: GameHistory) -> Self {
        let turns = history
            .group_by_player()
            .into_iter()
            .map(|turn_sequences| {
                let actions = turn_sequences
                    .iter()
                    .map(|(_, action)| PyAction::from(action.clone()))
                    .filter(|action| action.action_type != PyActionType::Continue)
                    .collect();

                let player_index = turn_sequences[0].0;
                PyTurn {
                    player_index,
                    actions,
                }
            })
            .collect();

        PyGameHistory { turns }
    }
}

#[pymethods]
impl PyGameHistory {
    #[getter]
    pub fn turns(&self) -> Vec<(usize, Vec<PyAction>)> {
        self.turns
            .iter()
            .map(|turn| (turn.player_index, turn.actions.clone()))
            .collect()
    }
}

#[pyfunction]
fn multiply(a: isize, b: isize) -> PyResult<isize> {
    Ok(a * b)
}

#[pymodule]
fn ffi(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(multiply, m)?)?;
    m.add_function(wrap_pyfunction!(run_python_bot, m)?)?;

    m.add_class::<PyClientInfo>()?;
    m.add_class::<PyPlayer>()?;
    m.add_class::<PyActionType>()?;
    m.add_class::<PyGems>()?;
    m.add_class::<PyAction>()?;
    m.add_class::<PyCard>()?;
    m.add_class::<PyNoble>()?;
    m.add_class::<PyGem>()?;
    Ok(())
}

/// A struct for making sure that the bot on the Python side
/// has proper access to the log stream protocol of the library
#[pyclass]
pub struct PyLog {
    log: Log,
}

impl PyLog {
    pub fn new(url: &str, port: u16, client_id: u64) -> Self {
        PyLog {
            log: Log::new(url, port, client_id),
        }
    }
}

/// Expose a method that allows for python-side logging
#[pymethods]
impl PyLog {
    pub fn send(&mut self, message: PyObject) {
        // TODO: can make this even better for the python side by
        // accepting positional args
        let message = message.to_string();
        self.log.send(&message);
    }
}

#[pyfunction]
pub fn run_python_bot(py: Python, bot_class: &PyAny) {
    let args = get_args();
    let port = args.port;
    let url = args.url.unwrap();
    let game_id = args.game_id.unwrap();
    let client_id = args.client_id;

    let url = format!("{}:{}/game/{}/{}",url, port, game_id, client_id);
    let url = Url::parse(&url).unwrap();
    let (mut game_socket, _) = connect(url).expect("Can't connect to the game server");

    // Give the server a chance to start up
    std::thread::sleep(std::time::Duration::from_millis(100));

    let py_log = PyCell::new(py, PyLog::new(port)).unwrap();

    let bot_instance = bot_class
        .call1((py_log.try_borrow_mut().unwrap(),))
        .expect("Unable to launch bot, could not call __init__");

    loop {
        let msg = game_socket.read();
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => {
                break;
            }
        };
        let msg = msg.to_text().expect("Error converting message to text");
        let message: ServerMessage = serde_json::from_str(msg).expect("Error parsing message");

        if let ServerMessage::PlayerActionRequest(info) = message { 
            let py_info = PyClientInfo::from_client_info(info);
            let result =
                bot_instance.call_method1("take_action", (py_info, py_log.try_borrow_mut().unwrap()));
            let py_action: PyAction = result
                .expect("Error when calling method take_action()")
                .extract()
                .expect("Incorrect type returned by method take_action()");

            let action = py_action.into_action();

            let msg = ClientMessage::Action(action);
            let msg_str = serde_json::to_string(&msg).expect("Error converting action to string");
            let game_send_result = game_socket.send(Message::Text(msg_str));
            if game_send_result.is_err() {
                break;
            }
        } else {
            // TODO: handle broadcasts
        }
    }
}

// TODO: Clean up and make sure equality checking is not referential equality (python default) but instead value equality
