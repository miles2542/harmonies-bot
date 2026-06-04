(function harmoniesMockAdvisorModule() {
  const COLOR_LABELS = {
    water: "Water",
    mountain: "Mountain",
    trunk: "Trunk",
    foliage: "Foliage",
    field: "Field",
    building: "Building",
  };

  function normalizeCentralGroups(snapshot) {
    const groups = snapshot?.centralTokenGroups || [];
    return groups.map((tokens, index) => ({
      groupId: String(index + 1),
      tokens: Array.isArray(tokens) ? tokens.map(labelColor) : [],
    }));
  }

  function recommend(snapshot) {
    const groups = normalizeCentralGroups(snapshot);
    const chosen = groups.find((group) => group.tokens.length > 0);
    const perspective = (snapshot?.players || []).find(
      (player) => player.playerId === snapshot.perspectivePlayerId,
    );
    const spiritChoice = perspective?.spiritCardChoices?.[0];
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
          ...(spiritChoice
            ? [`Choose Spirit card ${spiritChoice.typeArg} (id ${spiritChoice.cardId})`]
            : []),
          `Take: ${chosen.tokens.join(", ")}`,
          `Perspective: ${snapshot.perspectivePlayerId}`,
          `Active player: ${snapshot.activePlayerId}`,
          "Rust engine not connected yet",
          "Snapshot normalized in extension",
        ],
      },
      alternatives: groups.slice(0, 3).map((group) => ({
        centralGroupId: group.groupId,
        label: `Group ${group.groupId}: ${group.tokens.join(", ")}`,
      })),
    };
  }

  function labelColor(color) {
    return COLOR_LABELS[color] || "Unknown";
  }

  window.HarmoniesMockAdvisor = { recommend };
})();
