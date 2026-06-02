use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coord {
    pub col: i8,
    pub row: i8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BoardSide {
    SideA,
    SideB,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    Water,
    Mountain,
    Trunk,
    Foliage,
    Field,
    Building,
}

impl Color {
    pub fn from_bga_type_arg(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Water),
            2 => Some(Self::Mountain),
            3 => Some(Self::Trunk),
            4 => Some(Self::Foliage),
            5 => Some(Self::Field),
            6 | 7 => Some(Self::Building),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stack {
    pub tokens: Vec<Color>,
}

impl Stack {
    pub fn height(&self) -> usize {
        self.tokens.len()
    }

    pub fn top(&self) -> Option<Color> {
        self.tokens.last().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub coord: Coord,
    pub stack: Stack,
    pub locked_by_cube: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveCard {
    pub card_id: u32,
    pub type_arg: u8,
    pub remaining_cubes: u8,
    pub is_spirit: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub player_id: String,
    pub cells: Vec<Cell>,
    pub active_cards: Vec<ActiveCard>,
    pub completed_cards: Vec<ActiveCard>,
    pub empty_hexes: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshotV1 {
    pub schema_version: u8,
    pub perspective_player_id: String,
    pub active_player_id: String,
    pub board_side: BoardSide,
    pub players: Vec<PlayerState>,
    pub central_token_groups: Vec<Vec<Color>>,
    pub river_cards: Vec<ActiveCard>,
    pub cards_catalog_version: String,
}

impl Default for Coord {
    fn default() -> Self {
        Self { col: 0, row: 0 }
    }
}
