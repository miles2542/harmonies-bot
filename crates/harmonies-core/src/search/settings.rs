use std::env;

#[derive(Clone, Copy, Debug)]
pub(super) struct SearchSettings {
    pub root_turn_beam_width: usize,
    pub future_turn_beam_width: usize,
    pub future_branch_width: usize,
    pub future_depth: usize,
    pub refill_samples: usize,
    pub card_refill_samples: usize,
    pub hard_stop_margin_ms: u64,
    pub min_future_expand_ms: u64,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            root_turn_beam_width: 512,
            future_turn_beam_width: 50,
            future_branch_width: 50,
            future_depth: 4,
            refill_samples: 10,
            card_refill_samples: 4,
            hard_stop_margin_ms: 6_000,
            min_future_expand_ms: 7_000,
        }
    }
}

impl SearchSettings {
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            root_turn_beam_width: env_usize("HARMONIES_ROOT_BEAM", defaults.root_turn_beam_width),
            future_turn_beam_width: env_usize(
                "HARMONIES_FUTURE_BEAM",
                defaults.future_turn_beam_width,
            ),
            future_branch_width: env_usize("HARMONIES_FUTURE_BRANCH", defaults.future_branch_width),
            future_depth: env_usize("HARMONIES_FUTURE_DEPTH", defaults.future_depth),
            refill_samples: env_usize("HARMONIES_REFILL_SAMPLES", defaults.refill_samples),
            card_refill_samples: env_usize(
                "HARMONIES_CARD_REFILL_SAMPLES",
                defaults.card_refill_samples,
            ),
            hard_stop_margin_ms: env_u64(
                "HARMONIES_HARD_STOP_MARGIN_MS",
                defaults.hard_stop_margin_ms,
            ),
            min_future_expand_ms: env_u64(
                "HARMONIES_MIN_FUTURE_EXPAND_MS",
                defaults.min_future_expand_ms,
            ),
        }
    }
}

fn env_usize(key: &str, fallback: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn env_u64(key: &str, fallback: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(fallback)
}
