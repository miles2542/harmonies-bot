use serde_json::json;

use super::*;

#[test]
fn normalizes_observed_tutorial_shape() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [{"col": 0, "row": 0}, {"col": 1, "row": 0}],
        "gamestate": {"active_player": "p1"},
        "remainingTokens": 115,
        "players": {
            "p1": {
                "emptyHexes": 1,
                "tokensOnBoard": {
                    "cell_p1_0_0": [
                        {"location_arg": 1, "type_arg": 3},
                        {"location_arg": 2, "type_arg": 4}
                    ]
                },
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {
            "1": [{"type_arg": 2}, {"type_arg": 1}, {"type_arg": 4}]
        },
        "river": [{"id": 8, "type_arg": 22, "pointLocations": [3, 6, 10, 15], "isSpirit": false}],
        "spiritsCards": [],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert_eq!(snapshot.board_side, BoardSide::SideA);
    assert_eq!(
        snapshot.players[0].cells[0].stack.as_slice(),
        &[Color::Trunk, Color::Foliage]
    );
    assert_eq!(
        snapshot.central_token_groups[0],
        vec![Color::Mountain, Color::Water, Color::Foliage]
    );
    assert_eq!(snapshot.river_cards[0].type_arg, 22);
    assert_eq!(snapshot.bag_counts.trunk, 20);
    assert_eq!(snapshot.bag_counts.foliage, 17);
    assert_eq!(snapshot.bag_counts.mountain, 22);
    assert_eq!(snapshot.bag_counts.water, 22);
    assert_eq!(snapshot.bag_counts.unknown, 0);
}

#[test]
fn maps_anonymized_player_key_to_numeric_cell_prefix() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideB",
        "hexes": [{"col": 2, "row": 2}],
        "playerorder": [97479253],
        "gamestate": {"active_player": 97479253},
        "players": {
            "player_1": {
                "id": "player_1",
                "playerNo": 1,
                "emptyHexes": 0,
                "tokensOnBoard": {
                    "cell_97479253_2_2": [
                        {"location_arg": 2, "type_arg": 4},
                        {"location_arg": 1, "type_arg": 3}
                    ]
                },
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, None).unwrap();
    assert_eq!(snapshot.active_player_id, "player_1");
    assert_eq!(
        snapshot.players[0].cells[0].stack.as_slice(),
        &[Color::Trunk, Color::Foliage]
    );
}

#[test]
fn locks_cells_from_player_animal_cubes_on_board() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideB",
        "hexes": [{"col": 1, "row": 0}],
        "playerorder": [97479253],
        "gamestate": {"active_player": 97479253},
        "players": {
            "player_1": {
                "id": "player_1",
                "playerNo": 1,
                "emptyHexes": 0,
                "animalCubesOnBoard": ["cell_97479253_1_0"],
                "tokensOnBoard": {
                    "cell_97479253_1_0": [
                        {"location_arg": 1, "type_arg": 5}
                    ]
                },
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, None).unwrap();
    assert!(snapshot.players[0].cells[0].locked_by_cube);
}

#[test]
fn locks_cells_when_player_no_maps_second_order_entry() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideB",
        "hexes": [{"col": 1, "row": 0}],
        "playerorder": [98885479, 97479253],
        "gamestate": {"active_player": 98885479},
        "players": {
            "player_1": {
                "id": "player_1",
                "playerNo": 2,
                "emptyHexes": 0,
                "animalCubesOnBoard": ["cell_97479253_1_0"],
                "tokensOnBoard": {
                    "cell_97479253_1_0": [
                        {"location_arg": 1, "type_arg": 5}
                    ]
                },
                "boardAnimalCards": [],
                "doneAnimalCards": []
            },
            "player_2": {
                "id": "player_2",
                "playerNo": 1,
                "emptyHexes": 0,
                "animalCubesOnBoard": [],
                "tokensOnBoard": {},
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, None).unwrap();
    let player = snapshot
        .players
        .iter()
        .find(|player| player.player_id == "player_1")
        .unwrap();
    assert!(player.cells[0].locked_by_cube);
}

#[test]
fn player_location_ids_override_conflicting_player_order() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [{"col": 0, "row": 0}],
        "playerorder": [111, 222],
        "gamestate": {"active_player": "player_1"},
        "players": {
            "player_1": {
                "id": "player_1",
                "playerNo": 1,
                "emptyHexes": 0,
                "tokensOnBoard": {
                    "cell_222_0_0": [{"location_arg": 1, "type_arg": 5}]
                },
                "boardAnimalCards": [{"id": 10, "type_arg": 1, "pointLocations": [2]}],
                "doneAnimalCards": []
            },
            "player_2": {
                "id": "player_2",
                "playerNo": 2,
                "emptyHexes": 0,
                "tokensOnBoard": {
                    "cell_111_0_0": [{"location_arg": 1, "type_arg": 1}]
                },
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": [{"location": "card_10"}]
    });
    let snapshot = normalize_gamedatas(&raw, None).unwrap();
    let player_1 = snapshot
        .players
        .iter()
        .find(|player| player.player_id == "player_1")
        .unwrap();
    let player_2 = snapshot
        .players
        .iter()
        .find(|player| player.player_id == "player_2")
        .unwrap();
    assert_eq!(player_1.cells[0].stack.top(), Some(Color::Field));
    assert_eq!(player_2.cells[0].stack.top(), Some(Color::Water));
}

#[test]
fn active_card_remaining_cubes_come_from_card_cube_locations() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "gamestate": {
            "active_player": "p1",
            "args": {"canChooseSpirit": true},
            "possibleactions": ["actChooseSpirit"]
        },
        "players": {
            "p1": {
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [
                    {"id": 31, "type_arg": 21, "location_arg": 2, "pointLocations": [4, 10, 16]}
                ],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": [
            {"location": "card_31"},
            {"location": "card_31"},
            {"location": "card_31"}
        ]
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert_eq!(snapshot.players[0].active_cards[0].remaining_cubes, 3);
}

#[test]
fn unchosen_spirit_offer_is_separate_from_active_cards() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "gamestate": {
            "active_player": "p1",
            "args": {"canChooseSpirit": true},
            "possibleactions": ["actChooseSpirit"]
        },
        "players": {
            "p1": {
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [
            {"id": 19, "type_arg": 38, "location_arg": "p1", "isSpirit": true},
            {"id": 24, "type_arg": 41, "location_arg": "p1", "isSpirit": true}
        ],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert!(snapshot.players[0].active_cards.is_empty());
    assert_eq!(snapshot.players[0].spirit_card_choices.len(), 2);
    assert_eq!(
        snapshot.players[0].spirit_card_choices[0].remaining_cubes,
        1
    );
}

#[test]
fn chosen_spirit_with_cube_is_active_card() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "gamestate": {"active_player": "p1"},
        "players": {
            "p1": {
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [
            {"id": 19, "type_arg": 38, "location_arg": "p1", "isSpirit": true}
        ],
        "cubesOnAnimalCards": [{"location": "card_19"}]
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert_eq!(snapshot.players[0].active_cards.len(), 1);
    assert_eq!(snapshot.players[0].active_cards[0].type_arg, 38);
    assert!(snapshot.players[0].spirit_card_choices.is_empty());
}

#[test]
fn stale_no_cube_spirit_offer_is_ignored_after_choice_window() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "gamestate": {
            "active_player": "p1",
            "args": {"canChooseSpirit": false},
            "possibleactions": ["actChooseSpirit"]
        },
        "players": {
            "p1": {
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [
            {"id": 19, "type_arg": 38, "location_arg": "p1", "isSpirit": true}
        ],
        "cubesOnAnimalCards": []
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert!(snapshot.players[0].active_cards.is_empty());
    assert!(snapshot.players[0].spirit_card_choices.is_empty());
}

#[test]
fn selected_spirit_in_hand_is_active_without_top_level_spirits() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "gamestate": {"active_player": "p1"},
        "players": {
            "p1": {
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [
                    {
                        "id": 24,
                        "type_arg": 41,
                        "location": "boardp1",
                        "pointLocations": [0],
                        "isSpirit": true
                    }
                ],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": [{"location": "card_24"}]
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    assert_eq!(snapshot.players[0].active_cards.len(), 1);
    assert_eq!(snapshot.players[0].active_cards[0].type_arg, 41);
    assert!(snapshot.players[0].active_cards[0].is_spirit);
}

#[test]
fn active_and_completed_cards_must_belong_to_that_player_location() {
    let raw = json!({
        "version": "230603",
        "boardSide": "sideA",
        "hexes": [],
        "playerorder": [98395045, 9],
        "gamestate": {"active_player": 98395045},
        "players": {
            "p1": {
                "id": "p1",
                "playerNo": 1,
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [
                    {"id": 1, "type_arg": 8, "location": "board98395045", "pointLocations": [4]},
                    {"id": 2, "type_arg": 9, "location": "board9", "pointLocations": [4]}
                ],
                "doneAnimalCards": [
                    {"id": 3, "type_arg": 10, "location": "done98395045", "pointLocations": [4]},
                    {"id": 4, "type_arg": 11, "location": "done9", "pointLocations": [4]}
                ]
            },
            "p2": {
                "id": "p2",
                "playerNo": 2,
                "emptyHexes": 0,
                "tokensOnBoard": {},
                "boardAnimalCards": [],
                "doneAnimalCards": []
            }
        },
        "tokensOnCentralBoard": {},
        "river": [],
        "spiritsCards": [],
        "cubesOnAnimalCards": [{"location": "card_2"}]
    });
    let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
    let player = snapshot
        .players
        .iter()
        .find(|player| player.player_id == "p1")
        .unwrap();
    assert_eq!(player.active_cards.len(), 1);
    assert_eq!(player.active_cards[0].card_id, 1);
    assert_eq!(player.completed_cards.len(), 1);
    assert_eq!(player.completed_cards[0].card_id, 3);
}
