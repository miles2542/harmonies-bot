use serde::{Deserialize, Serialize};

use crate::{
    advisor::MovePlanV1,
    model::{ActiveCard, BagCounts, Color, PlayerState},
    scoring::ScoreBreakdown,
    turn::TurnSequence,
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProgress {
    pub depth_completed: usize,
    pub nodes_evaluated: usize,
    pub root_generation_ms: u64,
    pub root_sequences_generated: usize,
    pub stopped_early: bool,
}

#[derive(Clone, Debug)]
pub struct SearchOutcome {
    pub plans: Vec<MovePlanV1>,
    pub progress: SearchProgress,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub(super) struct RootCandidate {
    pub(super) group_index: usize,
    pub(super) tokens: Vec<Color>,
    pub(super) turn: TurnSequence,
    pub(super) immediate: ScoreBreakdown,
    pub(super) future_estimate: i32,
    pub(super) utility_estimate: i32,
    pub(super) opponent_denial_estimate: i32,
}

#[derive(Clone)]
pub(super) struct FutureState {
    pub(super) player: PlayerState,
    pub(super) central_groups: Vec<Vec<Color>>,
    pub(super) river_cards: Vec<ActiveCard>,
    pub(super) unseen_cards: Vec<u8>,
    pub(super) bag_counts: BagCounts,
    pub(super) score: i32,
}
