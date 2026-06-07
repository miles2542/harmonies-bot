(function harmoniesVisibleStateReaderModule() {
  const COLOR_BY_TYPE_ARG = {
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

  function readVisibleState(gamedatas, perspectivePlayerId) {
    const cardPointCounts = cardPointCountsByInstanceId(gamedatas);
    const players = Object.entries(gamedatas?.players || {}).map(([playerId, player], index) =>
      readVisiblePlayer(playerId, player, index + 1, gamedatas, cardPointCounts),
    );
    const centralTokenGroups = readDomCentralTokenGroups(gamedatas);
    const playerContainers = players.flatMap((player) => [player.handRect, player.doneRect]).filter(Boolean);
    const riverCards = readRiverCards(playerContainers, cardPointCounts);
    const domCardNodesSeen = allCardNodes().length > 0;
    const spiritChoicesByPlayerId = readSpiritChoicesByPlayerId(gamedatas, perspectivePlayerId, playerContainers, cardPointCounts);
    const notes = [];
    if (players.some((player) => !player.handRect)) {
      notes.push("missing hand container");
    }
    if (centralTokenGroups.length !== 5 || centralTokenGroups.some((group) => group.length !== 3)) {
      notes.push("incomplete central DOM groups");
    }
    return {
      schemaVersion: 1,
      capturedAt: new Date().toISOString(),
      activePlayerId: String(gamedatas?.gamestate?.active_player || ""),
      currentPlayerId: String(gamedatas?.player_id || gamedatas?.current_player_id || ""),
      perspectivePlayerId: String(perspectivePlayerId || ""),
      players: players.map(stripInternalRects),
      centralTokenGroups,
      riverCards,
      spiritChoicesByPlayerId,
      reliability: {
        domCards: domCardNodesSeen,
        domBoards: players.some((player) => player.cells.length > 0),
        domCentral: centralTokenGroups.length === 5 && centralTokenGroups.every((group) => group.length === 3),
        notes,
      },
    };
  }

  function readVisiblePlayer(playerId, player, order, gamedatas, cardPointCounts) {
    const hand = document.getElementById(`hand-${playerId}`);
    const done = document.getElementById(`done-${playerId}`);
    return {
      playerId,
      name: player?.name || player?.player_name || null,
      order,
      cells: readDomPlayerCells(playerId, gamedatas?.hexes || []),
      activeCards: hand ? cardsInsideContainer(hand, "hand", playerId, false, cardPointCounts) : [],
      completedCards: done ? cardsInsideContainer(done, "done", playerId, true, cardPointCounts) : [],
      handRect: visibleRectOrNull(hand),
      doneRect: visibleRectOrNull(done),
    };
  }

  function stripInternalRects(player) {
    return {
      playerId: player.playerId,
      name: player.name,
      order: player.order,
      cells: player.cells,
      activeCards: player.activeCards,
      completedCards: player.completedCards,
    };
  }

  function cardsInsideContainer(container, source, ownerPlayerId, completed, cardPointCounts) {
    const containerRect = container.getBoundingClientRect();
    if (!isVisibleRect(containerRect)) {
      return [];
    }
    return allCardNodes()
      .filter((node) => rectContainsCenter(containerRect, node.getBoundingClientRect()))
      .map((node) => readVisibleCard(node, source, ownerPlayerId, completed, cardPointCounts))
      .filter(Boolean)
      .sort(cardSortKey);
  }

  function readRiverCards(playerContainers, cardPointCounts) {
    const firstPlayerTop = Math.min(
      ...Array.from(document.querySelectorAll("[id^='player-table-']"))
        .map((node) => node.getBoundingClientRect())
        .filter(isVisibleRect)
        .map((rect) => rect.top),
    );
    if (!Number.isFinite(firstPlayerTop)) {
      return [];
    }
    return allCardNodes()
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        return (
          isVisibleRect(rect) &&
          rect.top < firstPlayerTop - 8 &&
          !playerContainers.some((container) => rectContainsCenter(container, rect))
        );
      })
      .map((node) => readVisibleCard(node, "river", null, false, cardPointCounts))
      .filter(Boolean)
      .sort(cardSortKey);
  }

  function readSpiritChoicesByPlayerId(gamedatas, perspectivePlayerId, playerContainers, cardPointCounts) {
    if (!isSpiritChoiceState(gamedatas)) {
      return {};
    }
    const playerId = String(perspectivePlayerId || gamedatas?.gamestate?.active_player || "");
    const choices = allCardNodes()
      .filter((node) => node.dataset?.isSpirit === "true")
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        return isVisibleRect(rect) && !playerContainers.some((container) => rectContainsCenter(container, rect));
      })
      .map((node) => readVisibleCard(node, "spiritChoice", playerId, false, cardPointCounts))
      .filter(Boolean)
      .sort(cardSortKey);
    return playerId && choices.length ? { [playerId]: choices } : {};
  }

  function readVisibleCard(node, source, ownerPlayerId, completed, cardPointCounts) {
    const cardInstanceId = numberValue(node.dataset?.cardId) || numberValue(cardIdFromNode(node));
    const typeArg = numberValue(node.dataset?.cardTypeArg);
    if (!Number.isFinite(cardInstanceId) || !Number.isFinite(typeArg)) {
      return null;
    }
    return {
      // cardInstanceId/cardId are BGA per-game instance ids. typeArg is persistent catalog id.
      cardInstanceId,
      cardId: cardInstanceId,
      typeArg,
      isSpirit: node.dataset?.isSpirit === "true",
      remainingCubes: completed ? 0 : remainingCubes(cardInstanceId, typeArg, cardPointCounts),
      source,
      ownerPlayerId,
      slotId: slotIdForCard(node),
      rect: roundedRect(node.getBoundingClientRect()),
    };
  }

  function slotIdForCard(node) {
    const slot = node.closest?.("[data-slot-id]");
    return slot?.dataset?.slotId || null;
  }

  function remainingCubes(cardInstanceId, typeArg, cardPointCounts) {
    const prefix = `card_${cardInstanceId}-score-`;
    const visibleCount = Array.from(document.querySelectorAll(`[id^='${CSS.escape(prefix)}']`)).filter((node) => {
      const className = String(node.className || "");
      return className.includes("points-location") && className.includes("animal-cube") && isVisibleElement(node);
    }).length;
    return visibleCount || cardPointCounts.get(cardInstanceId) || CUBE_COUNT_BY_CARD_TYPE_ARG[typeArg] || 0;
  }

  function cardPointCountsByInstanceId(gamedatas) {
    const counts = new Map();
    const add = (card) => {
      const cardId = numberValue(card?.id);
      const points = Array.isArray(card?.pointLocations) ? card.pointLocations.length : 0;
      if (Number.isFinite(cardId) && points > 0) {
        counts.set(cardId, points);
      }
    };
    Object.values(gamedatas?.players || {}).forEach((player) => {
      (Array.isArray(player?.boardAnimalCards) ? player.boardAnimalCards : []).forEach(add);
      (Array.isArray(player?.doneAnimalCards) ? player.doneAnimalCards : []).forEach(add);
    });
    (Array.isArray(gamedatas?.river) ? gamedatas.river : []).forEach(add);
    (Array.isArray(gamedatas?.spiritsCards) ? gamedatas.spiritsCards : []).forEach(add);
    return counts;
  }

  function readDomPlayerCells(playerId, hexes) {
    if (!playerId || !Array.isArray(hexes)) {
      return [];
    }
    const cells = [];
    for (const hex of hexes) {
      const col = Number(hex?.col);
      const row = Number(hex?.row);
      if (!Number.isFinite(col) || !Number.isFinite(row)) {
        continue;
      }
      const cell = document.getElementById(`cell_${playerId}_${col}_${row}`);
      if (!cell || !isVisibleElement(cell)) {
        continue;
      }
      cells.push({
        coord: { col, row },
        stack: { tokens: domTokensInCell(cell) },
        lockedByCube: domCubeInCell(cell),
      });
    }
    return cells;
  }

  function domTokensInCell(cell) {
    const rect = cell.getBoundingClientRect();
    return Array.from(document.querySelectorAll(".colored-token, [class*='colored-token']"))
      .filter((node) => isVisibleElement(node) && rectContainsCenter(rect, node.getBoundingClientRect()))
      .map((node) => ({ color: domTokenColor(node), level: domTokenLevel(node) }))
      .filter((token) => token.color)
      .sort((left, right) => left.level - right.level)
      .map((token) => token.color);
  }

  function domCubeInCell(cell) {
    const rect = cell.getBoundingClientRect();
    return Array.from(document.querySelectorAll(".animal-cube, [class*='animal-cube']")).some(
      (node) => {
        if (!isVisibleElement(node)) return false;
        const cubeRect = node.getBoundingClientRect();
        const center_x = cubeRect.left + cubeRect.width / 2;
        const center_y = cubeRect.top + cubeRect.height / 2;
        return (
          center_x >= rect.left &&
          center_x <= rect.right &&
          center_y >= rect.top - 65 &&
          center_y <= rect.top + rect.height * 0.6
        );
      }
    );
  }

  function readDomCentralTokenGroups(gamedatas) {
    const groups = [];
    for (let groupId = 1; groupId <= 5; groupId += 1) {
      const hole = document.getElementById(`hole-${groupId}`);
      const tokens = hole ? centralTokenNodes(hole, groupId).map(domTokenColor).filter(Boolean) : [];
      groups.push(tokens);
    }
    if (groups.length === 5 && groups.every((tokens) => tokens.length === 3)) {
      return groups;
    }
    return compactCentralGroups(gamedatas?.tokensOnCentralBoard).map(([, tokens]) => tokens);
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

  function compactCentralGroups(groups) {
    return Object.entries(groups || {})
      .sort(([left], [right]) => Number.parseInt(left, 10) - Number.parseInt(right, 10))
      .map(([key, tokens]) => [key, (Array.isArray(tokens) ? tokens : []).map((token) => colorByTypeArg(token?.type_arg)).filter(Boolean)]);
  }

  function allCardNodes() {
    return Array.from(document.querySelectorAll(".harmonies-card[id^='card_'], [data-card-id][data-card-type-arg]")).filter(
      isVisibleElement,
    );
  }

  function tokenNodeSortKey(node) {
    const match = /-token-(\d+)/.exec(String(node.id || ""));
    return match ? Number.parseInt(match[1], 10) : 99;
  }

  function cardSortKey(left, right) {
    const leftSlot = String(left.slotId || "").localeCompare(String(right.slotId || ""));
    if (leftSlot !== 0) {
      return leftSlot;
    }
    return left.rect.x - right.rect.x || left.rect.y - right.rect.y;
  }

  function isSpiritChoiceState(gamedatas) {
    const args = gamedatas?.gamestate?.args || {};
    const actions = Array.isArray(gamedatas?.gamestate?.possibleactions) ? gamedatas.gamestate.possibleactions : [];
    const text = `${gamedatas?.gamestate?.name || ""} ${gamedatas?.gamestate?.description || ""}`.toLowerCase();
    return (
      args.canChooseSpirit === true ||
      args.canChooseSpirit === "true" ||
      actions.includes("actChooseSpirit") ||
      actions.includes("chooseSpirit") ||
      (text.includes("choose one") && text.includes("spirit"))
    );
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

  function domTokenLevel(node) {
    const className = String(node.className || "");
    const matches = Array.from(className.matchAll(/(?:^|\s)level-(\d+)(?:\s|$)/g));
    const last = matches.at(-1);
    return last ? Number.parseInt(last[1], 10) : 1;
  }

  function colorByTypeArg(typeArg) {
    return COLOR_BY_TYPE_ARG[Number(typeArg)];
  }

  function cardIdFromNode(node) {
    const match = /^card_(\d+)$/.exec(String(node.id || ""));
    return match ? match[1] : "";
  }

  function rectContainsCenter(container, child) {
    const x = child.left + child.width / 2;
    const y = child.top + child.height / 2;
    return x >= container.left && x <= container.right && y >= container.top && y <= container.bottom;
  }

  function isVisibleRect(rect) {
    return rect.width > 2 && rect.height > 2;
  }

  function visibleRectOrNull(node) {
    if (!node) {
      return null;
    }
    const rect = node.getBoundingClientRect();
    return isVisibleRect(rect) ? rect : null;
  }

  function roundedRect(rect) {
    return {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };
  }

  function numberValue(value) {
    if (typeof value === "number") {
      return value;
    }
    if (typeof value === "string" && value.trim() !== "") {
      return Number(value);
    }
    return Number.NaN;
  }

  window.HarmoniesVisibleStateReader = { readVisibleState };
})();
