(function harmoniesPageBridge() {
  const EVENT_TYPE = "HARMONIES_BGA_STATE";
  const INTERVAL_MS = 1500;

  function readState() {
    const gameui = window.gameui;
    if (!gameui || !gameui.gamedatas) {
      return null;
    }
    return {
      capturedAt: new Date().toISOString(),
      locationHref: window.location.href,
      currentPlayerId: readCurrentPlayerId(gameui),
      isSpectator: Boolean(gameui.isSpectator || gameui.is_spectator),
      gamedatas: gameui.gamedatas,
    };
  }

  function readCurrentPlayerId(gameui) {
    const candidates = [
      gameui.player_id,
      gameui.current_player_id,
      gameui.currentPlayerId,
      gameui.gamedatas?.player_id,
      gameui.gamedatas?.current_player_id,
    ];
    return String(candidates.find((value) => value !== undefined && value !== null) || "");
  }

  function publishState() {
    const payload = readState();
    if (!payload) {
      return;
    }
    window.postMessage({ type: EVENT_TYPE, payload }, window.location.origin);
  }

  publishState();
  window.setInterval(publishState, INTERVAL_MS);
})();
