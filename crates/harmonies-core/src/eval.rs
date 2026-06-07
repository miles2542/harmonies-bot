use serde::{Deserialize, Serialize};

use crate::{
    cards::CardCatalog,
    model::{BoardSide, Color, PlayerState, BagCounts},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EvalWeights {
    pub version: String,
    pub self_score_percent: i32,
    pub opponent_denial_percent: i32,
    
    // Weight pairs (early/late)
    pub self_score_percent_early: i32,
    pub self_score_percent_late: i32,
    pub opponent_denial_percent_early: i32,
    pub opponent_denial_percent_late: i32,
    
    // Feature weights (early/late)
    pub completion_proximity_early: i32,
    pub completion_proximity_late: i32,
    pub height_variance_early: i32,
    pub height_variance_late: i32,
    pub wasted_height_penalty_early: i32,
    pub wasted_height_penalty_late: i32,
    
    // Spirit offsets and abandonment
    pub spirit_offset_early: i32,
    pub spirit_offset_late: i32,
    pub spirit_abandonment_threshold: i32,
    
    // Denial exponent (scaled by 100, e.g. 120 = 1.2)
    pub denial_exponent: i32,
}

impl Default for EvalWeights {
    fn default() -> Self {
        Self {
            version: "refined-2026-06-07".into(),
            self_score_percent: 100,
            opponent_denial_percent: 35,
            self_score_percent_early: 100,
            self_score_percent_late: 100,
            opponent_denial_percent_early: 20,
            opponent_denial_percent_late: 40,
            completion_proximity_early: 15,
            completion_proximity_late: 5,
            height_variance_early: -5,
            height_variance_late: -10,
            wasted_height_penalty_early: -15,
            wasted_height_penalty_late: -25,
            spirit_offset_early: 10,
            spirit_offset_late: 2,
            spirit_abandonment_threshold: 4,
            denial_exponent: 120,
        }
    }
}

impl EvalWeights {
    pub fn utility(&self, score_estimate: i32, opponent_denial_estimate: i32, t: f64) -> i32 {
        let self_score_percent = morph_weight(
            self.self_score_percent_early as f64,
            self.self_score_percent_late as f64,
            t,
        );
        let p = self.denial_exponent as f64 / 100.0;
        let alpha_max = self.opponent_denial_percent_late as f64;
        let t_clamped = t.clamp(0.0, 1.0);
        let alpha_t = alpha_max * t_clamped.powf(p);
        
        let denial_val = if opponent_denial_estimate > 0 {
            (opponent_denial_estimate as f64).powf(p)
        } else {
            0.0
        };
        
        let val = (score_estimate as f64 * self_score_percent
            + denial_val * alpha_t)
            / 100.0;
        val.round() as i32
    }
}

pub fn calculate_phase_index(
    player: &PlayerState,
    opponents: &[PlayerState],
    bag_counts: &BagCounts,
) -> f64 {
    // 1. Board occupancy progress
    let compute_board_progress = |p: &PlayerState| -> f64 {
        let total_cells = p.cells.len().max(37) as f64;
        let empty = p.empty_hexes as f64;
        if total_cells > 2.0 {
            let progress = (total_cells - empty) / (total_cells - 2.0);
            progress.clamp(0.0, 1.0)
        } else {
            1.0
        }
    };
    
    let player_progress = compute_board_progress(player);
    let opponents_max_progress = opponents
        .iter()
        .map(compute_board_progress)
        .fold(0.0, f64::max);
    let max_board_progress = player_progress.max(opponents_max_progress);
    
    // 2. Bag progress
    let num_players = opponents.len() + 1;
    let initial_bag = match num_players {
        3 => 108.0,
        4 => 138.0,
        _ => 78.0,
    };
    let remaining_bag = (bag_counts.total_known() + bag_counts.unknown) as f64;
    let bag_progress = ((initial_bag - remaining_bag) / initial_bag).clamp(0.0, 1.0);
    
    // 3. Unified index: maximum of the two progresses
    max_board_progress.max(bag_progress)
}

pub fn morph_weight(early: f64, late: f64, t: f64) -> f64 {
    let t_clamped = t.clamp(0.0, 1.0);
    early + (late - early) * t_clamped
}

pub fn calculate_completion_proximity(player: &PlayerState, catalog: &CardCatalog) -> f64 {
    player
        .active_cards
        .iter()
        .map(|card| {
            if let Some(def) = catalog.get(card.type_arg) {
                let total = def.point_locations.len() as f64;
                if total > 0.0 {
                    let progress = (total - card.remaining_cubes as f64).max(0.0);
                    // Add constant bonus of 0.5 to reward holding active cards in hand
                    progress + 0.5
                } else {
                    0.0
                }
            } else {
                0.0
            }
        })
        .sum()
}

pub fn calculate_height_variance(player: &PlayerState) -> f64 {
    if player.cells.is_empty() {
        return 0.0;
    }
    let heights: Vec<f64> = player
        .cells
        .iter()
        .map(|c| c.stack.height() as f64)
        .collect();
    let mean: f64 = heights.iter().sum::<f64>() / heights.len() as f64;
    let variance: f64 = heights
        .iter()
        .map(|h| (h - mean).powi(2))
        .sum::<f64>()
        / heights.len() as f64;
    variance
}

pub fn calculate_wasted_height_penalty(player: &PlayerState) -> f64 {
    let mut wasted = 0.0;
    for cell in &player.cells {
        let slice = cell.stack.as_slice();
        if slice.last() == Some(&Color::Trunk) {
            // Uncapped tree
            wasted += cell.stack.height() as f64;
        } else if slice == &[Color::Building] {
            // Uncapped building
            wasted += 1.0;
        }
    }
    wasted
}

pub fn eval_spirits(
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    t: f64,
) -> f64 {
    let w_spirit_offset = morph_weight(
        weights.spirit_offset_early as f64,
        weights.spirit_offset_late as f64,
        t,
    );
    let thresh = weights.spirit_abandonment_threshold as f64;
    
    player
        .active_cards
        .iter()
        .chain(&player.completed_cards)
        .filter_map(|card| {
            catalog
                .get(card.type_arg)
                .map(|def| (def, card))
        })
        .filter(|(def, _)| def.is_spirit)
        .map(|(def, card)| {
            let potential_score = crate::scoring::score_spirit_logic(player, def.type_arg) as f64;
            if card.remaining_cubes == 0 {
                // Completed
                potential_score + w_spirit_offset
            } else {
                // Incomplete
                let empty_hexes = player.empty_hexes as f64;
                let multiplier = if empty_hexes <= thresh {
                    0.0
                } else if empty_hexes >= 20.0 {
                    1.0
                } else {
                    (empty_hexes - thresh) / (20.0 - thresh)
                };
                // Scale incomplete spirit value by 0.80 to incentivize actual completion
                0.80 * multiplier * (potential_score + w_spirit_offset)
            }
        })
        .sum()
}

pub fn eval_player(
    player: &PlayerState,
    board_side: BoardSide,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    t: f64,
) -> i32 {
    let breakdown = crate::scoring::score_player(player, board_side, catalog);
    let completed_spirits_score = breakdown.spirits;
    let bga_score_no_spirits = breakdown.total() - completed_spirits_score;
    
    let w_self_score = morph_weight(
        weights.self_score_percent_early as f64,
        weights.self_score_percent_late as f64,
        t,
    ) / 100.0;
    
    let completion_prox = calculate_completion_proximity(player, catalog);
    let height_var = calculate_height_variance(player);
    let wasted_height = calculate_wasted_height_penalty(player);
    
    let w_completion_prox = morph_weight(
        weights.completion_proximity_early as f64,
        weights.completion_proximity_late as f64,
        t,
    );
    let w_height_var = morph_weight(
        weights.height_variance_early as f64,
        weights.height_variance_late as f64,
        t,
    );
    let w_wasted_height = morph_weight(
        weights.wasted_height_penalty_early as f64,
        weights.wasted_height_penalty_late as f64,
        t,
    );
    
    let spirits_eval = eval_spirits(player, catalog, weights, t);
    
    let score_val = bga_score_no_spirits as f64 * w_self_score
        + completion_prox * w_completion_prox
        + height_var * w_height_var
        + wasted_height * w_wasted_height
        + spirits_eval;
        
    score_val.round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cell, Coord, Stack};

    fn create_test_player(empty_hexes: u8, cells: Vec<Cell>) -> PlayerState {
        PlayerState {
            player_id: "test".into(),
            cells,
            active_cards: Vec::new(),
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes,
        }
    }

    #[test]
    fn test_phase_index_board_occupancy() {
        let mut cells = vec![];
        for i in 0..37 {
            cells.push(Cell {
                coord: Coord { col: i, row: 0 },
                stack: Stack::default(),
                locked_by_cube: false,
            });
        }
        let player = create_test_player(37, cells.clone());
        let opponents = vec![];
        let mut full_bag = BagCounts::default();
        full_bag.water = 78;
        
        let t = calculate_phase_index(&player, &opponents, &full_bag);
        assert_eq!(t, 0.0);
        
        let player_near_end = create_test_player(2, cells);
        let t_near_end = calculate_phase_index(&player_near_end, &opponents, &full_bag);
        assert_eq!(t_near_end, 1.0);
    }

    #[test]
    fn test_morph_weight() {
        assert_eq!(morph_weight(10.0, 20.0, 0.0), 10.0);
        assert_eq!(morph_weight(10.0, 20.0, 0.5), 15.0);
        assert_eq!(morph_weight(10.0, 20.0, 1.0), 20.0);
    }
}
