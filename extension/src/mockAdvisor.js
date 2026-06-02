(function harmoniesMockAdvisorModule() {
  const COLOR_NAMES = {
    1: "Water",
    2: "Mountain",
    3: "Trunk",
    4: "Foliage",
    5: "Field",
    6: "Building",
    7: "Building",
  };

  function normalizeCentralGroups(gamedatas) {
    const source = gamedatas?.tokensOnCentralBoard || {};
    return Object.entries(source).map(([groupId, tokens]) => ({
      groupId,
      tokens: Array.isArray(tokens) ? tokens.map((token) => COLOR_NAMES[token.type_arg] || "Unknown") : [],
    }));
  }

  function recommend(gamedatas) {
    const groups = normalizeCentralGroups(gamedatas);
    const chosen = groups.find((group) => group.tokens.length > 0);
    if (!chosen) {
      return {
        status: "No token groups found",
        bestMove: null,
        alternatives: [],
      };
    }

    return {
      status: "Mock advisor active",
      bestMove: {
        centralGroupId: chosen.groupId,
        title: `Take group ${chosen.groupId}`,
        steps: [
          `Take: ${chosen.tokens.join(", ")}`,
          "Rust engine not connected yet",
          "Use as extractor/overlay smoke test",
        ],
      },
      alternatives: groups.slice(0, 3).map((group) => ({
        centralGroupId: group.groupId,
        label: `Group ${group.groupId}: ${group.tokens.join(", ")}`,
      })),
    };
  }

  window.HarmoniesMockAdvisor = { recommend };
})();
