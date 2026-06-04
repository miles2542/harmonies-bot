(function harmoniesAdvisorOverlayModule() {
  const ROOT_ID = "harmonies-advisor-root";
  const HIGHLIGHT_CLASS = "harmonies-advisor-highlight";
  const CELL_HIGHLIGHT_CLASS = "harmonies-advisor-cell-highlight";
  const MARKER_CLASS = "harmonies-advisor-step-marker";

  function createOverlay() {
    const existing = document.getElementById(ROOT_ID);
    if (existing) {
      existing.remove();
    }

    const root = document.createElement("section");
    root.id = ROOT_ID;
    root.innerHTML = `
      <header class="harmonies-advisor-header">
        <strong>Harmonies Advisor</strong>
        <div class="harmonies-advisor-actions">
          <button type="button" data-action="analyze" title="Analyze current state">Analyze</button>
          <button type="button" data-action="stop" title="Stop search">Stop</button>
        </div>
        <button type="button" data-action="toggle" title="Collapse panel">_</button>
      </header>
      <div class="harmonies-advisor-body">
        <div class="harmonies-advisor-status">Starting</div>
        <div class="harmonies-advisor-best"></div>
        <ol class="harmonies-advisor-steps"></ol>
        <button type="button" class="harmonies-advisor-alt-toggle" data-action="alternatives">
          Alternatives
        </button>
        <div class="harmonies-advisor-alternatives"></div>
      </div>
    `;
    document.documentElement.appendChild(root);

    root.querySelector("[data-action='toggle']").addEventListener("click", () => {
      root.classList.toggle("is-collapsed");
    });
    root.querySelector("[data-action='alternatives']").addEventListener("click", () => {
      root.classList.toggle("show-alternatives");
    });

    return {
      onAnalyze(callback) {
        root.querySelector("[data-action='analyze']").addEventListener("click", callback);
      },
      onStop(callback) {
        root.querySelector("[data-action='stop']").addEventListener("click", callback);
      },
      setAnalyzeLabel(label) {
        root.querySelector("[data-action='analyze']").textContent = label;
      },
      setStatus(message) {
        root.querySelector(".harmonies-advisor-status").textContent = message;
      },
      renderRecommendation(response) {
        clearHighlights();
        this.setStatus(response.status);

        const best = root.querySelector(".harmonies-advisor-best");
        const steps = root.querySelector(".harmonies-advisor-steps");
        const alternatives = root.querySelector(".harmonies-advisor-alternatives");
        const responseAlternatives = response.alternatives || [];
        const hasAlternatives = responseAlternatives.length > 0;
        root.classList.toggle("has-alternatives", hasAlternatives);
        root.classList.toggle("show-alternatives", false);
        best.textContent = response.bestMove?.title || "No recommendation";
        steps.replaceChildren(...(response.bestMove?.steps || []).map(renderStep));
        alternatives.replaceChildren(...responseAlternatives.map(renderAlternative));

        if (response.bestMove?.centralGroupId) {
          highlightCentralGroup(response.bestMove.centralGroupId);
        }
        if (response.bestMove) {
          highlightBoardActions(response.bestMove);
        }
      },
    };
  }

  function renderStep(text) {
    const item = document.createElement("li");
    item.textContent = text;
    return item;
  }

  function renderAlternative(alternative) {
    const item = document.createElement("div");
    item.className = "harmonies-advisor-alt";
    item.textContent = alternative.label;
    return item;
  }

  function highlightCentralGroup(groupId) {
    const selectors = [
      `#hole-${CSS.escape(groupId)}`,
      `[data-hole='${CSS.escape(groupId)}']`,
      `[id$='_${CSS.escape(groupId)}'].token_group`,
    ];
    for (const selector of selectors) {
      const target = document.querySelector(selector);
      if (target) {
        target.classList.add(HIGHLIGHT_CLASS);
        return;
      }
    }
  }

  function highlightBoardActions(bestMove) {
    const playerId = bestMove.playerId;
    if (!playerId) {
      return;
    }
    (bestMove.actions || []).forEach((action, index) => {
      if (!["placeToken", "settleCard"].includes(action.kind)) {
        return;
      }
      const cell = findBoardCell(playerId, action.col, action.row);
      if (!cell) {
        return;
      }
      cell.classList.add(CELL_HIGHLIGHT_CLASS);
      appendStepMarker(cell, String(index + 1), action.kind);
    });
  }

  function findBoardCell(playerId, col, row) {
    const cellId = `cell_${playerId}_${col}_${row}`;
    return document.getElementById(cellId);
  }

  function appendStepMarker(cell, label, kind) {
    let marker = cell.querySelector(`.${MARKER_CLASS}`);
    if (!marker) {
      marker = document.createElement("span");
      marker.className = MARKER_CLASS;
      marker.dataset.kind = kind;
      cell.appendChild(marker);
    }
    marker.textContent = marker.textContent ? `${marker.textContent},${label}` : label;
  }

  function clearHighlights() {
    document
      .querySelectorAll(`.${HIGHLIGHT_CLASS}`)
      .forEach((node) => node.classList.remove(HIGHLIGHT_CLASS));
    document
      .querySelectorAll(`.${CELL_HIGHLIGHT_CLASS}`)
      .forEach((node) => node.classList.remove(CELL_HIGHLIGHT_CLASS));
    document.querySelectorAll(`.${MARKER_CLASS}`).forEach((node) => node.remove());
  }

  window.HarmoniesAdvisorOverlay = { createOverlay };
})();
