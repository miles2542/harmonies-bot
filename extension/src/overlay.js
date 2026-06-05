(function harmoniesAdvisorOverlayModule() {
  const ROOT_ID = "harmonies-advisor-root";
  const VISUAL_LAYER_ID = "harmonies-advisor-visual-layer";

  function createOverlay() {
    const existing = document.getElementById(ROOT_ID);
    if (existing) {
      existing.remove();
    }
    const existingLayer = document.getElementById(VISUAL_LAYER_ID);
    if (existingLayer) {
      existingLayer.remove();
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
        <div class="harmonies-advisor-tiers"></div>
        <button type="button" class="harmonies-advisor-alt-toggle" data-action="alternatives">
          Alternatives
        </button>
        <div class="harmonies-advisor-alternatives"></div>
      </div>
    `;
    document.documentElement.appendChild(root);
    const visualLayer = document.createElement("div");
    visualLayer.id = VISUAL_LAYER_ID;
    document.documentElement.appendChild(visualLayer);
    const renderedTierKeys = new Set();
    let primaryRendered = false;

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
      beginAnalysis() {
        renderedTierKeys.clear();
        primaryRendered = false;
        clearVisualLayer();
        root.classList.remove("has-alternatives", "show-alternatives");
        root.querySelector(".harmonies-advisor-best").textContent = "";
        root.querySelector(".harmonies-advisor-steps").replaceChildren();
        root.querySelector(".harmonies-advisor-tiers").replaceChildren();
        root.querySelector(".harmonies-advisor-alternatives").replaceChildren();
      },
      renderRecommendationTier(response) {
        this.setStatus(response.status);
        const bestMove = response.bestMove;
        const tierKey = tierId(response);
        if (!bestMove || renderedTierKeys.has(tierKey)) {
          if (!bestMove) {
            root.querySelector(".harmonies-advisor-best").textContent = "No recommendation";
          }
          return;
        }
        renderedTierKeys.add(tierKey);

        const best = root.querySelector(".harmonies-advisor-best");
        const steps = root.querySelector(".harmonies-advisor-steps");
        const alternatives = root.querySelector(".harmonies-advisor-alternatives");
        const tiers = root.querySelector(".harmonies-advisor-tiers");
        const responseAlternatives = response.alternatives || [];
        const hasAlternatives = responseAlternatives.length > 0;
        root.classList.toggle("has-alternatives", hasAlternatives || root.classList.contains("has-alternatives"));
        root.classList.toggle("show-alternatives", false);

        if (!primaryRendered) {
          primaryRendered = true;
          best.textContent = bestMove.title;
          steps.replaceChildren(...(bestMove.steps || []).map(renderStep));
          alternatives.replaceChildren(...responseAlternatives.map(renderAlternative));
          clearVisualLayer();
          drawPlanOverlays(bestMove);
          return;
        }
        tiers.appendChild(renderTier(response));
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

  function renderTier(response) {
    const tier = document.createElement("section");
    tier.className = "harmonies-advisor-tier";
    const title = document.createElement("button");
    title.type = "button";
    title.className = "harmonies-advisor-tier-title";
    title.textContent = `${tierLabel(response)}: ${response.bestMove.title}`;
    const body = document.createElement("div");
    body.className = "harmonies-advisor-tier-body";
    const steps = document.createElement("ol");
    steps.replaceChildren(...(response.bestMove.steps || []).map(renderStep));
    const alternatives = document.createElement("div");
    alternatives.className = "harmonies-advisor-tier-alternatives";
    alternatives.replaceChildren(...(response.alternatives || []).map(renderAlternative));
    body.append(steps, alternatives);
    title.addEventListener("click", () => tier.classList.toggle("is-open"));
    tier.append(title, body);
    return tier;
  }

  function tierLabel(response) {
    const progress = response.progress || {};
    const depth = progress.depthCompleted || 0;
    const elapsed = response.elapsedMs ?? 0;
    return `Depth ${depth}, ${elapsed}ms`;
  }

  function tierId(response) {
    const progress = response.progress || {};
    return [
      response.elapsedMs ?? 0,
      progress.depthCompleted || 0,
      progress.nodesEvaluated || 0,
      response.bestMove?.title || "",
    ].join(":");
  }

  function drawPlanOverlays(bestMove) {
    if (bestMove.centralGroupId) {
      drawCentralGroupOverlay(bestMove.centralGroupId);
    }
    drawBoardActionOverlays(bestMove);
  }

  function drawCentralGroupOverlay(groupId) {
    const selectors = [
      `#hole-${CSS.escape(groupId)}`,
      `[data-hole='${CSS.escape(groupId)}']`,
      `[id$='_${CSS.escape(groupId)}'].token_group`,
    ];
    for (const selector of selectors) {
      const target = document.querySelector(selector);
      if (target) {
        addRectOverlay(target, "harmonies-advisor-group-ring");
        return;
      }
    }
  }

  function drawBoardActionOverlays(bestMove) {
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
      addRectOverlay(cell, "harmonies-advisor-cell-ring");
      addStepMarker(cell, String(index + 1), action.kind);
    });
  }

  function findBoardCell(playerId, col, row) {
    const cellId = `cell_${playerId}_${col}_${row}`;
    return document.getElementById(cellId);
  }

  function addRectOverlay(target, className) {
    const rect = target.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return;
    }
    const overlay = document.createElement("div");
    overlay.className = className;
    overlay.style.left = `${rect.left}px`;
    overlay.style.top = `${rect.top}px`;
    overlay.style.width = `${rect.width}px`;
    overlay.style.height = `${rect.height}px`;
    visualLayer().appendChild(overlay);
  }

  function addStepMarker(cell, label, kind) {
    const rect = cell.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return;
    }
    const marker = document.createElement("span");
    marker.className = "harmonies-advisor-step-marker";
    marker.dataset.kind = kind;
    marker.textContent = label;
    marker.style.left = `${rect.left + rect.width - 16}px`;
    marker.style.top = `${rect.top + 4}px`;
    visualLayer().appendChild(marker);
  }

  function visualLayer() {
    return document.getElementById(VISUAL_LAYER_ID);
  }

  function clearVisualLayer() {
    visualLayer()?.replaceChildren();
  }

  window.HarmoniesAdvisorOverlay = { createOverlay };
})();
