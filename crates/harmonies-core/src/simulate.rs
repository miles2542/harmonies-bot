use serde::{Deserialize, Serialize};

use crate::{
    advisor::{advise, AdvisorRequestV1, AdvisorStatus, MoveActionV1, MovePlanV1},
    cards::CardCatalog,
    eval::EvalWeights,
    model::{GameSnapshotV1, PlayerState},
    moves::{apply_settlement, SettlementMove},
    rules::place_token,
    scoring::{score_player, ScoreBreakdown},
};

mod deck;

use deck::{draw_color, synthetic_card, SimulationDeck, RIVER_SIZE};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfPlayConfig {
    pub turn_budget_ms: u64,
    pub max_turns: usize,
    pub max_results: usize,
    pub seed: u64,
    pub runtime_mode: String,
    pub scorer_validated: bool,
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        Self {
            turn_budget_ms: 250,
            max_turns: 80,
            max_results: 1,
            seed: 1,
            runtime_mode: "self-play".into(),
            scorer_validated: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfPlayReport {
    pub completed: bool,
    pub turns: Vec<SelfPlayTurnReport>,
    pub final_scores: Vec<SelfPlayPlayerScore>,
    pub final_snapshot: GameSnapshotV1,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfPlayTurnReport {
    pub turn_index: usize,
    pub player_id: String,
    pub advisor_status: AdvisorStatus,
    pub central_group_index: Option<usize>,
    pub score_before: i32,
    pub score_after: i32,
    pub score_delta: i32,
    pub utility_estimate: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfPlayPlayerScore {
    pub player_id: String,
    pub total: i32,
    pub breakdown: ScoreBreakdown,
}

struct ApplyResult {
    bag_empty_triggered: bool,
}

pub fn run_self_play(
    mut snapshot: GameSnapshotV1,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    config: &SelfPlayConfig,
) -> SelfPlayReport {
    let mut warnings = initial_warnings(&snapshot, config);
    let mut turns = Vec::new();
    let mut deck = SimulationDeck::from_snapshot(&snapshot, catalog);
    let mut rng = config.seed.max(1);
    let mut final_round_remaining: Option<usize> = None;
    let mut completed = false;

    for turn_index in 0..config.max_turns {
        let Some(active_index) = active_player_index(&snapshot) else {
            warnings.push("active player missing from players list; self-play stopped".into());
            break;
        };
        let player_id = snapshot.players[active_index].player_id.clone();
        snapshot.perspective_player_id = player_id.clone();
        snapshot.active_player_id = player_id.clone();

        let before = score_player(
            &snapshot.players[active_index],
            snapshot.board_side,
            catalog,
        )
        .total();
        let response = advise(AdvisorRequestV1 {
            snapshot: snapshot.clone(),
            time_budget_ms: config.turn_budget_ms,
            max_results: config.max_results,
            seed: config.seed.wrapping_add(turn_index as u64),
            runtime_mode: config.runtime_mode.clone(),
            catalog: catalog.clone(),
            weights: weights.clone(),
        });
        let plan = response.best_moves.first().cloned();
        if plan.is_none() {
            warnings.push(format!(
                "advisor returned {:?} for {}; self-play stopped",
                response.status, player_id
            ));
        }
        let Some(plan) = plan else {
            break;
        };

        let apply = apply_plan(
            &mut snapshot,
            active_index,
            &plan,
            catalog,
            &mut deck,
            &mut rng,
        );
        warnings.extend(response.warnings);
        let after = score_player(
            &snapshot.players[active_index],
            snapshot.board_side,
            catalog,
        )
        .total();
        turns.push(SelfPlayTurnReport {
            turn_index,
            player_id,
            advisor_status: response.status,
            central_group_index: Some(plan.central_group_index),
            score_before: before,
            score_after: after,
            score_delta: after - before,
            utility_estimate: Some(plan.utility_estimate),
        });

        if final_round_remaining.is_none() && end_triggered(&snapshot.players[active_index], &apply)
        {
            final_round_remaining = Some(snapshot.players.len().saturating_sub(active_index + 1));
        }
        if let Some(remaining) = final_round_remaining {
            if remaining == 0 {
                completed = true;
                break;
            }
            final_round_remaining = Some(remaining - 1);
        }
        advance_active_player(&mut snapshot, active_index);
    }

    SelfPlayReport {
        completed,
        turns,
        final_scores: final_scores(&snapshot, catalog),
        final_snapshot: snapshot,
        warnings,
    }
}

fn apply_plan(
    snapshot: &mut GameSnapshotV1,
    player_index: usize,
    plan: &MovePlanV1,
    catalog: &CardCatalog,
    deck: &mut SimulationDeck,
    rng: &mut u64,
) -> ApplyResult {
    let mut group_index = plan.central_group_index;
    for action in &plan.ordered_actions {
        match action {
            MoveActionV1::TakeGroup {
                group_index: index, ..
            } => group_index = *index,
            MoveActionV1::PlaceToken { token, col, row } => {
                let player = &mut snapshot.players[player_index];
                if let Some(cell) = player
                    .cells
                    .iter_mut()
                    .find(|cell| cell.coord.col == *col && cell.coord.row == *row)
                {
                    let was_empty = cell.stack.is_empty();
                    if place_token(cell, *token).is_ok() && was_empty {
                        player.empty_hexes = player.empty_hexes.saturating_sub(1);
                    }
                }
            }
            MoveActionV1::DraftCard { card_id, type_arg } => {
                draft_card(snapshot, player_index, *card_id, *type_arg, catalog);
            }
            MoveActionV1::ChooseSpirit { card_id, type_arg } => {
                choose_spirit(snapshot, player_index, *card_id, *type_arg);
            }
            MoveActionV1::SettleCard {
                card_id,
                type_arg,
                col,
                row,
            } => {
                let settlement = SettlementMove {
                    card_id: *card_id,
                    type_arg: *type_arg,
                    is_spirit: *type_arg >= 33,
                    cube_coord: crate::Coord {
                        col: *col,
                        row: *row,
                    },
                    pattern_coords: Vec::new(),
                };
                apply_settlement(&mut snapshot.players[player_index], &settlement);
            }
        }
    }
    let bag_empty_triggered = refill_group(snapshot, group_index, rng);
    refill_river(snapshot, catalog, deck, rng);
    ApplyResult {
        bag_empty_triggered,
    }
}

fn choose_spirit(snapshot: &mut GameSnapshotV1, player_index: usize, card_id: u32, type_arg: u8) {
    let player = &mut snapshot.players[player_index];
    let choice = player
        .spirit_card_choices
        .iter()
        .find(|card| card.card_id == card_id)
        .cloned()
        .unwrap_or(crate::ActiveCard {
            card_id,
            type_arg,
            remaining_cubes: 1,
            is_spirit: true,
        });
    player.spirit_card_choices.clear();
    player.active_cards.push(choice);
}

fn draft_card(
    snapshot: &mut GameSnapshotV1,
    player_index: usize,
    card_id: u32,
    type_arg: u8,
    catalog: &CardCatalog,
) {
    let card = snapshot
        .river_cards
        .iter()
        .position(|card| card.card_id == card_id)
        .map(|index| snapshot.river_cards.remove(index))
        .unwrap_or_else(|| synthetic_card(card_id, type_arg, catalog));
    if snapshot.players[player_index].active_cards.len() < 4 {
        snapshot.players[player_index].active_cards.push(card);
    }
}

fn refill_group(snapshot: &mut GameSnapshotV1, group_index: usize, rng: &mut u64) -> bool {
    let mut refill = Vec::new();
    for _ in 0..3 {
        let Some(color) = draw_color(&mut snapshot.bag_counts, rng) else {
            break;
        };
        refill.push(color);
    }
    let bag_empty_triggered = refill.len() < 3;
    if let Some(group) = snapshot.central_token_groups.get_mut(group_index) {
        *group = refill;
    }
    bag_empty_triggered
}

fn refill_river(
    snapshot: &mut GameSnapshotV1,
    catalog: &CardCatalog,
    deck: &mut SimulationDeck,
    rng: &mut u64,
) {
    while snapshot.river_cards.len() < RIVER_SIZE {
        let Some(type_arg) = deck.draw_card(rng) else {
            break;
        };
        let card = synthetic_card(deck.next_card_id(), type_arg, catalog);
        snapshot.river_cards.push(card);
    }
}

fn end_triggered(player: &PlayerState, apply: &ApplyResult) -> bool {
    player.empty_hexes <= 2 || apply.bag_empty_triggered
}

fn active_player_index(snapshot: &GameSnapshotV1) -> Option<usize> {
    snapshot
        .players
        .iter()
        .position(|player| player.player_id == snapshot.active_player_id)
}

fn advance_active_player(snapshot: &mut GameSnapshotV1, active_index: usize) {
    let next_index = (active_index + 1) % snapshot.players.len();
    let next_id = snapshot.players[next_index].player_id.clone();
    snapshot.active_player_id = next_id.clone();
    snapshot.perspective_player_id = next_id;
}

fn final_scores(snapshot: &GameSnapshotV1, catalog: &CardCatalog) -> Vec<SelfPlayPlayerScore> {
    snapshot
        .players
        .iter()
        .map(|player| {
            let breakdown = score_player(player, snapshot.board_side, catalog);
            SelfPlayPlayerScore {
                player_id: player.player_id.clone(),
                total: breakdown.total(),
                breakdown,
            }
        })
        .collect()
}

fn initial_warnings(snapshot: &GameSnapshotV1, config: &SelfPlayConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    if !config.scorer_validated {
        warnings
            .push("self-play scorer has not been validated against Side A 2p BGA finals".into());
    }
    if snapshot.players.len() != 2 {
        warnings
            .push("self-play tuning target is 2-player; fixture has different player count".into());
    }
    warnings
}

#[cfg(test)]
mod tests {
    use crate::model::{BagCounts, BoardSide, Cell, Color, Coord, Stack};

    use super::*;

    fn empty_cell(col: i8) -> Cell {
        Cell {
            coord: Coord { col, row: 0 },
            stack: Stack::default(),
            locked_by_cube: false,
        }
    }

    #[test]
    fn self_play_applies_turn_and_refills_selected_group() {
        let player = |id: &str| PlayerState {
            player_id: id.into(),
            cells: vec![empty_cell(0), empty_cell(1), empty_cell(2), empty_cell(3)],
            active_cards: Vec::new(),
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 4,
        };
        let snapshot = GameSnapshotV1 {
            schema_version: 1,
            perspective_player_id: "p1".into(),
            active_player_id: "p1".into(),
            board_side: BoardSide::SideA,
            players: vec![player("p1"), player("p2")],
            central_token_groups: vec![vec![Color::Water, Color::Field, Color::Mountain]],
            river_cards: Vec::new(),
            bag_counts: BagCounts {
                water: 3,
                mountain: 3,
                field: 3,
                ..BagCounts::default()
            },
            cards_catalog_version: "test".into(),
        };
        let report = run_self_play(
            snapshot,
            &CardCatalog::default(),
            &EvalWeights::default(),
            &SelfPlayConfig {
                max_turns: 1,
                ..SelfPlayConfig::default()
            },
        );
        assert_eq!(report.turns.len(), 1);
        assert_eq!(report.final_snapshot.active_player_id, "p2");
        assert_eq!(report.final_snapshot.central_token_groups[0].len(), 3);
        assert_eq!(report.final_snapshot.players[0].empty_hexes, 1);
    }
}
