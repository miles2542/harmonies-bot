use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::{
    cards::CardCatalog,
    model::{Color, GameSnapshotV1},
    scoring::ScoreBreakdown,
    search::{search_current_player_turn, SearchProgress},
    turn::TurnStep,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvisorRequestV1 {
    pub snapshot: GameSnapshotV1,
    pub time_budget_ms: u64,
    pub max_results: usize,
    pub seed: u64,
    pub runtime_mode: String,
    #[serde(default)]
    pub catalog: CardCatalog,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvisorResponseV1 {
    pub status: AdvisorStatus,
    pub elapsed_ms: u128,
    pub best_moves: Vec<MovePlanV1>,
    pub progress: SearchProgress,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdvisorStatus {
    Ready,
    NotParticipantTurn,
    NoLegalMove,
    InvalidSnapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovePlanV1 {
    pub central_group_index: usize,
    pub ordered_actions: Vec<MoveActionV1>,
    pub score_estimate: i32,
    pub score_breakdown: ScoreBreakdown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MoveActionV1 {
    TakeGroup {
        group_index: usize,
        tokens: Vec<Color>,
    },
    PlaceToken {
        token: Color,
        col: i8,
        row: i8,
    },
    SettleCard {
        card_id: u32,
        type_arg: u8,
        col: i8,
        row: i8,
    },
    DraftCard {
        card_id: u32,
        type_arg: u8,
    },
}

pub fn advise(request: AdvisorRequestV1) -> AdvisorResponseV1 {
    let started = Instant::now();
    let mut warnings = Vec::new();

    if request.snapshot.active_player_id != request.snapshot.perspective_player_id {
        return AdvisorResponseV1 {
            status: AdvisorStatus::NotParticipantTurn,
            elapsed_ms: started.elapsed().as_millis(),
            best_moves: Vec::new(),
            progress: SearchProgress::default(),
            warnings,
        };
    }

    let Some(player) = request
        .snapshot
        .players
        .iter()
        .find(|player| player.player_id == request.snapshot.perspective_player_id)
        .cloned()
    else {
        return AdvisorResponseV1 {
            status: AdvisorStatus::InvalidSnapshot,
            elapsed_ms: started.elapsed().as_millis(),
            best_moves: Vec::new(),
            progress: SearchProgress::default(),
            warnings: vec!["perspective player missing from snapshot".into()],
        };
    };

    let outcome = search_current_player_turn(
        &request.snapshot,
        &player,
        &request.catalog,
        request.max_results,
        request.seed,
        request.time_budget_ms,
        started,
    );

    if outcome.plans.is_empty() {
        warnings.push("no legal placement found for any central group".into());
    }
    warnings.extend(outcome.warnings);

    AdvisorResponseV1 {
        status: if outcome.plans.is_empty() {
            AdvisorStatus::NoLegalMove
        } else {
            AdvisorStatus::Ready
        },
        elapsed_ms: started.elapsed().as_millis(),
        best_moves: outcome.plans,
        progress: outcome.progress,
        warnings,
    }
}

pub(crate) fn turn_step_action(step: TurnStep) -> MoveActionV1 {
    match step {
        TurnStep::PlaceToken { token, coord } => MoveActionV1::PlaceToken {
            token,
            col: coord.col,
            row: coord.row,
        },
        TurnStep::DraftCard { card_id, type_arg } => MoveActionV1::DraftCard { card_id, type_arg },
        TurnStep::SettleCard {
            card_id,
            type_arg,
            coord,
        } => MoveActionV1::SettleCard {
            card_id,
            type_arg,
            col: coord.col,
            row: coord.row,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{BagCounts, BoardSide, Cell, Coord, PlayerState, Stack};

    use super::*;

    #[test]
    fn advisor_returns_greedy_legal_plan() {
        let cells = vec![
            Cell {
                coord: Coord { col: 0, row: 0 },
                stack: Stack::default(),
                locked_by_cube: false,
            },
            Cell {
                coord: Coord { col: 1, row: 0 },
                stack: Stack::default(),
                locked_by_cube: false,
            },
            Cell {
                coord: Coord { col: 2, row: 0 },
                stack: Stack::default(),
                locked_by_cube: false,
            },
        ];
        let snapshot = GameSnapshotV1 {
            schema_version: 1,
            perspective_player_id: "p1".into(),
            active_player_id: "p1".into(),
            board_side: BoardSide::SideA,
            players: vec![PlayerState {
                player_id: "p1".into(),
                cells,
                active_cards: Vec::new(),
                completed_cards: Vec::new(),
                empty_hexes: 3,
            }],
            central_token_groups: vec![vec![Color::Field, Color::Field, Color::Water]],
            river_cards: Vec::new(),
            bag_counts: BagCounts::default(),
            cards_catalog_version: "test".into(),
        };
        let response = advise(AdvisorRequestV1 {
            snapshot,
            time_budget_ms: 1000,
            max_results: 1,
            seed: 1,
            runtime_mode: "native".into(),
            catalog: CardCatalog::default(),
        });
        assert_eq!(response.status, AdvisorStatus::Ready);
        assert_eq!(response.best_moves[0].ordered_actions.len(), 4);
    }
}
