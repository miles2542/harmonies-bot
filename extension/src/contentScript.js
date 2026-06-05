(function harmoniesContentScript() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const SCRIPT_ID = "harmonies-advisor-page-bridge";

  const overlay = window.HarmoniesAdvisorOverlay.createOverlay();
  const advisorClient = window.HarmoniesAdvisorClient.createAdvisorClient();
  let latestPayload = null;
  let isAnalyzing = false;
  let activeStateKey = "";
  let activeRunId = 0;
  overlay.onAnalyze(() => {
    handleAnalyzeClick().catch((error) => {
      activeRunId += 1;
      isAnalyzing = false;
      activeStateKey = "";
      overlay.setAnalyzeLabel("Retry");
      overlay.setStatus(`Advisor error: ${error.message}`);
    });
  });
  overlay.onStop(() => {
    activeRunId += 1;
    advisorClient.stop();
    isAnalyzing = false;
    activeStateKey = "";
    overlay.setAnalyzeLabel("Retry");
    overlay.setStatus("Search stopped");
  });

  function injectPageBridge() {
    if (document.getElementById(SCRIPT_ID)) {
      return;
    }
    const script = document.createElement("script");
    script.id = SCRIPT_ID;
    script.src = browser.runtime.getURL("src/pageBridge.js");
    script.onload = () => script.remove();
    (document.head || document.documentElement).appendChild(script);
  }

  function isHarmoniesGame(gamedatas) {
    const name = String(gamedatas?.game_name || gamedatas?.gamename || "").toLowerCase();
    return name.includes("harmonies") || Boolean(gamedatas?.tokensOnCentralBoard);
  }

  function getCurrentPlayerId(gamedatas, payload) {
    return String(
      payload?.currentPlayerId ||
        gamedatas?.player_id ||
        gamedatas?.current_player_id ||
        gamedatas?.gamestate?.active_player ||
        "",
    );
  }

  function isActiveParticipant(gamedatas, payload) {
    if (payload?.isSpectator) {
      return false;
    }
    const playerId = getCurrentPlayerId(gamedatas, payload);
    const players = gamedatas?.players || {};
    return Boolean(playerId && players[playerId]);
  }

  function handleState(payload) {
    latestPayload = payload || null;
    const gamedatas = latestPayload?.gamedatas;
    if (!isHarmoniesGame(gamedatas)) {
      overlay.setStatus("Waiting for Harmonies table");
      return;
    }
    if (!isAnalyzing && !activeStateKey) {
      const perspective = resolveAnalysisPerspective(gamedatas, latestPayload);
      overlay.setStatus(
        perspective.playerId
          ? `Ready to analyze ${perspective.label}`
          : perspective.reason
            ? perspective.reason
          : "Waiting for active player state",
      );
    }
  }

  async function handleAnalyzeClick() {
    const payload = latestPayload;
    const gamedatas = clonePlainObject(payload?.gamedatas);
    if (!isHarmoniesGame(gamedatas)) {
      overlay.setStatus("Waiting for Harmonies table");
      return;
    }
    const perspective = resolveAnalysisPerspective(gamedatas, payload);
    if (!perspective.playerId) {
      overlay.setStatus(perspective.reason || "Active player not detected");
      return;
    }
    const playerId = perspective.playerId;
    const visibleStateV1 = window.HarmoniesVisibleStateReader.readVisibleState(gamedatas, playerId);
    const stateKey = buildStateKey(gamedatas, playerId, visibleStateV1);
    if (!stateKey) {
      overlay.setStatus("Waiting for complete table state");
      return;
    }
    if (isAnalyzing) {
      return;
    }

    isAnalyzing = true;
    activeStateKey = stateKey;
    const runId = activeRunId + 1;
    activeRunId = runId;
    overlay.beginAnalysis();
    overlay.setAnalyzeLabel("Analyzing");
    overlay.setStatus("Analyzing visible state");
    try {
      const response = await advisorClient.getRecommendation(gamedatas, (partialResponse) => {
        if (activeRunId === runId && activeStateKey === stateKey) {
          overlay.renderRecommendationTier(partialResponse);
        }
      }, playerId, { visibleStateV1 });
      if (activeRunId === runId && activeStateKey === stateKey) {
        overlay.renderRecommendationTier(response);
      }
    } finally {
      if (activeRunId === runId && activeStateKey === stateKey) {
        isAnalyzing = false;
        activeStateKey = "";
        overlay.setAnalyzeLabel("Retry");
      }
    }
  }

  function resolveAnalysisPerspective(gamedatas, payload) {
    const players = gamedatas?.players || {};
    const participant = getCurrentPlayerId(gamedatas, payload);
    const activePlayer = String(gamedatas?.gamestate?.active_player || "");
    const activeName = playerLabel(gamedatas, activePlayer);

    if (!payload?.isSpectator && participant && players[participant]) {
      if (String(activePlayer) === String(participant)) {
        return {
          playerId: participant,
          label: playerLabel(gamedatas, participant),
          mode: "participant",
        };
      }
      return {
        playerId: "",
        label: "",
        mode: "participant",
        reason: activePlayer
          ? `Waiting for your turn; active: ${activeName}`
          : "Waiting for active player state",
      };
    }

    if (activePlayer && players[activePlayer]) {
      return {
        playerId: activePlayer,
        label: `${activeName} (spectator)`,
        mode: "spectator",
      };
    }

    const firstPlayer = Object.keys(players)[0] || "";
    return firstPlayer
      ? {
          playerId: firstPlayer,
          label: `${playerLabel(gamedatas, firstPlayer)} (fallback)`,
          mode: "fallback",
        }
      : { playerId: "", label: "", mode: "unknown", reason: "Waiting for active player state" };
  }

  function playerLabel(gamedatas, playerId) {
    const player = gamedatas?.players?.[playerId];
    const name = player?.name || player?.player_name;
    return name ? `${name} (${playerId})` : String(playerId || "unknown");
  }

  function buildStateKey(gamedatas, playerId, visibleStateV1) {
    const player = gamedatas?.players?.[playerId];
    if (!player) {
      return "";
    }
    return JSON.stringify({
      playerId,
      activePlayer: gamedatas?.gamestate?.active_player || "",
      stateName: gamedatas?.gamestate?.name || "",
      remainingTokens: gamedatas?.remainingTokens ?? null,
      visibleStateV1,
      river: compactCards(gamedatas?.river),
      spirits: compactCards(gamedatas?.spiritsCards),
      boardCards: compactCards(player.boardAnimalCards),
      doneCards: compactCards(player.doneAnimalCards),
      animalCubesOnBoard: player.animalCubesOnBoard || {},
      cubesOnAnimalCards: gamedatas?.cubesOnAnimalCards || {},
      emptyHexes: player.emptyHexes ?? null,
    });
  }

  function compactCards(cards) {
    return (Array.isArray(cards) ? cards : []).map((card) => [
      card?.id ?? null,
      card?.type_arg ?? null,
      card?.location ?? null,
      card?.location_arg ?? null,
      card?.isSpirit ?? null,
    ]);
  }

  function clonePlainObject(value) {
    if (!value) {
      return value;
    }
    try {
      return structuredClone(value);
    } catch (_error) {
      return JSON.parse(JSON.stringify(value));
    }
  }

  window.addEventListener("message", (event) => {
    if (event.source !== window || event.data?.type !== EVENT_TYPE) {
      return;
    }
    try {
      handleState(event.data.payload);
    } catch (error) {
      overlay.setStatus(`Advisor error: ${error.message}`);
    }
  });

  injectPageBridge();
})();
