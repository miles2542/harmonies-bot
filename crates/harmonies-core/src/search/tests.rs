use crate::{
    model::{ActiveCard, PlayerState},
    turn::{TurnSequence, TurnStep},
};

use super::river_after_turn;

#[test]
fn river_after_turn_removes_drafted_card() {
    let river = vec![
        ActiveCard {
            card_id: 1,
            type_arg: 8,
            remaining_cubes: 4,
            is_spirit: false,
        },
        ActiveCard {
            card_id: 2,
            type_arg: 9,
            remaining_cubes: 4,
            is_spirit: false,
        },
    ];
    let turn = TurnSequence {
        steps: vec![TurnStep::DraftCard {
            card_id: 1,
            type_arg: 8,
        }],
        player: PlayerState {
            player_id: "p1".into(),
            cells: Vec::new(),
            active_cards: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 0,
        },
    };
    assert_eq!(river_after_turn(&river, &turn).len(), 1);
    assert_eq!(river_after_turn(&river, &turn)[0].card_id, 2);
}
