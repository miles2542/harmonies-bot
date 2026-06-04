(function harmoniesContentScript() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const SCRIPT_ID = "harmonies-advisor-page-bridge";

  const overlay = window.HarmoniesAdvisorOverlay.createOverlay();
  const advisorClient = window.HarmoniesAdvisorClient.createAdvisorClient();
  overlay.onStop(() => {
    advisorClient.stop();
    overlay.setStatus("Stopping search");
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

    overlay.setStatus("Analyzing visible state");
    const playerId = getCurrentPlayerId(gamedatas, payload);
    const response = await advisorClient.getRecommendation(gamedatas, (partialResponse) => {
      overlay.renderRecommendation(partialResponse);
    }, playerId);
    overlay.renderRecommendation(response);
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
