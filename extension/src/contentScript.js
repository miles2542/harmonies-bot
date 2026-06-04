(function harmoniesContentScript() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const SCRIPT_ID = "harmonies-advisor-page-bridge";

  const overlay = window.HarmoniesAdvisorOverlay.createOverlay();
  const advisorClient = window.HarmoniesAdvisorClient.createAdvisorClient();
  let isAnalyzing = false;
  let activeStateKey = "";
  let completedStateKey = "";
  let stoppedStateKey = "";
  overlay.onStop(() => {
    stoppedStateKey = activeStateKey;
    advisorClient.stop();
    overlay.setStatus(isAnalyzing ? "Stopping search" : "Search stopped; waiting for state change");
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

  async function handleState(payload) {
    const gamedatas = payload?.gamedatas;
    if (!isHarmoniesGame(gamedatas)) {
      overlay.setStatus("Waiting for Harmonies table");
      return;
    }
    if (!isActiveParticipant(gamedatas, payload)) {
      const detected = getCurrentPlayerId(gamedatas, payload) || "none";
      overlay.setStatus(`Participant not detected; player ${detected}`);
      return;
    }

    const playerId = getCurrentPlayerId(gamedatas, payload);
    const stateKey = buildStateKey(gamedatas, playerId);
    if (!stateKey) {
      overlay.setStatus("Waiting for complete table state");
      return;
    }
    if (isAnalyzing || stateKey === completedStateKey) {
      return;
    }
    if (stateKey === stoppedStateKey) {
      overlay.setStatus("Search stopped; waiting for state change");
      return;
    }

    isAnalyzing = true;
    activeStateKey = stateKey;
    overlay.setStatus("Analyzing visible state");
    try {
      const response = await advisorClient.getRecommendation(gamedatas, (partialResponse) => {
        if (activeStateKey === stateKey) {
          overlay.renderRecommendation(partialResponse);
        }
      }, playerId);
      if (activeStateKey === stateKey) {
        overlay.renderRecommendation(response);
        completedStateKey = stateKey;
      }
    } finally {
      if (activeStateKey === stateKey) {
        isAnalyzing = false;
      }
    }
  }

  function buildStateKey(gamedatas, playerId) {
    const player = gamedatas?.players?.[playerId];
    if (!player) {
      return "";
    }
    return JSON.stringify({
      playerId,
      activePlayer: gamedatas?.gamestate?.active_player || "",
      stateName: gamedatas?.gamestate?.name || "",
      remainingTokens: gamedatas?.remainingTokens ?? null,
      central: compactCentralGroups(gamedatas?.tokensOnCentralBoard),
      river: compactCards(gamedatas?.river),
      spirits: compactCards(gamedatas?.spiritsCards),
      boardCards: compactCards(player.boardAnimalCards),
      doneCards: compactCards(player.doneAnimalCards),
      tokensOnBoard: player.tokensOnBoard || {},
      animalCubesOnBoard: player.animalCubesOnBoard || {},
      cubesOnAnimalCards: gamedatas?.cubesOnAnimalCards || {},
      emptyHexes: player.emptyHexes ?? null,
    });
  }

  function compactCentralGroups(groups) {
    return Object.entries(groups || {})
      .sort(([left], [right]) => Number.parseInt(left, 10) - Number.parseInt(right, 10))
      .map(([key, tokens]) => [key, compactTokens(tokens)]);
  }

  function compactTokens(tokens) {
    return (Array.isArray(tokens) ? tokens : []).map((token) => [
      token?.id ?? null,
      token?.type_arg ?? null,
      token?.location ?? null,
      token?.location_arg ?? null,
    ]);
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

  window.addEventListener("message", (event) => {
    if (event.source !== window || event.data?.type !== EVENT_TYPE) {
      return;
    }
    handleState(event.data.payload).catch((error) => {
      overlay.setStatus(`Advisor error: ${error.message}`);
    });
  });

  injectPageBridge();
})();
