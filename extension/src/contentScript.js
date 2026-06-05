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
    const centralTokenGroups = readDomCentralTokenGroups();
    const stateKey = buildStateKey(gamedatas, playerId, centralTokenGroups);
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
      }, playerId, { centralTokenGroups });
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

  function buildStateKey(gamedatas, playerId, centralTokenGroups = readDomCentralTokenGroups()) {
    const player = gamedatas?.players?.[playerId];
    if (!player) {
      return "";
    }
    return JSON.stringify({
      playerId,
      activePlayer: gamedatas?.gamestate?.active_player || "",
      stateName: gamedatas?.gamestate?.name || "",
      remainingTokens: gamedatas?.remainingTokens ?? null,
      central: centralTokenGroups,
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
      const tokenNodes = centralTokenNodes(hole, groupId);
      const tokens = tokenNodes
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

  function centralTokenNodes(hole, groupId) {
    const orderedIds = [1, 2, 3]
      .map((tokenIndex) => document.getElementById(`hole-${groupId}-token-${tokenIndex}`))
      .filter((node) => node && hole.contains(node) && isVisibleElement(node));
    const candidates = orderedIds.length
      ? orderedIds
      : Array.from(
          hole.querySelectorAll(
            ".hole-token, .colored-token, [class*='color-'], [id^='hole-'][id*='-token-']",
          ),
        ).filter(isVisibleElement);
    const unique = [];
    const seen = new Set();
    candidates.forEach((node) => {
      if (!seen.has(node)) {
        seen.add(node);
        unique.push(node);
      }
    });
    unique.sort((left, right) => tokenNodeSortKey(left) - tokenNodeSortKey(right));
    return unique.filter((node) => domTokenColor(node)).slice(0, 3);
  }

  function tokenNodeSortKey(node) {
    const match = /-token-(\d+)/.exec(String(node.id || ""));
    return match ? Number.parseInt(match[1], 10) : 99;
  }

  function isVisibleElement(node) {
    const style = window.getComputedStyle(node);
    const rect = node.getBoundingClientRect();
    return (
      style.display !== "none" &&
      style.visibility !== "hidden" &&
      Number(style.opacity || 1) > 0.01 &&
      rect.width > 2 &&
      rect.height > 2
    );
  }

  function domTokenColor(node) {
    const className = [node, ...node.querySelectorAll("*")]
      .map((item) => String(item.className || ""))
      .join(" ");
    const match = /(?:^|\s)color-(\d)(?:\s|$)/.exec(className);
    return colorByTypeArg(match?.[1]);
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
