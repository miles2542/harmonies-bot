(function harmoniesAdvisorClientModule() {
  const SERVICE_URL = "http://127.0.0.1:17848/advise";
  const SERVICE_WS_URL = "ws://127.0.0.1:17848/ws";
  const SERVICE_TIMEOUT_MS = 115000;
  const COLOR_LABELS = {
    water: "Water",
    mountain: "Mountain",
    trunk: "Trunk",
    foliage: "Foliage",
    field: "Field",
    building: "Building",
  };
  let activeSocket = null;

  function createAdvisorClient() {
    return {
      async getRecommendation(gamedatas, onUpdate, perspectivePlayerId, options = {}) {
        const snapshot = window.HarmoniesBgaNormalizer.normalizeGamedatas(
          gamedatas,
          perspectivePlayerId,
          {
            visibleStateV1: options.visibleStateV1,
            centralTokenGroups: options.centralTokenGroups,
            boardCellsByPlayerId: options.boardCellsByPlayerId,
          },
        );
        try {
          const response = await requestNativeAdvisor(snapshot, (partialResponse) => {
            if (onUpdate) {
              onUpdate(adaptAdvisorResponse(partialResponse, snapshot, false));
            }
          });
          return adaptAdvisorResponse(response, snapshot, true);
        } catch (error) {
          const mock = window.HarmoniesMockAdvisor.recommend(snapshot);
          mock.status = `Mock advisor active; native service unavailable: ${error.message}`;
          mock.isFinal = true;
          return mock;
        }
      },
      stop() {
        if (activeSocket?.readyState === WebSocket.OPEN) {
          activeSocket.send(JSON.stringify({ command: "stop" }));
        }
      },
    };
  }

  async function requestNativeAdvisor(snapshot, onUpdate) {
    const request = buildAdvisorRequest(snapshot);
    try {
      return await requestNativeAdvisorWs(request, onUpdate);
    } catch (_error) {
      return requestNativeAdvisorHttp(request);
    }
  }

  function buildAdvisorRequest(snapshot) {
    return {
      snapshot,
      timeBudgetMs: 106000,
      maxResults: 3,
      seed: optionsSeed(snapshot),
      runtimeMode: "native-service",
    };
  }

  function optionsSeed(snapshot) {
    const input = JSON.stringify({
      perspectivePlayerId: snapshot.perspectivePlayerId,
      activePlayerId: snapshot.activePlayerId,
      centralTokenGroups: snapshot.centralTokenGroups,
      riverCards: snapshot.riverCards,
      players: snapshot.players.map((player) => ({
        playerId: player.playerId,
        cells: player.cells,
        activeCards: player.activeCards,
        spiritCardChoices: player.spiritCardChoices,
        completedCards: player.completedCards,
      })),
    });
    let hash = 2166136261;
    for (let index = 0; index < input.length; index += 1) {
      hash ^= input.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return hash >>> 0;
  }

  async function requestNativeAdvisorHttp(request) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), SERVICE_TIMEOUT_MS);
    try {
      const response = await fetch(SERVICE_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
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

  function requestNativeAdvisorWs(request, onUpdate) {
    return new Promise((resolve, reject) => {
      const socket = new WebSocket(SERVICE_WS_URL);
      activeSocket = socket;
      let settled = false;
      const timeout = setTimeout(() => {
        socket.close();
        settled = true;
        reject(new Error("WebSocket timeout"));
      }, SERVICE_TIMEOUT_MS);
      socket.addEventListener("open", () => socket.send(JSON.stringify(request)));
      socket.addEventListener("message", (event) => {
        const message = JSON.parse(event.data);
        if (message.event !== "advisorResponse" || !message.response) {
          return;
        }
        if (message.final) {
          clearTimeout(timeout);
          settled = true;
          socket.close();
          activeSocket = null;
          resolve(message.response);
        } else if (onUpdate) {
          onUpdate(message.response);
        }
      });
      socket.addEventListener("error", () => {
        clearTimeout(timeout);
        settled = true;
        activeSocket = null;
        reject(new Error("WebSocket unavailable"));
      });
      socket.addEventListener("close", () => {
        clearTimeout(timeout);
        if (activeSocket === socket) {
          activeSocket = null;
        }
        if (!settled) {
          settled = true;
          reject(new Error("WebSocket closed"));
        }
      });
    });
  }

  function adaptAdvisorResponse(response, snapshot, isFinal = false) {
    const best = response.bestMoves?.[0] || null;
    const progress = response.progress || {};
    return {
      status: statusText(response, progress),
      elapsedMs: response.elapsedMs || 0,
      progress,
      isFinal,
      bestMove: best
        ? {
            playerId: snapshot?.perspectivePlayerId || "",
            centralGroupId: String(best.centralGroupIndex + 1),
            title: `Take group ${best.centralGroupIndex + 1}; utility ${best.utilityEstimate ?? best.scoreEstimate}; future ${best.scoreEstimate}`,
            utilityEstimate: best.utilityEstimate ?? best.scoreEstimate ?? 0,
            futureEstimate: best.scoreEstimate ?? 0,
            denialEstimate: best.opponentDenialEstimate ?? 0,
            actions: best.orderedActions || [],
            steps: actionSteps(best).concat(scoreSteps(best)),
          }
        : null,
      alternatives: (response.bestMoves || []).slice(1).map((move) => ({
        centralGroupId: String(move.centralGroupIndex + 1),
        label: `Group ${move.centralGroupIndex + 1}: utility ${move.utilityEstimate ?? move.scoreEstimate}`,
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
        const groupIndex = action.groupIndex ?? action.group_index;
        return `${prefix} Take group ${groupIndex + 1}: ${action.tokens.map(labelColor).join(", ")}`;
      }
      if (action.kind === "placeToken") {
        return `${prefix} Place ${labelColor(action.token)} at (${action.col}, ${action.row})`;
      }
      if (action.kind === "draftCard") {
        return `${prefix} Draft type ${cardTypeArg(action)} card (instance ${cardId(action)})`;
      }
      if (action.kind === "chooseSpirit") {
        return `${prefix} Choose Spirit type ${cardTypeArg(action)} card (instance ${cardId(action)})`;
      }
      if (action.kind === "settleCard") {
        return `${prefix} Settle type ${cardTypeArg(action)} card cube at (${action.col}, ${action.row})`;
      }
      return `${prefix} ${action.kind}`;
    });
  }

  function scoreSteps(move) {
    const breakdown = move.scoreBreakdown || {};
    const immediateTotal =
      (breakdown.trees || 0) +
      (breakdown.mountains || 0) +
      (breakdown.fields || 0) +
      (breakdown.buildings || 0) +
      (breakdown.water || 0) +
      (breakdown.animals || 0) +
      (breakdown.spirits || 0);
    return [
      `Search estimate: future self ${move.scoreEstimate || 0} VP, denial ${move.opponentDenialEstimate || 0}, utility ${
        move.utilityEstimate || move.scoreEstimate || 0
      }`,
      `Immediate score if followed now: ${immediateTotal} VP; trees ${breakdown.trees || 0}, mountains ${breakdown.mountains || 0}, fields ${
        breakdown.fields || 0
      }, buildings ${breakdown.buildings || 0}, water ${breakdown.water || 0}, animals ${
        breakdown.animals || 0
      }, spirits ${breakdown.spirits || 0}`,
    ];
  }

  function labelColor(color) {
    return COLOR_LABELS[color] || "Unknown";
  }

  function cardId(action) {
    return action.cardId ?? action.card_id;
  }

  function cardTypeArg(action) {
    return action.typeArg ?? action.type_arg;
  }

  window.HarmoniesAdvisorClient = { createAdvisorClient };
})();
