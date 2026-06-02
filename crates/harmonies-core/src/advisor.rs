use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::{
    cards::CardCatalog,
    model::{Color, GameSnapshotV1, PlayerState},
    rules::place_token,
    scoring::{score_player, ScoreBreakdown},
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
}

pub fn advise(request: AdvisorRequestV1) -> AdvisorResponseV1 {
    let started = Instant::now();
    let mut warnings = Vec::new();

    if request.snapshot.active_player_id != request.snapshot.perspective_player_id {
        return AdvisorResponseV1 {
            status: AdvisorStatus::NotParticipantTurn,
            elapsed_ms: started.elapsed().as_millis(),
            best_moves: Vec::new(),
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
            warnings: vec!["perspective player missing from snapshot".into()],
        };
    };

    let mut plans: Vec<MovePlanV1> = request
        .snapshot
        .central_token_groups
        .iter()
        .enumerate()
        .filter_map(|(group_index, tokens)| {
            greedy_place_group(&player, tokens, group_index, &request)
        })
        .collect();

    plans.sort_by(|left, right| right.score_estimate.cmp(&left.score_estimate));
    plans.truncate(request.max_results.max(1));

    if plans.is_empty() {
        warnings.push("no legal placement found for any central group".into());
    }

    AdvisorResponseV1 {
        status: if plans.is_empty() {
            AdvisorStatus::NoLegalMove
        } else {
            AdvisorStatus::Ready
        },
        elapsed_ms: started.elapsed().as_millis(),
        best_moves: plans,
        warnings,
    }
}

fn greedy_place_group(
    player: &PlayerState,
    tokens: &[Color],
    group_index: usize,
    request: &AdvisorRequestV1,
) -> Option<MovePlanV1> {
    let mut next_player = player.clone();
    let mut actions = vec![MoveActionV1::TakeGroup {
        group_index,
        tokens: tokens.to_vec(),
    }];

    for token in tokens {
        let Some((cell_index, score)) = best_cell_for_token(&next_player, *token, request) else {
            return None;
        };
        let coord = next_player.cells[cell_index].coord;
        place_token(&mut next_player.cells[cell_index], *token).ok()?;
        actions.push(MoveActionV1::PlaceToken {
            token: *token,
            col: coord.col,
            row: coord.row,
        });
        if score == i32::MIN {
            return None;
        }
    }

    let score_breakdown = score_player(&next_player, request.snapshot.board_side, &request.catalog);
    Some(MovePlanV1 {
        central_group_index: group_index,
        ordered_actions: actions,
        score_estimate: score_breakdown.total(),
        score_breakdown,
    })
}

fn best_cell_for_token(
    player: &PlayerState,
    token: Color,
    request: &AdvisorRequestV1,
) -> Option<(usize, i32)> {
    player
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            let mut candidate = player.clone();
            place_token(&mut candidate.cells[index], token).ok()?;
            let score =
                score_player(&candidate, request.snapshot.board_side, &request.catalog).total();
            Some((index, score))
        })
        .max_by_key(|(_, score)| *score)
}

#[cfg(test)]
mod tests {
    use crate::model::{BoardSide, Cell, Coord, Stack};

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
