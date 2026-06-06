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
pub struct BagCounts {
    pub water: u16,
    pub mountain: u16,
    pub trunk: u16,
    pub foliage: u16,
    pub field: u16,
    pub building: u16,
    pub unknown: u16,
}

impl BagCounts {
    pub fn get(&self, color: Color) -> u16 {
        match color {
            Color::Water => self.water,
            Color::Mountain => self.mountain,
            Color::Trunk => self.trunk,
            Color::Foliage => self.foliage,
            Color::Field => self.field,
            Color::Building => self.building,
        }
    }

    pub fn total_known(&self) -> u16 {
        self.water + self.mountain + self.trunk + self.foliage + self.field + self.building
    }

    pub fn is_empty(&self) -> bool {
        self.total_known() == 0 && self.unknown == 0
    }

    pub fn saturating_sub_color(&mut self, color: Color) {
        match color {
            Color::Water => self.water = self.water.saturating_sub(1),
            Color::Mountain => self.mountain = self.mountain.saturating_sub(1),
            Color::Trunk => self.trunk = self.trunk.saturating_sub(1),
            Color::Foliage => self.foliage = self.foliage.saturating_sub(1),
            Color::Field => self.field = self.field.saturating_sub(1),
            Color::Building => self.building = self.building.saturating_sub(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Stack {
    pub tokens: [Color; 3],
    pub height: u8,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            tokens: [Color::Water; 3],
            height: 0,
        }
    }
}

impl Serialize for Stack {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Stack", 1)?;
        state.serialize_field("tokens", self.as_slice())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Stack {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StackHelper {
            tokens: Vec<Color>,
        }
        let helper = StackHelper::deserialize(deserializer)?;
        let mut stack = Stack::default();
        for token in helper.tokens {
            stack.push(token);
        }
        Ok(stack)
    }
}

impl Stack {
    pub fn height(&self) -> usize {
        self.height as usize
    }

    pub fn top(&self) -> Option<Color> {
        if self.height > 0 {
            Some(self.tokens[self.height as usize - 1])
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.height == 0
    }

    pub fn as_slice(&self) -> &[Color] {
        &self.tokens[..self.height as usize]
    }

    pub fn push(&mut self, color: Color) {
        if (self.height as usize) < 3 {
            self.tokens[self.height as usize] = color;
            self.height += 1;
        }
    }

    pub fn pop(&mut self) -> Option<Color> {
        if self.height > 0 {
            let val = self.tokens[self.height as usize - 1];
            self.height -= 1;
            Some(val)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub coord: Coord,
    pub stack: Stack,
    pub locked_by_cube: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
    #[serde(default)]
    pub spirit_card_choices: Vec<ActiveCard>,
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
    #[serde(default)]
    pub bag_counts: BagCounts,
    pub cards_catalog_version: String,
}

impl Default for Coord {
    fn default() -> Self {
        Self { col: 0, row: 0 }
    }
}
