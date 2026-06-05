const assert = require("node:assert/strict");
const path = require("node:path");

global.window = {};
require(path.join("..", "extension", "src", "normalizer.js"));

const normalizer = global.window.HarmoniesBgaNormalizer;

function baseGamedatas() {
  return {
    version: "test",
    boardSide: "sideA",
    remainingTokens: 80,
    gamestate: { name: "chooseAction", active_player: "p1" },
    hexes: [
      { col: 0, row: 0 },
      { col: 1, row: 0 },
    ],
    tokensOnCentralBoard: {
      1: [
        { type_arg: 1 },
        { type_arg: 2 },
        { type_arg: 3 },
      ],
    },
    river: [
      {
        id: 11,
        type_arg: 6,
        location: "river",
        pointLocations: [2, 4, 8, 13],
        isSpirit: false,
      },
    ],
    cubesOnAnimalCards: [{ location: "card_9" }, { location: "card_9" }],
    players: {
      p1: {
        id: "p1",
        playerNo: 1,
        tokensOnBoard: {},
        animalCubesOnBoard: {},
        boardAnimalCards: [
          {
            id: 9,
            type_arg: 5,
            location: "boardp1",
            pointLocations: [2, 4, 6],
            isSpirit: false,
          },
        ],
        doneAnimalCards: [],
        emptyHexes: 2,
      },
    },
  };
}

function visibleState(overrides = {}) {
  return {
    schemaVersion: 1,
    activePlayerId: "p1",
    players: [
      {
        playerId: "p1",
        cells: [
          { coord: { col: 0, row: 0 }, stack: { tokens: ["water"] }, lockedByCube: false },
          { coord: { col: 1, row: 0 }, stack: { tokens: [] }, lockedByCube: false },
        ],
        activeCards: [],
        completedCards: [],
        ...overrides.player,
      },
    ],
    centralTokenGroups: [["field", "field", "trunk"]],
    riverCards: [{ cardId: 12, cardInstanceId: 12, typeArg: 7, remainingCubes: 3, isSpirit: false }],
    spiritChoicesByPlayerId: {},
    reliability: { domCards: true, domBoards: true, domCentral: true, notes: [] },
    ...overrides.state,
  };
}

function activePlayer(snapshot) {
  return snapshot.players.find((player) => player.playerId === "p1");
}

function testDomCardsFalseFallsBackToGamedatasCards() {
  const snapshot = normalizer.normalizeGamedatas(baseGamedatas(), "p1", {
    visibleStateV1: visibleState({
      state: { reliability: { domCards: false, domBoards: true, domCentral: true, notes: [] } },
    }),
  });
  assert.deepEqual(activePlayer(snapshot).activeCards, [
    { cardId: 9, typeArg: 5, remainingCubes: 2, isSpirit: false },
  ]);
  assert.deepEqual(snapshot.riverCards, [
    { cardId: 11, typeArg: 6, remainingCubes: 4, isSpirit: false },
  ]);
  assert.deepEqual(activePlayer(snapshot).cells[0].stack.tokens, ["water"]);
}

function testDomCardsTrueOverridesStaleGamedatasCards() {
  const snapshot = normalizer.normalizeGamedatas(baseGamedatas(), "p1", {
    visibleStateV1: visibleState({
      player: {
        activeCards: [
          {
            cardId: 20,
            cardInstanceId: 20,
            typeArg: 9,
            remainingCubes: 3,
            isSpirit: false,
          },
        ],
      },
    }),
  });
  assert.deepEqual(activePlayer(snapshot).activeCards, [
    { cardId: 20, typeArg: 9, remainingCubes: 3, isSpirit: false },
  ]);
  assert.deepEqual(snapshot.riverCards, [
    { cardId: 12, typeArg: 7, remainingCubes: 3, isSpirit: false },
  ]);
}

testDomCardsFalseFallsBackToGamedatasCards();
testDomCardsTrueOverridesStaleGamedatasCards();
console.log("extension normalizer visible state checks ok");
