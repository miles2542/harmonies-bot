// ==UserScript==
// @name         Harmonies BGA Snapshot Capture
// @namespace    harmonies-bga-advisor-local
// @version      0.2.0
// @description  Read-only Harmonies snapshot capture helper for scorer validation.
// @match        https://boardgamearena.com/*
// @match        https://*.boardgamearena.com/*
// @grant        GM_setClipboard
// @grant        unsafeWindow
// ==/UserScript==

(function harmoniesCaptureUserScript() {
  const ROOT_ID = "harmonies-capture-panel";
  const STORAGE_KEY = "harmonies-bga-capture-latest-v2";
  const DOM_KEY_RE = /(harm|board|cell|hex|token|cube|card|animal|player|score|result)/i;
  const MAX_DOM_NODES = 2500;

  function readPayload() {
    const gamedatas = readGamedatas();
    const stored = readStoredPayload();
    return {
      kind: "harmonies-bga-capture-v2",
      capturedAt: new Date().toISOString(),
      url: window.location.href,
      title: document.title,
      context: readContext(gamedatas),
      scoreHints: readScoreHints(gamedatas),
      visibleScoreText: readVisibleScoreText(),
      domSnapshot: readDomSnapshot(),
      storedLatest: stored,
      gamedatas,
    };
  }

  function rememberLatestGamedatas() {
    const gamedatas = readGamedatas();
    if (!gamedatas) {
      return;
    }
    const payload = {
      storedAt: new Date().toISOString(),
      url: window.location.href,
      title: document.title,
      context: readContext(gamedatas),
      gamedatas,
    };
    try {
      window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
    } catch (_error) {
      // Best-effort cache only. Capture should still work if storage quota blocks us.
    }
  }

  function readStoredPayload() {
    try {
      const raw = window.sessionStorage.getItem(STORAGE_KEY);
      return raw ? JSON.parse(raw) : null;
    } catch (_error) {
      return null;
    }
  }

  function readPageWindow() {
    if (typeof unsafeWindow !== "undefined" && unsafeWindow) {
      return unsafeWindow;
    }
    return window;
  }

  function readGamedatas() {
    return readPageWindow().gameui?.gamedatas || null;
  }

  function readContext(gamedatas) {
    return {
      boardSide: gamedatas?.boardSide || null,
      activePlayer: String(gamedatas?.gamestate?.active_player || ""),
      playerCount: Object.keys(gamedatas?.players || {}).length,
      remainingTokens: gamedatas?.remainingTokens ?? null,
      gameStateName: gamedatas?.gamestate?.name || null,
      gameStateDescription: gamedatas?.gamestate?.description || null,
    };
  }

  function readScoreHints(gamedatas) {
    const hints = new Map();
    Object.entries(gamedatas?.players || {}).forEach(([playerId, player]) => {
      addHint(hints, playerId, player.score, "gamedatas.players.score");
      addHint(hints, playerId, player.player_score, "gamedatas.players.player_score");
    });
    document.querySelectorAll("[id*='score'], [class*='score']").forEach((node) => {
      const total = parseScoreText(node.textContent || "");
      if (!Number.isFinite(total)) {
        return;
      }
      const playerId = playerIdFromNode(node);
      if (playerId) {
        addHint(hints, playerId, total, "dom");
      }
    });
    return Array.from(hints.values()).sort((left, right) =>
      left.playerId.localeCompare(right.playerId),
    );
  }

  function readVisibleScoreText() {
    const snippets = new Set();
    document
      .querySelectorAll("[id*='score'], [class*='score'], [id*='result'], [class*='result']")
      .forEach((node) => {
        const text = compactText(node.textContent || "");
        if (text.length >= 2 && text.length <= 240) {
          snippets.add(text);
        }
      });
    return Array.from(snippets).slice(0, 80);
  }

  function readDomSnapshot() {
    const nodes = [];
    const allNodes = Array.from(document.body?.querySelectorAll("*") || []);
    for (const node of allNodes) {
      if (nodes.length >= MAX_DOM_NODES) {
        break;
      }
      const id = node.id || "";
      const className = stringClassName(node.className);
      const dataset = readDataset(node);
      const combined = `${node.tagName} ${id} ${className} ${Object.keys(dataset).join(" ")}`;
      if (!DOM_KEY_RE.test(combined)) {
        continue;
      }
      nodes.push({
        tag: node.tagName.toLowerCase(),
        id,
        className,
        dataset,
        title: node.getAttribute("title") || "",
        ariaLabel: node.getAttribute("aria-label") || "",
        style: compactStyle(node.getAttribute("style") || ""),
        text: readableNodeText(node),
        rect: readRect(node),
      });
    }
    return {
      nodeCount: nodes.length,
      truncated: nodes.length >= MAX_DOM_NODES,
      nodes,
    };
  }

  function stringClassName(value) {
    if (typeof value === "string") {
      return value;
    }
    return value?.baseVal || "";
  }

  function readDataset(node) {
    return Object.fromEntries(
      Object.entries(node.dataset || {}).filter(([, value]) => String(value).length <= 120),
    );
  }

  function compactStyle(style) {
    return style
      .split(";")
      .map((part) => part.trim())
      .filter((part) => /(?:left|top|transform|background|width|height|display|visibility)/i.test(part))
      .join("; ")
      .slice(0, 500);
  }

  function readableNodeText(node) {
    const text = compactText(node.textContent || "");
    return text.length <= 180 ? text : "";
  }

  function readRect(node) {
    const rect = node.getBoundingClientRect();
    return {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };
  }

  function compactText(text) {
    return text.replace(/\s+/g, " ").trim();
  }

  function addHint(hints, playerId, rawTotal, source) {
    const total = Number(rawTotal);
    if (!playerId || !Number.isInteger(total)) {
      return;
    }
    hints.set(String(playerId), { playerId: String(playerId), total, source });
  }

  function parseScoreText(text) {
    const trimmed = text.trim();
    if (!/^-?\d+$/.test(trimmed)) {
      return Number.NaN;
    }
    return Number.parseInt(trimmed, 10);
  }

  function playerIdFromNode(node) {
    const raw = `${node.id || ""} ${node.className || ""}`;
    const match = /(?:player|score)[_-]?(\d{5,})/.exec(raw);
    return match?.[1] || "";
  }

  function installPanel() {
    if (document.getElementById(ROOT_ID)) {
      return;
    }
    const root = document.createElement("section");
    root.id = ROOT_ID;
    root.innerHTML = `
      <strong>Harmonies Capture</strong>
      <button type="button" data-action="copy">Copy</button>
      <button type="button" data-action="download">Download</button>
      <span data-role="status"></span>
    `;
    root.style.cssText = [
      "position:fixed",
      "right:12px",
      "bottom:12px",
      "z-index:99999",
      "display:flex",
      "gap:6px",
      "align-items:center",
      "padding:8px",
      "background:#111",
      "color:#fff",
      "font:12px sans-serif",
      "border:1px solid #555",
    ].join(";");
    document.documentElement.appendChild(root);
    root.querySelector("[data-action='copy']").addEventListener("click", () => copyPayload(root));
    root
      .querySelector("[data-action='download']")
      .addEventListener("click", () => downloadPayload(root));
  }

  function copyPayload(root) {
    const text = JSON.stringify(readPayload(), null, 2);
    if (typeof GM_setClipboard === "function") {
      GM_setClipboard(text);
      setStatus(root, "copied");
      return;
    }
    navigator.clipboard.writeText(text).then(() => setStatus(root, "copied"));
  }

  function downloadPayload(root) {
    const payload = readPayload();
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `harmonies-gamedatas-${Date.now()}.json`;
    link.click();
    URL.revokeObjectURL(link.href);
    setStatus(root, "downloaded");
  }

  function setStatus(root, message) {
    root.querySelector("[data-role='status']").textContent = message;
  }

  window.setInterval(() => {
    rememberLatestGamedatas();
    installPanel();
  }, 1000);
  rememberLatestGamedatas();
  installPanel();
})();
