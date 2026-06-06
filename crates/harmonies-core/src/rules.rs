use thiserror::Error;

use crate::model::{Cell, Color, Stack};

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PlacementError {
    #[error("cell locked by cube")]
    Locked,
    #[error("stack height already 3")]
    MaxHeight,
    #[error("token cannot be placed on this stack")]
    IllegalStack,
}

pub fn can_place(cell: &Cell, color: Color) -> Result<(), PlacementError> {
    if cell.locked_by_cube {
        return Err(PlacementError::Locked);
    }
    if cell.stack.height() >= 3 {
        return Err(PlacementError::MaxHeight);
    }
    if is_legal_stack_after_place(&cell.stack, color) {
        Ok(())
    } else {
        Err(PlacementError::IllegalStack)
    }
}

pub fn is_legal_stack_after_place(stack: &Stack, color: Color) -> bool {
    match color {
        Color::Mountain => stack.tokens.iter().all(|token| *token == Color::Mountain),
        Color::Trunk => {
            stack.height() < 2 && stack.tokens.iter().all(|token| *token == Color::Trunk)
        }
        Color::Foliage => {
            stack.is_empty() || stack.tokens.iter().all(|token| *token == Color::Trunk)
        }
        Color::Building => {
            stack.height() < 2
                && matches!(
                    stack.top(),
                    None | Some(Color::Trunk | Color::Mountain | Color::Building)
                )
        }
        Color::Field | Color::Water => stack.is_empty(),
    }
}

pub fn place_token(cell: &mut Cell, color: Color) -> Result<(), PlacementError> {
    can_place(cell, color)?;
    cell.stack.tokens.push(color);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::model::{Coord, Stack};

    use super::*;

    fn cell(tokens: Vec<Color>) -> Cell {
        Cell {
            coord: Coord { col: 0, row: 0 },
            stack: Stack { tokens },
            locked_by_cube: false,
        }
    }

    #[test]
    fn foliage_can_cap_one_or_two_trunks() {
        assert!(can_place(&cell(vec![Color::Trunk]), Color::Foliage).is_ok());
        assert!(can_place(&cell(vec![Color::Trunk, Color::Trunk]), Color::Foliage).is_ok());
    }

    #[test]
    fn water_and_field_only_empty() {
        assert_eq!(
            can_place(&cell(vec![Color::Field]), Color::Water),
            Err(PlacementError::IllegalStack)
        );
        assert!(can_place(&cell(vec![]), Color::Water).is_ok());
    }

    #[test]
    fn building_can_start_empty_and_stack_on_support() {
        assert!(can_place(&cell(vec![]), Color::Building).is_ok());
        assert!(can_place(&cell(vec![Color::Mountain]), Color::Building).is_ok());
        assert_eq!(
            can_place(&cell(vec![Color::Field]), Color::Building),
            Err(PlacementError::IllegalStack)
        );
    }

    #[test]
    fn building_on_two_height_stack_is_illegal() {
        assert_eq!(
            can_place(&cell(vec![Color::Mountain, Color::Building]), Color::Building),
            Err(PlacementError::IllegalStack)
        );
    }
}
