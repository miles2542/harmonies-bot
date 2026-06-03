(function harmoniesContentScript() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const SCRIPT_ID = "harmonies-advisor-page-bridge";

  const overlay = window.HarmoniesAdvisorOverlay.createOverlay();
  const advisorClient = window.HarmoniesAdvisorClient.createAdvisorClient();

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

  function getCurrentPlayerId(gamedatas) {
    return String(gamedatas?.player_id || gamedatas?.current_player_id || "");
  }

  function isActiveParticipant(gamedatas) {
    const playerId = getCurrentPlayerId(gamedatas);
    const players = gamedatas?.players || {};
    return Boolean(playerId && players[playerId]);
  }

  async function handleState(payload) {
    const gamedatas = payload?.gamedatas;
    if (!isHarmoniesGame(gamedatas)) {
      overlay.setStatus("Waiting for Harmonies table");
      return;
    }
    if (!isActiveParticipant(gamedatas)) {
      overlay.setStatus("Participant not detected");
      return;
    }

    overlay.setStatus("Analyzing visible state");
    const response = await advisorClient.getRecommendation(gamedatas, (partialResponse) => {
      overlay.renderRecommendation(partialResponse);
    });
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
