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
      gamedatas: gameui.gamedatas,
    };
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
