// ==UserScript==
// @name         Harmonies BGA Snapshot Capture
// @namespace    harmonies-bga-advisor-local
// @version      0.3.4
// @description  Read-only Harmonies snapshot capture helper for scorer validation.
// @match        https://boardgamearena.com/*
// @match        https://*.boardgamearena.com/*
// @grant        GM_setClipboard
// @grant        GM.setClipboard
// @grant        unsafeWindow
// ==/UserScript==

(function harmoniesCaptureUserScript() {
  const SCRIPT_VERSION = "0.3.4";
  const ROOT_ID = "harmonies-capture-panel";
  const STORAGE_KEY = "harmonies-bga-capture-latest-v2";
  const DOM_KEY_RE = /(harm|board|cell|hex|token|cube|card|animal|player|score|result)/i;
  const MAX_DOM_NODES = 2500;
  const CELL_RE = /^cell_(.+)_(-?\d+)_(-?\d+)$/;
  const CARD_RE = /^card_(\d+)$/;
  const PLAYER_TABLE_RE = /^player-table-(\d+)$/;
  const COLOR_BY_CLASS = {
    1: "water",
    2: "mountain",
    3: "trunk",
    4: "foliage",
    5: "field",
    6: "building",
    7: "building",
  };
  const CUBE_COUNT_BY_CARD_TYPE_ARG = {
    1: 3, 2: 3, 3: 4, 4: 3, 5: 5, 6: 4, 7: 3, 8: 3, 9: 3, 10: 3, 11: 3,
    12: 2, 13: 2, 14: 2, 15: 3, 16: 3, 17: 3, 18: 4, 19: 3, 20: 3, 21: 3,
    22: 4, 23: 3, 24: 2, 25: 2, 26: 4, 27: 2, 28: 2, 29: 3, 30: 2, 31: 5,
    32: 2, 33: 1, 34: 1, 35: 1, 36: 1, 37: 1, 38: 1, 39: 1, 40: 1, 41: 1, 42: 1,
  };

  function readPayload() {
    const gamedatas = readGamedatas();
    const stored = readStoredPayload();
    const domSnapshot = readDomSnapshot();
    return {
      kind: "harmonies-bga-capture-v2",
      scriptVersion: SCRIPT_VERSION,
      capturedAt: new Date().toISOString(),
      url: window.location.href,
      title: document.title,
      context: readContext(gamedatas),
      scoreHints: readScoreHints(gamedatas),
      visibleScoreText: readVisibleScoreText(),
      visibleStateV1: readVisibleStateV1(gamedatas, domSnapshot),
      domSnapshot,
      storedLatest: stored,
      gamedatas,
    };
  }

  function readVisibleStateV1(gamedatas, domSnapshot) {
    const notes = [];
    if (domSnapshot.truncated) notes.push("domSnapshot truncated before all matching nodes captured");
    const tables = playerTables(notes);
    const cardPointCounts = cardPointCountsByInstanceId(gamedatas);
    const cards = visibleCards(notes, cardPointCounts);
    const centralTokenGroups = centralGroups(notes);
    const players = tables.map((table, index) =>
      visiblePlayer(table, index, gamedatas, cards, notes),
    );
    const firstTableY = Math.min(...tables.map((table) => table.rect.y).filter(Number.isFinite));
    const riverCards = cards
      .filter((card) => card.source === "river" || isRiverCard(card, firstTableY))
      .map((card) => ownedCard(card, "river", null, null));
    const spiritChoicesByPlayerId = {};
    for (const player of players) spiritChoicesByPlayerId[player.playerId] = player.spiritChoices;
    for (const player of players) delete player.spiritChoices;
    return {
      schemaVersion: 1,
      capturedAt: new Date().toISOString(),
      activePlayerId: String(gamedatas?.gamestate?.active_player || ""),
      currentPlayerId: currentPlayerId(gamedatas),
      players,
      centralTokenGroups,
      riverCards: sortCards(uniqueCards(riverCards, notes)),
      spiritChoicesByPlayerId,
      reliability: {
        domCards: cards.length > 0,
        domBoards: tables.length > 0 && players.every((player) => player.cells.length > 0),
        domCentral: centralTokenGroups.length === 5,
        notes,
      },
    };
  }

  function playerTables(notes) {
    const tables = Array.from(document.querySelectorAll("[id^='player-table-']"))
      .map((node) => ({ node, rect: readRect(node), playerId: PLAYER_TABLE_RE.exec(node.id)?.[1] || "" }))
      .filter((table) => table.playerId && table.rect.width > 0 && table.rect.height > 0)
      .sort((left, right) => left.rect.y - right.rect.y || left.rect.x - right.rect.x);
    if (!tables.length) notes.push("no #player-table-<id> containers found");
    return tables;
  }

  function visiblePlayer(table, index, gamedatas, allCards, notes) {
    const playerId = table.playerId;
    const hand = document.getElementById(`hand-${playerId}`);
    const done = document.getElementById(`done-${playerId}`);
    if (!hand) notes.push(`missing #hand-${playerId}`);
    if (!done) notes.push(`missing #done-${playerId}`);
    const handRect = hand ? readRect(hand) : emptyRect();
    const doneRect = done ? readRect(done) : emptyRect();
    const activeCards = sortCards(
      uniqueCards(allCards
        .filter((card) => contains(handRect, card.rect))
        .map((card) => ownedCard(card, "hand", playerId, slotId(card.node, hand))), notes),
    );
    const completedCards = sortCards(
      uniqueCards(allCards
        .filter((card) => nonzero(card.rect) && contains(doneRect, card.rect))
        .map((card) => ownedCard(card, "done", playerId, slotId(card.node, done))), notes),
    ).map((card) => ({ ...card, remainingCubes: 0 }));
    const assigned = new Set([...activeCards, ...completedCards].map((card) => card.cardInstanceId));
    const spiritChoices = sortCards(
      uniqueCards(allCards
        .filter((card) => !assigned.has(card.cardInstanceId) && contains(table.rect, card.rect))
        .filter((card) => card.isSpirit)
        .map((card) => ownedCard(card, "spiritChoice", playerId, slotId(card.node, table.node))), notes),
    );
    if (activeCards.length > 4) notes.push(`hand-${playerId} has ${activeCards.length} cards`);
    return {
      playerId,
      name: playerName(gamedatas, playerId),
      order: playerOrder(gamedatas, playerId, index),
      cells: cellsForPlayer(playerId),
      activeCards,
      completedCards,
      spiritChoices,
    };
  }

  function visibleCards(notes, cardPointCounts) {
    const seen = new Map();
    return Array.from(document.querySelectorAll("[id^='card_'][data-card-id][data-card-type-arg]"))
      .filter((node) => !isCaptureUi(node) && nonzero(readRect(node)) && CARD_RE.test(node.id))
      .map((node) => {
        const idFromNode = Number.parseInt(CARD_RE.exec(node.id)[1], 10);
        const cardInstanceId = numberOr(node.dataset.cardId, idFromNode);
        const typeArg = numberOr(node.dataset.cardTypeArg, 0);
        if (seen.has(cardInstanceId)) notes.push(`duplicated card id ${cardInstanceId}`);
        seen.set(cardInstanceId, true);
        // cardId is BGA per-game instance id here. typeArg is persistent catalog id.
        return {
          node,
          cardInstanceId,
          cardId: cardInstanceId,
          typeArg,
          isSpirit: isSpiritCard(node, typeArg),
          remainingCubes: remainingCubes(cardInstanceId, typeArg, cardPointCounts),
          source: null,
          ownerPlayerId: null,
          slotId: null,
          rect: readRect(node),
        };
      })
      .filter((card) => card.cardInstanceId > 0 && card.typeArg > 0);
  }

  function cellsForPlayer(playerId) {
    const tokenNodes = visibleTokenNodes();
    const cubeNodes = visibleCubeNodes();
    return Array.from(document.querySelectorAll("[id^='cell_']"))
      .map((node) => ({ node, match: CELL_RE.exec(node.id), rect: readRect(node) }))
      .filter((cell) => cell.match?.[1] === playerId && nonzero(cell.rect))
      .sort((left, right) => Number(left.match[3]) - Number(right.match[3]) || Number(left.match[2]) - Number(right.match[2]))
      .map((cell) => ({
        coord: { col: Number(cell.match[2]), row: Number(cell.match[3]) },
        stack: { tokens: tokensInCell(cell.rect, tokenNodes) },
        lockedByCube: cubeNodes.some((cube) => centerInside(cell.rect, readRect(cube))),
      }));
  }

  function centralGroups(notes) {
    const groups = [];
    for (let hole = 1; hole <= 5; hole += 1) {
      const node = document.getElementById(`hole-${hole}`);
      if (!node) notes.push(`missing #hole-${hole}`);
      const tokenNodes = node
        ? Array.from(node.querySelectorAll(`[id^='hole-${hole}-token-'], .colored-token, [class*='color-']`))
        : [];
      groups.push(
        tokenNodes
          .filter((token) => nonzero(readRect(token)))
          .sort((left, right) => readRect(left).y - readRect(right).y || readRect(left).x - readRect(right).x)
          .map((token) => parseColor(stringClassName(token.className)))
          .filter(Boolean),
      );
    }
    return groups;
  }

  function visibleTokenNodes() {
    return Array.from(document.querySelectorAll(".colored-token, [class*='color-']"))
      .filter((node) => !isCaptureUi(node) && parseColor(stringClassName(node.className)) && nonzero(readRect(node)));
  }

  function visibleCubeNodes() {
    return Array.from(document.querySelectorAll(".animal-cube, [class*='animal-cube']"))
      .filter((node) => !isCaptureUi(node) && nonzero(readRect(node)));
  }

  function tokensInCell(cellRect, tokenNodes) {
    return tokenNodes
      .filter((token) => centerInside(cellRect, readRect(token)))
      .map((token) => ({
        level: parseLevel(stringClassName(token.className)),
        color: parseColor(stringClassName(token.className)),
      }))
      .filter((token) => token.color)
      .sort((left, right) => left.level - right.level)
      .map((token) => token.color);
  }

  function remainingCubes(cardInstanceId, typeArg, cardPointCounts) {
    const visibleCount = Array.from(document.querySelectorAll(`[id^='card_${cardInstanceId}-score-']`)).filter((node) => {
      const className = stringClassName(node.className);
      return className.includes("points-location") && className.includes("animal-cube") && nonzero(readRect(node));
    }).length;
    return visibleCount || cardPointCounts.get(cardInstanceId) || CUBE_COUNT_BY_CARD_TYPE_ARG[typeArg] || 0;
  }

  function cardPointCountsByInstanceId(gamedatas) {
    const counts = new Map();
    const add = (card) => {
      const cardId = Number(card?.id);
      const points = Array.isArray(card?.pointLocations) ? card.pointLocations.length : 0;
      if (Number.isFinite(cardId) && points > 0) counts.set(cardId, points);
    };
    Object.values(gamedatas?.players || {}).forEach((player) => {
      (Array.isArray(player?.boardAnimalCards) ? player.boardAnimalCards : []).forEach(add);
      (Array.isArray(player?.doneAnimalCards) ? player.doneAnimalCards : []).forEach(add);
    });
    (Array.isArray(gamedatas?.river) ? gamedatas.river : []).forEach(add);
    (Array.isArray(gamedatas?.spiritsCards) ? gamedatas.spiritsCards : []).forEach(add);
    return counts;
  }

  function ownedCard(card, source, ownerPlayerId, slotIdValue) {
    const { node: _node, ...plain } = card;
    return { ...plain, source, ownerPlayerId, slotId: slotIdValue };
  }

  function isRiverCard(card, firstTableY) {
    if (!Number.isFinite(firstTableY)) return false;
    return card.rect.y < firstTableY;
  }

  function isSpiritCard(node, typeArg) {
    const raw = `${node.dataset.isSpirit || ""} ${node.className || ""}`.toLowerCase();
    return raw.includes("spirit") || typeArg >= 33;
  }

  function uniqueCards(cards, notes) {
    const byId = new Map();
    for (const card of cards) {
      if (byId.has(card.cardInstanceId)) notes.push(`duplicated visible card assignment ${card.cardInstanceId}`);
      byId.set(card.cardInstanceId, card);
    }
    return Array.from(byId.values());
  }

  function sortCards(cards) {
    return cards.sort((left, right) => left.rect.y - right.rect.y || left.rect.x - right.rect.x || left.cardInstanceId - right.cardInstanceId);
  }

  function slotId(cardNode, container) {
    const parent = cardNode.parentElement?.closest("[id]");
    return parent && container.contains(parent) ? parent.id : container.id || null;
  }

  function playerName(gamedatas, playerId) {
    const player = gamedatas?.players?.[playerId];
    return player?.name || player?.player_name || null;
  }

  function playerOrder(gamedatas, playerId, fallback) {
    const raw = gamedatas?.players?.[playerId]?.playerNo ?? gamedatas?.players?.[playerId]?.player_no;
    const value = Number(raw);
    return Number.isFinite(value) ? value : fallback;
  }

  function currentPlayerId(gamedatas) {
    const raw = readPageWindow().gameui?.player_id ?? gamedatas?.current_player_id ?? gamedatas?.player_id ?? null;
    return raw === null || raw === undefined || raw === "" ? null : String(raw);
  }

  function parseColor(className) {
    const match = /\bcolor-(\d+)\b/.exec(className);
    return match ? COLOR_BY_CLASS[match[1]] || null : null;
  }

  function parseLevel(className) {
    const matches = Array.from(className.matchAll(/\blevel-(\d+)\b/g));
    return matches.length ? Number(matches.at(-1)[1]) : 1;
  }

  function contains(outer, inner) {
    return nonzero(outer) && nonzero(inner) && inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.width <= outer.x + outer.width && inner.y + inner.height <= outer.y + outer.height;
  }

  function centerInside(outer, inner) {
    if (!nonzero(outer) || !nonzero(inner)) return false;
    const x = inner.x + inner.width / 2;
    const y = inner.y + inner.height / 2;
    return outer.x <= x && x <= outer.x + outer.width && outer.y <= y && y <= outer.y + outer.height;
  }

  function nonzero(rect) {
    return rect.width > 0 && rect.height > 0;
  }

  function emptyRect() {
    return { x: 0, y: 0, width: 0, height: 0 };
  }

  function numberOr(value, fallback) {
    const parsed = Number.parseInt(String(value), 10);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function isCaptureUi(node) {
    return Boolean(node.closest(`#${ROOT_ID}, #harmonies-advisor-root, #harmonies-advisor-visual-layer`));
  }

  function rememberLatestGamedatas() {
    const gamedatas = readGamedatas();
    if (!gamedatas) return;
    const payload = { storedAt: new Date().toISOString(), url: window.location.href, title: document.title, context: readContext(gamedatas), gamedatas };
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
    return typeof unsafeWindow !== "undefined" && unsafeWindow ? unsafeWindow : window;
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
      const playerId = Number.isFinite(total) ? playerIdFromNode(node) : "";
      if (playerId) addHint(hints, playerId, total, "dom");
    });
    return Array.from(hints.values()).sort((left, right) => left.playerId.localeCompare(right.playerId));
  }

  function readVisibleScoreText() {
    const snippets = new Set();
    document.querySelectorAll("[id*='score'], [class*='score'], [id*='result'], [class*='result']").forEach((node) => {
      const text = compactText(node.textContent || "");
      if (text.length >= 2 && text.length <= 240) snippets.add(text);
    });
    return Array.from(snippets).slice(0, 80);
  }

  function readDomSnapshot() {
    const nodes = [];
    for (const node of Array.from(document.body?.querySelectorAll("*") || [])) {
      if (nodes.length >= MAX_DOM_NODES) break;
      const id = node.id || "";
      const className = stringClassName(node.className);
      const dataset = readDataset(node);
      const combined = `${node.tagName} ${id} ${className} ${Object.keys(dataset).join(" ")}`;
      if (!DOM_KEY_RE.test(combined)) continue;
      nodes.push({ tag: node.tagName.toLowerCase(), id, className, dataset, title: node.getAttribute("title") || "", ariaLabel: node.getAttribute("aria-label") || "", style: compactStyle(node.getAttribute("style") || ""), text: readableNodeText(node), rect: readRect(node) });
    }
    return { nodeCount: nodes.length, truncated: nodes.length >= MAX_DOM_NODES, nodes };
  }

  function stringClassName(value) {
    return typeof value === "string" ? value : value?.baseVal || "";
  }

  function readDataset(node) {
    return Object.fromEntries(Object.entries(node.dataset || {}).filter(([, value]) => String(value).length <= 120));
  }

  function compactStyle(style) {
    return style.split(";").map((part) => part.trim()).filter((part) => /(?:left|top|transform|background|width|height|display|visibility)/i.test(part)).join("; ").slice(0, 500);
  }

  function readableNodeText(node) {
    const text = compactText(node.textContent || "");
    return text.length <= 180 ? text : "";
  }

  function readRect(node) {
    const rect = node.getBoundingClientRect();
    return { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) };
  }

  function compactText(text) {
    return text.replace(/\s+/g, " ").trim();
  }

  function addHint(hints, playerId, rawTotal, source) {
    const total = Number(rawTotal);
    if (playerId && Number.isInteger(total)) hints.set(String(playerId), { playerId: String(playerId), total, source });
  }

  function parseScoreText(text) {
    return /^-?\d+$/.test(text.trim()) ? Number.parseInt(text.trim(), 10) : Number.NaN;
  }

  function playerIdFromNode(node) {
    const match = /(?:player|score)[_-]?(\d{5,})/.exec(`${node.id || ""} ${node.className || ""}`);
    return match?.[1] || "";
  }

  function installPanel() {
    const existing = document.getElementById(ROOT_ID);
    if (existing?.dataset.version === SCRIPT_VERSION) return;
    if (existing) existing.remove();
    const root = document.createElement("section");
    root.id = ROOT_ID;
    root.dataset.version = SCRIPT_VERSION;
    root.innerHTML = `<strong>Harmonies Capture</strong><button type="button" data-action="copy">Copy</button><button type="button" data-action="download">Download</button><a data-role="download-link" style="display:none;color:#fff;text-decoration:underline" download>Save</a><span data-role="status">v${SCRIPT_VERSION}</span>`;
    root.style.cssText = ["position:fixed", "right:12px", "bottom:12px", "z-index:99999", "display:flex", "gap:6px", "align-items:center", "padding:8px", "background:#111", "color:#fff", "font:12px sans-serif", "border:1px solid #555"].join(";");
    document.documentElement.appendChild(root);
    root.querySelector("[data-action='copy']").addEventListener("click", () => copyPayload(root));
    root.querySelector("[data-action='download']").addEventListener("click", () => downloadPayload(root));
  }

  function copyPayload(root) {
    runCaptureAction(root, "copy", () => {
      const text = JSON.stringify(readPayload(), null, 2);
      copyText(text);
      setStatus(root, `copied ${formatBytes(text.length)}`);
    });
  }

  function downloadPayload(root) {
    runCaptureAction(root, "download", () => {
      const text = JSON.stringify(readPayload(), null, 2);
      const blob = new Blob([text], { type: "application/json" });
      const filename = `harmonies-gamedatas-${Date.now()}.json`;
      const link = downloadLink(root);
      if (link.href) URL.revokeObjectURL(link.href);
      link.href = URL.createObjectURL(blob);
      link.download = filename;
      link.style.display = "inline";
      link.textContent = "Save";
      link.click();
      setTimeout(() => {
        setStatus(root, `download ready ${formatBytes(blob.size)}`);
      }, 250);
    });
  }

  function runCaptureAction(root, label, action) {
    setBusy(root, true);
    setStatus(root, `${label}: building...`);
    window.setTimeout(() => {
      try {
        action();
      } catch (error) {
        fail(root, error);
      } finally {
        setBusy(root, false);
      }
    }, 25);
  }

  function copyText(text) {
    if (typeof GM_setClipboard === "function") {
      GM_setClipboard(text);
      return;
    }
    if (typeof GM !== "undefined" && typeof GM.setClipboard === "function") {
      GM.setClipboard(text);
      return;
    }
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(text).catch((error) => {
        console.warn("[Harmonies Capture] async clipboard failed", error);
      });
      return;
    }
    const area = document.createElement("textarea");
    area.value = text;
    area.style.cssText = "position:fixed;left:-9999px;top:0";
    document.body.appendChild(area);
    area.focus();
    area.select();
    document.execCommand("copy");
    area.remove();
  }

  function setStatus(root, message) {
    root.querySelector("[data-role='status']").textContent = message;
  }

  function downloadLink(root) {
    return root.querySelector("[data-role='download-link']");
  }

  function setBusy(root, busy) {
    root.querySelectorAll("button").forEach((button) => {
      button.disabled = busy;
    });
  }

  function fail(root, error) {
    console.error("[Harmonies Capture]", error);
    setStatus(root, `error: ${error?.message || error}`);
  }

  function formatBytes(bytes) {
    if (!Number.isFinite(bytes)) return "";
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  window.setInterval(() => {
    rememberLatestGamedatas();
    installPanel();
  }, 1000);
  rememberLatestGamedatas();
  installPanel();
})();
