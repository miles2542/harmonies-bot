use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalWeights {
    pub version: String,
    pub self_score_percent: i32,
    pub opponent_denial_percent: i32,
}

impl Default for EvalWeights {
    fn default() -> Self {
        Self {
            version: "baseline-2026-06-03".into(),
            self_score_percent: 100,
            opponent_denial_percent: 35,
        }
    }
}

impl EvalWeights {
    pub fn utility(&self, score_estimate: i32, opponent_denial_estimate: i32) -> i32 {
        (score_estimate * self.self_score_percent
            + opponent_denial_estimate * self.opponent_denial_percent)
            / 100
    }
}
