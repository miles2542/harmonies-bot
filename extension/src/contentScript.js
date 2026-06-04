(function harmoniesContentScript() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const SCRIPT_ID = "harmonies-advisor-page-bridge";

  const overlay = window.HarmoniesAdvisorOverlay.createOverlay();
  const advisorClient = window.HarmoniesAdvisorClient.createAdvisorClient();
  let latestPayload = null;
  let isAnalyzing = false;
  let activeStateKey = "";
  overlay.onAnalyze(() => {
    handleAnalyzeClick().catch((error) => {
      isAnalyzing = false;
      overlay.setAnalyzeLabel("Retry");
      overlay.setStatus(`Advisor error: ${error.message}`);
    });
  });
  overlay.onStop(() => {
    advisorClient.stop();
    isAnalyzing = false;
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
    if (!isActiveParticipant(gamedatas, latestPayload)) {
      const detected = getCurrentPlayerId(gamedatas, latestPayload) || "none";
      overlay.setStatus(`Participant not detected; player ${detected}`);
      return;
    }
    if (!isAnalyzing && !activeStateKey) {
      overlay.setStatus("Ready to analyze current state");
    }
  }

  async function handleAnalyzeClick() {
    const payload = latestPayload;
    const gamedatas = payload?.gamedatas;
    if (!isHarmoniesGame(gamedatas)) {
      overlay.setStatus("Waiting for Harmonies table");
      return;
    }
    if (!isActiveParticipant(gamedatas, payload)) {
      overlay.setStatus("Participant not detected");
      return;
    }

    const playerId = getCurrentPlayerId(gamedatas, payload);
    if (!isOurActionPhase(gamedatas, playerId)) {
      overlay.setStatus("Waiting for your active action phase");
      return;
    }
    const stateKey = buildStateKey(gamedatas, playerId);
    if (!stateKey) {
      overlay.setStatus("Waiting for complete table state");
      return;
    }
    if (isAnalyzing) {
      return;
    }

    isAnalyzing = true;
    activeStateKey = stateKey;
    overlay.setAnalyzeLabel("Analyze");
    overlay.setStatus("Analyzing visible state");
    const centralTokenGroups = readDomCentralTokenGroups();
    try {
      const response = await advisorClient.getRecommendation(gamedatas, (partialResponse) => {
        if (activeStateKey === stateKey) {
          overlay.renderRecommendation(partialResponse);
        }
      }, playerId, { centralTokenGroups });
      if (activeStateKey === stateKey) {
        overlay.renderRecommendation(response);
      }
    } finally {
      if (activeStateKey === stateKey) {
        isAnalyzing = false;
        overlay.setAnalyzeLabel("Retry");
      }
    }
  }

  function isOurActionPhase(gamedatas, playerId) {
    return String(gamedatas?.gamestate?.active_player || "") === String(playerId);
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
      central: readDomCentralTokenGroups(),
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
      .map(([key, tokens]) => [key, compactCentralTokens(tokens)]);
  }

  function compactCentralTokens(tokens) {
    return (Array.isArray(tokens) ? tokens : [])
      .map((token) => colorByTypeArg(token?.type_arg))
      .filter(Boolean);
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

  function readDomCentralTokenGroups() {
    const groups = [];
    for (let groupId = 1; groupId <= 5; groupId += 1) {
      const hole = document.getElementById(`hole-${groupId}`);
      if (!hole) {
        continue;
      }
      const tokenNodes = [1, 2, 3]
        .map((tokenIndex) => document.getElementById(`hole-${groupId}-token-${tokenIndex}`))
        .filter(Boolean);
      const tokens = (tokenNodes.length ? tokenNodes : Array.from(hole.querySelectorAll(".hole-token, .colored-token")))
        .map(domTokenColor)
        .filter(Boolean);
      groups.push(tokens);
    }
    if (groups.length === 5 && groups.every((tokens) => tokens.length === 3)) {
      return groups;
    }
    return compactCentralGroups(latestPayload?.gamedatas?.tokensOnCentralBoard).map(
      ([, tokens]) => tokens,
    );
  }

  function domTokenColor(node) {
    const className = String(node.className || "");
    const match = /(?:^|\s)color-(\d)(?:\s|$)/.exec(className);
    return colorByTypeArg(match?.[1]);
  }

  function colorByTypeArg(typeArg) {
    return {
      1: "water",
      2: "mountain",
      3: "trunk",
      4: "foliage",
      5: "field",
      6: "building",
      7: "building",
    }[Number(typeArg)];
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
