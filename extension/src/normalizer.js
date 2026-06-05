(function harmoniesBgaNormalizerModule() {
  const COLOR_BY_TYPE_ARG = {
    1: "water",
    2: "mountain",
    3: "trunk",
    4: "foliage",
    5: "field",
    6: "building",
    7: "building",
  };

  const TODO_GAPS = [
    "No parity tests against Rust normalizer yet.",
    "Card pattern/catalog details omitted because GameSnapshotV1 only needs card ids, type args, cube counts.",
  ];

  function normalizeGamedatas(gamedatas, perspectivePlayerId, options = {}) {
    if (!isObject(gamedatas)) {
      throw new Error("gamedatas must be an object");
    }
    const playersById = objectValue(gamedatas.players);
    if (!playersById) {
      throw new Error("players missing or invalid");
    }
    const hexes = parseHexes(gamedatas.hexes);
    if (!hexes) {
      throw new Error("hexes missing or invalid");
    }

    const playerIds = mapPlayerIds(gamedatas, playersById);
    const perspective = resolvePerspective(gamedatas, playersById, playerIds, perspectivePlayerId);
    const activePlayerId = normalizePlayerId(readActivePlayerId(gamedatas), playerIds);
    const allCubeLocations = collectAllCubeLocations(gamedatas, playersById);
    const cardCubeCounts = countCardCubes(gamedatas.cubesOnAnimalCards);
    const centralTokenGroups =
      Array.isArray(options.centralTokenGroups) && options.centralTokenGroups.length
        ? options.centralTokenGroups
        : parseCentralGroups(gamedatas.tokensOnCentralBoard);
    const boardCellsByPlayerId = objectValue(options.boardCellsByPlayerId) || {};
    const players = Object.entries(playersById).map(([playerId, player]) =>
      normalizePlayer(
        playerId,
        player,
        gamedatas,
        hexes,
        allCubeLocations,
        cardCubeCounts,
        playerIds,
        boardCellsByPlayerId,
      ),
    );

    return {
      schemaVersion: 1,
      perspectivePlayerId: perspective,
      activePlayerId,
      boardSide: parseBoardSide(gamedatas.boardSide),
      players,
      centralTokenGroups,
      riverCards: parseCards(gamedatas.river, new Map(), false, { locations: ["river"] }),
      bagCounts: inferBagCounts(players, centralTokenGroups, gamedatas.remainingTokens),
      cardsCatalogVersion: stringValue(gamedatas.version) || "bga",
    };
  }

  function normalizePlayer(
    playerId,
    player,
    gamedatas,
    hexes,
    allCubeLocations,
    cardCubeCounts,
    playerIds,
    boardCellsByPlayerId,
  ) {
    const bgaIds = bgaIdsForPlayer(playerId, player, gamedatas, playerIds);
    const tokenStacks = parseTokensOnBoard(player.tokensOnBoard);
    const playerCubeLocations = new Set(allCubeLocations);
    collectSinglePlayerCubeLocations(player, playerCubeLocations);
    const playerCubeCoords = collectSinglePlayerCubeCoords(player);
    const domCells = domCellsForPlayer(boardCellsByPlayerId, playerId, bgaIds);
    const cells = hexes.map((coord) => {
      const key = bgaIds
        .map((id) => cellKey(id, coord))
        .find((candidate) => tokenStacks.has(candidate) || playerCubeLocations.has(candidate));
      const coordKey = `${coord.col},${coord.row}`;
      const domCell = domCells.get(coordKey);
      const lockedByCube = Boolean((key && playerCubeLocations.has(key)) || playerCubeCoords.has(coordKey));
      return {
        coord,
        stack: { tokens: domCell ? domCell.tokens : key ? tokenStacks.get(key) || [] : [] },
        lockedByCube: lockedByCube || Boolean(domCell?.lockedByCube),
      };
    });
    const activeCards = parseCards(player.boardAnimalCards, cardCubeCounts, false, {
      locations: bgaIds.map((id) => `board${id}`),
    });
    const playerSpirits = parsePlayerSpirits(
      gamedatas.spiritsCards,
      playerId,
      bgaIds,
      cardCubeCounts,
      gamedatas,
      playerIds,
    );
    activeCards.push(...playerSpirits);
    const spiritCardChoices = playerSpirits.length
      ? []
      : parsePlayerSpiritChoices(gamedatas.spiritsCards, playerId, bgaIds, cardCubeCounts, gamedatas, playerIds);

    return {
      playerId,
      cells,
      activeCards,
      spiritCardChoices,
      completedCards: parseCards(player.doneAnimalCards, cardCubeCounts, true, {
        locations: bgaIds.map((id) => `done${id}`),
      }),
      emptyHexes: domCells.size
        ? clampU8(cells.filter((cell) => !cell.stack.tokens.length).length)
        : clampU8(numberValue(player.emptyHexes) || 0),
    };
  }

  function resolvePerspective(gamedatas, playersById, playerIds, perspectivePlayerId) {
    const raw =
      perspectivePlayerId ||
      stringValue(gamedatas.player_id) ||
      stringValue(gamedatas.current_player_id) ||
      readActivePlayerId(gamedatas);
    const mapped = normalizePlayerId(raw, playerIds);
    if (mapped && Object.prototype.hasOwnProperty.call(playersById, mapped)) {
      return mapped;
    }
    return Object.keys(playersById)[0] || "";
  }

  function mapPlayerIds(gamedatas, playersById) {
    const ids = new Map();
    Object.entries(playersById).forEach(([key, player]) => {
      ids.set(key, key);
      const id = stringValue(player.id);
      if (id) {
        ids.set(id, key);
      }
      const orderId = playerOrderIdForPlayer(gamedatas, player);
      if (orderId) {
        ids.set(orderId, key);
      }
      inferPlayerIdsFromLocations(player).forEach((inferred) => ids.set(inferred, key));
    });
    return ids;
  }

  function bgaIdsForPlayer(playerId, player, gamedatas, mappedIds) {
    const ids = [playerId, stringValue(player.id), playerOrderIdForPlayer(gamedatas, player)];
    ids.push(...inferPlayerIdsFromLocations(player));
    mappedIds.forEach((mapped, raw) => {
      if (mapped === playerId) {
        ids.push(raw);
      }
    });
    return Array.from(new Set(ids.filter(Boolean))).sort();
  }

  function inferPlayerIdsFromLocations(player) {
    const ids = [];
    const tokens = objectValue(player.tokensOnBoard);
    if (tokens) {
      Object.keys(tokens).forEach((key) => {
        const playerId = cellKeyPlayerId(key);
        if (playerId) {
          ids.push(playerId);
        }
      });
    }
    const cubeLocations = new Set();
    collectSinglePlayerCubeLocations(player, cubeLocations);
    cubeLocations.forEach((location) => {
      const playerId = cellKeyPlayerId(location);
      if (playerId) {
        ids.push(playerId);
      }
    });
    return Array.from(new Set(ids.filter(Boolean))).sort();
  }

  function parseHexes(value) {
    if (!Array.isArray(value)) {
      return null;
    }
    return value
      .map((hex) => {
        const col = numberValue(hex?.col);
        const row = numberValue(hex?.row);
        return Number.isFinite(col) && Number.isFinite(row) ? { col, row } : null;
      })
      .filter(Boolean);
  }

  function parseBoardSide(value) {
    return stringValue(value) === "sideB" || stringValue(value) === "SideB" ? "sideB" : "sideA";
  }

  function parseCentralGroups(value) {
    const groups = objectValue(value);
    if (!groups) {
      return [];
    }
    return Object.entries(groups)
      .sort(([left], [right]) => Number.parseInt(left, 10) - Number.parseInt(right, 10))
      .map(([, tokens]) => parseTokenList(tokens));
  }

  function parseTokensOnBoard(value) {
    const stacks = new Map();
    if (Array.isArray(value)) {
      value.forEach((token) => addTokenToStack(stacks, stringValue(token.location), token));
    } else if (isObject(value)) {
      Object.entries(value).forEach(([cell, tokens]) => {
        arrayValue(tokens).forEach((token) => addTokenToStack(stacks, cell, token));
      });
    }
    const normalized = new Map();
    stacks.forEach((tokens, cell) => {
      tokens.sort((left, right) => left.level - right.level);
      normalized.set(
        cell,
        tokens.map((token) => token.color),
      );
    });
    return normalized;
  }

  function addTokenToStack(stacks, cell, token) {
    const color = tokenColor(token);
    if (!cell || !color) {
      return;
    }
    const level = numberValue(token.location_arg) || 1;
    const stack = stacks.get(cell) || [];
    stack.push({ level, color });
    stacks.set(cell, stack);
  }

  function parseTokenList(value) {
    return arrayValue(value).map(tokenColor).filter(Boolean);
  }

  function domCellsForPlayer(boardCellsByPlayerId, playerId, bgaIds) {
    const rawCells = [playerId, ...bgaIds].map((id) => boardCellsByPlayerId[id]).find(Array.isArray) || [];
    const cells = new Map();
    rawCells.forEach((cell) => {
      const col = numberValue(cell?.coord?.col);
      const row = numberValue(cell?.coord?.row);
      const tokens = arrayValue(cell?.stack?.tokens || cell?.tokens).filter((token) =>
        Object.values(COLOR_BY_TYPE_ARG).includes(token),
      );
      if (Number.isFinite(col) && Number.isFinite(row)) {
        cells.set(`${col},${row}`, {
          tokens,
          lockedByCube: Boolean(cell.lockedByCube),
        });
      }
    });
    return cells;
  }

  function parseCards(value, cubeCounts, completed, options = {}) {
    const locations = options.locations || [];
    return arrayValue(value)
      .map((card) => {
        const cardId = numberValue(card.id);
        const typeArg = numberValue(card.type_arg);
        if (!Number.isFinite(cardId) || !Number.isFinite(typeArg)) {
          return null;
        }
        if (!cardLocationMatches(card, locations)) {
          return null;
        }
        return {
          cardId,
          typeArg,
          remainingCubes: completed ? 0 : remainingCubeCount(card, cardId, cubeCounts),
          isSpirit: Boolean(card.isSpirit),
        };
      })
      .filter(Boolean);
  }

  function parsePlayerSpirits(value, playerId, bgaIds, cubeCounts, gamedatas, playerIds) {
    const inChoiceState = isSpiritChoiceState(gamedatas, playerId, playerIds);
    return arrayValue(value)
      .filter((card) => valueMatchesAnyId(card.location_arg, bgaIds))
      .map((card) => {
        const cardId = numberValue(card.id);
        const typeArg = numberValue(card.type_arg);
        if (!Number.isFinite(cardId) || !Number.isFinite(typeArg)) {
          return null;
        }
        if (inChoiceState || !cubeCounts.has(cardId)) {
          return null;
        }
        return {
          cardId,
          typeArg,
          remainingCubes: cubeCounts.get(cardId),
          isSpirit: true,
        };
      })
      .filter(Boolean);
  }

  function parsePlayerSpiritChoices(value, playerId, bgaIds, cubeCounts, gamedatas, playerIds) {
    if (!isSpiritChoiceState(gamedatas, playerId, playerIds)) {
      return [];
    }
    return arrayValue(value)
      .filter((card) => valueMatchesAnyId(card.location_arg, bgaIds))
      .map((card) => {
        const cardId = numberValue(card.id);
        const typeArg = numberValue(card.type_arg);
        if (!Number.isFinite(cardId) || !Number.isFinite(typeArg) || cubeCounts.has(cardId)) {
          return null;
        }
        return {
          cardId,
          typeArg,
          remainingCubes: 1,
          isSpirit: true,
        };
      })
      .filter(Boolean);
  }

  function isSpiritChoiceState(gamedatas, playerId, playerIds) {
    const args = gamedatas?.gamestate?.args || {};
    if (args.canChooseSpirit !== true && args.canChooseSpirit !== "true") {
      return false;
    }
    const activePlayer = normalizePlayerId(readActivePlayerId(gamedatas), playerIds || new Map());
    if (activePlayer && playerId && activePlayer !== playerId) {
      return false;
    }
    const possibleActions = arrayValue(gamedatas?.gamestate?.possibleactions);
    if (possibleActions.includes("actChooseSpirit") || possibleActions.includes("chooseSpirit")) {
      return true;
    }
    const text = `${gamedatas?.gamestate?.name || ""} ${
      gamedatas?.gamestate?.description || ""
    }`.toLowerCase();
    return text.includes("choose one") && text.includes("spirit");
  }

  function remainingCubeCount(card, cardId, cubeCounts) {
    if (cubeCounts.has(cardId)) {
      return cubeCounts.get(cardId);
    }
    return arrayValue(card.pointLocations).length;
  }

  function cardLocationMatches(card, locations) {
    if (!locations.length) {
      return true;
    }
    const location = stringValue(card.location);
    return !location || locations.includes(location);
  }

  function countCardCubes(value) {
    const counts = new Map();
    arrayValue(value).forEach((cube) => {
      const location = stringValue(cube.location);
      const rawId = location?.startsWith("card_") ? location.slice("card_".length) : "";
      const cardId = Number.parseInt(rawId, 10);
      if (Number.isFinite(cardId)) {
        counts.set(cardId, (counts.get(cardId) || 0) + 1);
      }
    });
    return counts;
  }

  function inferBagCounts(players, centralTokenGroups, remainingTokens) {
    const counts = {
      water: 23,
      mountain: 23,
      trunk: 21,
      foliage: 19,
      field: 19,
      building: 15,
      unknown: 0,
    };
    players
      .flatMap((player) => player.cells)
      .flatMap((cell) => cell.stack.tokens)
      .concat(centralTokenGroups.flat())
      .forEach((color) => {
        counts[color] = Math.max(0, (counts[color] || 0) - 1);
      });
    const reported = numberValue(remainingTokens);
    if (Number.isFinite(reported)) {
      const known = counts.water + counts.mountain + counts.trunk + counts.foliage + counts.field + counts.building;
      counts.unknown = Math.max(0, Math.trunc(reported) - known);
    }
    return counts;
  }

  function collectAllCubeLocations(gamedatas, playersById) {
    const locations = collectCubeLocations(gamedatas.cubesOnAnimalCards);
    Object.values(playersById).forEach((player) => collectSinglePlayerCubeLocations(player, locations));
    return locations;
  }

  function collectCubeLocations(value) {
    const locations = new Set();
    arrayValue(value).forEach((cube) => {
      const location = stringValue(cube.location);
      if (location?.startsWith("cell_")) {
        locations.add(location);
      }
    });
    return locations;
  }

  function collectSinglePlayerCubeLocations(player, locations) {
    const cubes = player.animalCubesOnBoard;
    if (Array.isArray(cubes)) {
      cubes.filter((location) => typeof location === "string").forEach((location) => locations.add(location));
    } else if (isObject(cubes)) {
      Object.keys(cubes).forEach((location) => locations.add(location));
    }
  }

  function collectSinglePlayerCubeCoords(player) {
    const locations = new Set();
    collectSinglePlayerCubeLocations(player, locations);
    return new Set(Array.from(locations).map(cellKeyCoord).filter(Boolean).map((coord) => `${coord.col},${coord.row}`));
  }

  function tokenColor(token) {
    return COLOR_BY_TYPE_ARG[numberValue(token?.type_arg)];
  }

  function readActivePlayerId(gamedatas) {
    const activePlayer = gamedatas.gamestate?.active_player;
    if (activePlayer === undefined || activePlayer === null) {
      return "";
    }
    return String(activePlayer);
  }

  function normalizePlayerId(rawId, mappedIds) {
    return rawId ? mappedIds.get(String(rawId)) || String(rawId) : "";
  }

  function valueMatchesAnyId(value, ids) {
    if (value === undefined || value === null) {
      return false;
    }
    const raw = String(value);
    return ids.some((id) => raw === id);
  }

  function playerOrderIdForPlayer(gamedatas, player) {
    const playerNo = numberValue(player.playerNo);
    if (!playerNo) {
      return "";
    }
    const value = Array.isArray(gamedatas.playerorder) ? gamedatas.playerorder[playerNo - 1] : undefined;
    return value === undefined || value === null ? "" : String(value);
  }

  function cellKey(playerId, coord) {
    return `cell_${playerId}_${coord.col}_${coord.row}`;
  }

  function cellKeyPlayerId(key) {
    const match = /^cell_(.+)_-?\d+_-?\d+$/.exec(key);
    return match?.[1] || "";
  }

  function cellKeyCoord(key) {
    const match = /^cell_.+_(-?\d+)_(-?\d+)$/.exec(key);
    if (!match) {
      return null;
    }
    return { col: Number.parseInt(match[1], 10), row: Number.parseInt(match[2], 10) };
  }

  function arrayValue(value) {
    return Array.isArray(value) ? value : [];
  }

  function objectValue(value) {
    return isObject(value) ? value : null;
  }

  function isObject(value) {
    return Boolean(value && typeof value === "object" && !Array.isArray(value));
  }

  function stringValue(value) {
    return typeof value === "string" ? value : "";
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

  function clampU8(value) {
    return Math.max(0, Math.min(255, Math.trunc(value)));
  }

  window.HarmoniesBgaNormalizer = { TODO_GAPS, normalizeGamedatas };
})();
