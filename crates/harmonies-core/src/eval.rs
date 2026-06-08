use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::{
    cards::{CardCatalog, pattern_cells, stack_matches_colors},
    model::{BoardSide, Color, PlayerState, BagCounts},
};

static HEURISTIC_MODE: OnceLock<String> = OnceLock::new();
static SPIRIT_PROX_MULT: OnceLock<f64> = OnceLock::new();
static SPIRIT_PENALTY_COEFF: OnceLock<f64> = OnceLock::new();
static COMPENSATION_COEFF: OnceLock<f64> = OnceLock::new();

pub fn get_heuristic_mode() -> &'static str {
    HEURISTIC_MODE.get_or_init(|| {
        std::env::var("HARMONIES_HEURISTIC_MODE")
            .unwrap_or_else(|_| "baseline".to_string())
    })
}

pub fn get_spirit_prox_mult() -> f64 {
    *SPIRIT_PROX_MULT.get_or_init(|| {
        std::env::var("HARMONIES_SPIRIT_PROX_MULT")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.5)
    })
}

pub fn get_spirit_penalty_coeff() -> f64 {
    *SPIRIT_PENALTY_COEFF.get_or_init(|| {
        std::env::var("HARMONIES_SPIRIT_PENALTY_COEFF")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(-2.5)
    })
}

pub fn get_compensation_coeff() -> f64 {
    *COMPENSATION_COEFF.get_or_init(|| {
        std::env::var("HARMONIES_COMPENSATION_COEFF")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    })
}

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
    let mode = get_heuristic_mode();
    player
        .active_cards
        .iter()
        .chain(&player.completed_cards)
        .map(|card| {
            if let Some(def) = catalog.get(card.type_arg) {
                if def.is_spirit {
                    // Spirits are handled by eval_spirits, just give a flat hand bonus
                    0.5
                } else {
                    let total = def.point_locations.len() as f64;
                    if total > 0.0 {
                        let progress = (total - card.remaining_cubes as f64).max(0.0);
                        let base = progress + 0.5;
                        if mode == "h2_soft_relaxed_spirits_animal_scaling" {
                            let max_vp = *def.point_locations.last().unwrap_or(&0) as f64;
                            let multiplier = 1.0 + (max_vp - 10.0) * 0.03;
                            base * multiplier
                        } else {
                            base
                        }
                    } else {
                        0.0
                    }
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

pub fn calculate_wasted_height_penalty(
    player: &PlayerState,
    exempt_coords: &std::collections::HashSet<crate::model::Coord>,
) -> f64 {
    let mut wasted = 0.0;
    for cell in &player.cells {
        if exempt_coords.contains(&cell.coord) {
            continue;
        }
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
    let mode = get_heuristic_mode();
    
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
                let multiplier = if mode == "h2_soft_relaxed_spirits"
                    || mode == "h2_soft_relaxed_spirits_animal_scaling"
                    || mode == "dynamic_demand"
                    || mode == "dynamic_demand_space_penalty"
                    || mode == "dynamic_demand_space_hand_penalty"
                    || mode == "dynamic_demand_clog_free"
                    || mode == "dynamic_demand_spirit_guided"
                    || mode == "dynamic_demand_spirit_guided_clog"
                    || mode == "dynamic_demand_spirit_guided_clog_space"
                    || mode == "dynamic_demand_tuned"
                {
                    let start_drop = (thresh + 6.0).max(10.0);
                    if empty_hexes <= thresh {
                        0.0
                    } else if empty_hexes >= start_drop {
                        1.0
                    } else {
                        (empty_hexes - thresh) / (start_drop - thresh)
                    }
                } else {
                    if empty_hexes <= thresh {
                        0.0
                    } else if empty_hexes >= 20.0 {
                        1.0
                    } else {
                        (empty_hexes - thresh) / (20.0 - thresh)
                    }
                };
                // Scale incomplete spirit value by 0.80 to incentivize actual completion
                let scale = if mode == "dynamic_demand_tuned" {
                    0.40
                } else {
                    0.80
                };
                scale * multiplier * (potential_score + w_spirit_offset)
            }
        })
        .sum()
}

pub fn calculate_color_demands(
    player: &PlayerState,
    catalog: &CardCatalog,
) -> std::collections::HashMap<Color, f64> {
    let mut demands = std::collections::HashMap::new();
    for card in &player.active_cards {
        if let Some(def) = catalog.get(card.type_arg) {
            for step in &def.pattern {
                for &raw_color in &step.colors {
                    if let Some(color) = Color::from_bga_type_arg(raw_color) {
                        *demands.entry(color).or_insert(0.0) += 1.0;
                    }
                }
            }
        }
    }
    demands
}

pub fn calculate_landscape_soft_heuristics(
    player: &PlayerState,
    demands: &std::collections::HashMap<Color, f64>,
) -> f64 {
    let mut score = 0.0;
    for cell in &player.cells {
        let slice = cell.stack.as_slice();
        for token in slice {
            let demand = demands.get(token).copied().unwrap_or(0.0);
            let soft_val = 0.05 + demand * 0.05;
            score += soft_val;
        }
    }
    score
}

#[derive(Clone, Debug)]
pub struct SpiritProximityInfo {
    pub total_proximity: f64,
    pub candidate_coords: std::collections::HashSet<crate::model::Coord>,
}

pub fn calculate_spirit_proximity(
    player: &PlayerState,
    catalog: &CardCatalog,
) -> SpiritProximityInfo {
    let cells_by_coord: std::collections::HashMap<crate::model::Coord, &crate::model::Cell> =
        player.cells.iter().map(|cell| (cell.coord, cell)).collect();
        
    let mut total_proximity = 0.0;
    let mut candidate_coords = std::collections::HashSet::new();
    
    for card in player.active_cards.iter().chain(&player.completed_cards) {
        if card.is_spirit {
            if card.remaining_cubes == 0 {
                total_proximity += 1.0;
            } else if let Some(definition) = catalog.get(card.type_arg) {
                let mut max_progress = 0;
                let mut best_coords = Vec::new();
                let pattern_len = definition.pattern.len();
                if pattern_len > 0 {
                    for origin in player.cells.iter().map(|c| c.coord) {
                        for rotation in 0..6 {
                            let coords = pattern_cells(origin, &definition.pattern, rotation);
                            let mut matched_steps = 0;
                            for (coord, step) in coords.iter().zip(&definition.pattern) {
                                if let Some(cell) = cells_by_coord.get(coord) {
                                    if stack_matches_colors(&cell.stack, &step.colors) {
                                        matched_steps += 1;
                                    }
                                }
                            }
                            if matched_steps > max_progress {
                                max_progress = matched_steps;
                                best_coords = coords.clone();
                            }
                        }
                    }
                    total_proximity += max_progress as f64 / pattern_len as f64;
                    candidate_coords.extend(best_coords);
                }
            }
        }
    }
    
    SpiritProximityInfo {
        total_proximity,
        candidate_coords,
    }
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
    
    let mode = get_heuristic_mode();
    let spirit_prox_info = calculate_spirit_proximity(player, catalog);
    let mut completion_prox = calculate_completion_proximity(player, catalog);
    if mode == "dynamic_demand_tuned" {
        completion_prox += spirit_prox_info.total_proximity * get_spirit_prox_mult();
    }
    let height_var = calculate_height_variance(player);
    let wasted_height = calculate_wasted_height_penalty(player, &spirit_prox_info.candidate_coords);
    
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
    let landscape_soft = if mode != "baseline" {
        let mut demands = if mode == "dynamic_demand"
            || mode == "dynamic_demand_space_penalty"
            || mode == "dynamic_demand_space_hand_penalty"
            || mode == "dynamic_demand_clog_free"
            || mode == "dynamic_demand_spirit_guided"
            || mode == "dynamic_demand_spirit_guided_clog"
            || mode == "dynamic_demand_spirit_guided_clog_space"
            || mode == "dynamic_demand_tuned"
        {
            calculate_color_demands(player, catalog)
        } else {
            // Mock demands to match static H2 soft values: Mountain=0.3, Water=0.4, Field=0.2
            let mut map = std::collections::HashMap::new();
            map.insert(Color::Mountain, 5.0);
            map.insert(Color::Water, 7.0);
            map.insert(Color::Field, 3.0);
            map
        };

        if mode == "dynamic_demand_spirit_guided"
            || mode == "dynamic_demand_spirit_guided_clog"
            || mode == "dynamic_demand_spirit_guided_clog_space"
            || mode == "dynamic_demand_tuned"
        {
            for card in &player.active_cards {
                if card.is_spirit {
                    match card.type_arg {
                        33 | 34 => {
                            *demands.entry(Color::Field).or_insert(0.0) += 8.0;
                        }
                        35 | 36 => {
                            *demands.entry(Color::Trunk).or_insert(0.0) += 4.0;
                            *demands.entry(Color::Foliage).or_insert(0.0) += 4.0;
                        }
                        37 | 38 => {
                            *demands.entry(Color::Building).or_insert(0.0) += 8.0;
                        }
                        39 | 40 => {
                            *demands.entry(Color::Mountain).or_insert(0.0) += 8.0;
                        }
                        41 | 42 => {
                            *demands.entry(Color::Water).or_insert(0.0) += 8.0;
                        }
                        _ => {}
                    }
                }
            }
        }

        calculate_landscape_soft_heuristics(player, &demands)
    } else {
        0.0
    };
    
    let space_penalty = if (mode == "dynamic_demand_space_penalty"
        || mode == "dynamic_demand_space_hand_penalty"
        || mode == "dynamic_demand_spirit_guided_clog_space"
        || mode == "dynamic_demand_tuned")
        && t > 0.8
    {
        (t - 0.8) * player.empty_hexes as f64 * -5.0
    } else {
        0.0
    };

    let hand_penalty = if mode == "dynamic_demand_space_hand_penalty" {
        player.active_cards.iter().map(|card| {
            if card.is_spirit {
                0.0
            } else {
                card.remaining_cubes as f64 * -0.3
            }
        }).sum::<f64>()
    } else if mode == "dynamic_demand_clog_free"
        || mode == "dynamic_demand_spirit_guided_clog"
        || mode == "dynamic_demand_spirit_guided_clog_space"
        || mode == "dynamic_demand_tuned"
    {
        let cubes_penalty: f64 = player.active_cards.iter().map(|card| {
            if card.is_spirit {
                0.0
            } else {
                card.remaining_cubes as f64 * -0.3
            }
        }).sum();
        let active_count = player.active_cards.iter().filter(|c| !c.is_spirit).count();
        let count_penalty = if active_count >= 3 {
            -10.0
        } else if active_count >= 2 {
            -3.0
        } else {
            0.0
        };
        cubes_penalty + count_penalty
    } else {
        0.0
    };
    
    let building_penalty = if mode == "dynamic_demand_tuned" {
        let building_count = player
            .cells
            .iter()
            .filter(|c| c.stack.top() == Some(Color::Building))
            .count() as f64;
        building_count * -1.2
    } else {
        0.0
    };

    let spirit_incomplete_penalty = if mode == "dynamic_demand_tuned" {
        player.active_cards.iter().map(|card| {
            if card.is_spirit && card.remaining_cubes > 0 {
                let filled_hexes = 25.0 - player.empty_hexes as f64;
                if filled_hexes >= 12.0 {
                    // Turn 4+ (12+ tokens)
                    -40.0 - (filled_hexes - 12.0) * 5.0
                } else if filled_hexes >= 9.0 {
                    // Turn 3 (9 tokens)
                    -15.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        }).sum::<f64>()
    } else {
        0.0
    };

    let compensation_penalty = if mode == "dynamic_demand_tuned" {
        let target_animals = 35.0 * t;
        let target_spirits = 15.0 * t;
        
        let current_animals = (breakdown.animals as f64) + calculate_completion_proximity(player, catalog);
        let current_spirits = spirit_prox_info.total_proximity * 15.0;
        
        let def_a = (target_animals - current_animals).max(0.0);
        let def_s = (target_spirits - current_spirits).max(0.0);
        
        def_a * def_s * get_compensation_coeff()
    } else {
        0.0
    };

    let score_val = bga_score_no_spirits as f64 * w_self_score
        + completion_prox * w_completion_prox
        + height_var * w_height_var
        + wasted_height * w_wasted_height
        + spirits_eval
        + landscape_soft
        + space_penalty
        + hand_penalty
        + building_penalty
        + spirit_incomplete_penalty
        + compensation_penalty;
        
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
