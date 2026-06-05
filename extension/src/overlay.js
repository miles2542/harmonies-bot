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
        <div class="harmonies-advisor-plans"></div>
      </div>
    `;
    document.documentElement.appendChild(root);
    const visualLayer = document.createElement("div");
    visualLayer.id = VISUAL_LAYER_ID;
    document.documentElement.appendChild(visualLayer);
    const renderedTierKeys = new Set();
    const plans = [];
    let selectedPlanKey = "";

    root.querySelector("[data-action='toggle']").addEventListener("click", () => {
      root.classList.toggle("is-collapsed");
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
        plans.length = 0;
        selectedPlanKey = "";
        clearVisualLayer();
        root.querySelector(".harmonies-advisor-plans").replaceChildren();
      },
      renderRecommendationTier(response) {
        this.setStatus(response.status);
        const bestMove = response.bestMove;
        const tierKey = tierId(response);
        if (!bestMove || renderedTierKeys.has(tierKey)) {
          if (!bestMove) {
            root.querySelector(".harmonies-advisor-plans").textContent = "No recommendation";
          }
          return;
        }
        renderedTierKeys.add(tierKey);

        const plan = {
          key: tierKey,
          index: plans.length + 1,
          move: bestMove,
          alternatives: response.alternatives || [],
          label: planLabel(response, plans.length),
        };
        plans.push(plan);
        root.querySelector(".harmonies-advisor-plans").appendChild(renderPlan(plan, selectPlan));
        if (!selectedPlanKey) {
          selectPlan(plan.key);
        }
      },
    };

    function selectPlan(planKey) {
      const plan = plans.find((candidate) => candidate.key === planKey);
      if (!plan) {
        return;
      }
      selectedPlanKey = planKey;
      root.querySelectorAll(".harmonies-advisor-plan").forEach((section) => {
        const selected = section.dataset.planKey === planKey;
        section.classList.toggle("is-selected", selected);
        const button = section.querySelector("[data-action='select-plan']");
        if (button) {
          button.textContent = selected ? "Showing indicators" : "Show indicators";
        }
      });
      clearVisualLayer();
      drawPlanOverlays(plan.move);
    }
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

  function renderPlan(plan, onSelect) {
    const section = document.createElement("section");
    section.className = "harmonies-advisor-plan";
    section.dataset.planKey = plan.key;

    const header = document.createElement("div");
    header.className = "harmonies-advisor-plan-header";
    const title = document.createElement("button");
    title.type = "button";
    title.className = "harmonies-advisor-plan-title";
    title.textContent = `${plan.label}: ${plan.move.title}`;
    const select = document.createElement("button");
    select.type = "button";
    select.className = "harmonies-advisor-plan-select";
    select.dataset.action = "select-plan";
    select.textContent = "Show indicators";

    const body = document.createElement("div");
    body.className = "harmonies-advisor-plan-body";
    const steps = document.createElement("ol");
    steps.replaceChildren(...(plan.move.steps || []).map(renderStep));
    body.appendChild(steps);

    if (plan.alternatives.length) {
      const alternatives = document.createElement("details");
      alternatives.className = "harmonies-advisor-plan-alternatives";
      const summary = document.createElement("summary");
      summary.textContent = "Alternative groups";
      const list = document.createElement("div");
      list.replaceChildren(...plan.alternatives.map(renderAlternative));
      alternatives.append(summary, list);
      body.appendChild(alternatives);
    }

    if (plan.index === 1) {
      section.classList.add("is-open");
    }
    title.addEventListener("click", () => section.classList.toggle("is-open"));
    select.addEventListener("click", () => onSelect(plan.key));
    header.append(title, select);
    section.append(header, body);
    return section;
  }

  function planLabel(response, existingPlanCount) {
    const progress = response.progress || {};
    const depth = progress.depthCompleted || 0;
    const elapsed = formatElapsed(response.elapsedMs ?? 0);
    if (response.isFinal) {
      return `Final best-so-far, depth ${depth}, ${elapsed}`;
    }
    if (existingPlanCount === 0) {
      return `Fast plan, depth ${depth}, ${elapsed}`;
    }
    if (depth > 0) {
      return `Deeper plan, depth ${depth}, ${elapsed}`;
    }
    return `Refined plan, depth ${depth}, ${elapsed}`;
  }

  function formatElapsed(ms) {
    if (ms >= 1000) {
      return `${(ms / 1000).toFixed(1)}s`;
    }
    return `${ms}ms`;
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
    overlay.style.left = `${rect.left + window.scrollX}px`;
    overlay.style.top = `${rect.top + window.scrollY}px`;
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
    marker.style.left = `${rect.left + window.scrollX + rect.width - 16}px`;
    marker.style.top = `${rect.top + window.scrollY + 4}px`;
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
