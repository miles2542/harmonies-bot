(function harmoniesAdvisorClientModule() {
  const SERVICE_URL = "http://127.0.0.1:17848/advise";
  const SERVICE_TIMEOUT_MS = 50000;
  const COLOR_LABELS = {
    water: "Water",
    mountain: "Mountain",
    trunk: "Trunk",
    foliage: "Foliage",
    field: "Field",
    building: "Building",
  };

  function createAdvisorClient() {
    return {
      async getRecommendation(gamedatas) {
        const snapshot = window.HarmoniesBgaNormalizer.normalizeGamedatas(gamedatas);
        try {
          const response = await requestNativeAdvisor(snapshot);
          return adaptAdvisorResponse(response);
        } catch (error) {
          const mock = window.HarmoniesMockAdvisor.recommend(snapshot);
          mock.status = `Mock advisor active; native service unavailable: ${error.message}`;
          return mock;
        }
      },
    };
  }

  async function requestNativeAdvisor(snapshot) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), SERVICE_TIMEOUT_MS);
    try {
      const response = await fetch(SERVICE_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          snapshot,
          timeBudgetMs: 48000,
          maxResults: 3,
          seed: Date.now(),
          runtimeMode: "native-service",
        }),
        signal: controller.signal,
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      return await response.json();
    } finally {
      clearTimeout(timeout);
    }
  }

  function adaptAdvisorResponse(response) {
    const best = response.bestMoves?.[0] || null;
    const progress = response.progress || {};
    return {
      status: statusText(response, progress),
      bestMove: best
        ? {
            centralGroupId: String(best.centralGroupIndex + 1),
            title: `Take group ${best.centralGroupIndex + 1}; estimate ${best.scoreEstimate} VP`,
            steps: actionSteps(best).concat(scoreSteps(best)),
          }
        : null,
      alternatives: (response.bestMoves || []).slice(1).map((move) => ({
        centralGroupId: String(move.centralGroupIndex + 1),
        label: `Group ${move.centralGroupIndex + 1}: ${move.scoreEstimate} VP`,
      })),
    };
  }

  function statusText(response, progress) {
    const warnings = response.warnings?.length ? `; ${response.warnings.join("; ")}` : "";
    return `${response.status}; ${response.elapsedMs}ms; depth ${progress.depthCompleted || 0}; nodes ${
      progress.nodesEvaluated || 0
    }${warnings}`;
  }

  function actionSteps(move) {
    return (move.orderedActions || []).map((action, index) => {
      const prefix = `${index + 1}.`;
      if (action.kind === "takeGroup") {
        return `${prefix} Take group ${action.groupIndex + 1}: ${action.tokens.map(labelColor).join(", ")}`;
      }
      if (action.kind === "placeToken") {
        return `${prefix} Place ${labelColor(action.token)} at (${action.col}, ${action.row})`;
      }
      if (action.kind === "draftCard") {
        return `${prefix} Draft card ${action.typeArg} (id ${action.cardId})`;
      }
      if (action.kind === "settleCard") {
        return `${prefix} Settle card ${action.typeArg} cube at (${action.col}, ${action.row})`;
      }
      return `${prefix} ${action.kind}`;
    });
  }

  function scoreSteps(move) {
    const breakdown = move.scoreBreakdown || {};
    return [
      `Score: trees ${breakdown.trees || 0}, mountains ${breakdown.mountains || 0}, fields ${
        breakdown.fields || 0
      }, buildings ${breakdown.buildings || 0}, water ${breakdown.water || 0}, animals ${
        breakdown.animals || 0
      }, spirits ${breakdown.spirits || 0}`,
    ];
  }

  function labelColor(color) {
    return COLOR_LABELS[color] || "Unknown";
  }

  window.HarmoniesAdvisorClient = { createAdvisorClient };
})();
