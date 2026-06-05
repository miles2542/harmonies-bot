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
        updateBestPlanBadges();
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

    function updateBestPlanBadges() {
      const best = plans
        .slice()
        .sort((left, right) => {
          const utilityDiff = (right.move.utilityEstimate || 0) - (left.move.utilityEstimate || 0);
          if (utilityDiff !== 0) {
            return utilityDiff;
          }
          return right.index - left.index;
        })[0];
      root.querySelectorAll(".harmonies-advisor-plan").forEach((section) => {
        const isBest = best && section.dataset.planKey === best.key;
        section.classList.toggle("is-best-so-far", Boolean(isBest));
        const badge = section.querySelector(".harmonies-advisor-best-badge");
        if (badge) {
          badge.textContent = isBest ? "Best so far" : "";
        }
      });
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
    const badge = document.createElement("span");
    badge.className = "harmonies-advisor-best-badge";
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
    header.append(title, badge, select);
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
      if (action.kind === "settleCard") {
        drawSettlementLink(action, cell);
      }
    });
  }

  function findBoardCell(playerId, col, row) {
    const cellId = `cell_${playerId}_${col}_${row}`;
    return document.getElementById(cellId);
  }

  function drawSettlementLink(action, cell) {
    const card = findCard(action);
    if (!card) {
      return;
    }
    addRectOverlay(card, "harmonies-advisor-card-ring");
    addArrowOverlay(card, cell, String(action.typeArg ?? action.type_arg ?? ""));
  }

  function findCard(action) {
    const typeArg = action.typeArg ?? action.type_arg;
    const cardId = action.cardId ?? action.card_id;
    const selectors = [
      typeArg ? `#card_${CSS.escape(String(typeArg))}` : "",
      typeArg ? `#card-${CSS.escape(String(typeArg))}` : "",
      cardId ? `[data-card-id='${CSS.escape(String(cardId))}']` : "",
      cardId ? `[data-cardid='${CSS.escape(String(cardId))}']` : "",
      typeArg ? `[data-type-arg='${CSS.escape(String(typeArg))}']` : "",
    ].filter(Boolean);
    for (const selector of selectors) {
      const target = document.querySelector(selector);
      if (target && isVisibleRect(target.getBoundingClientRect())) {
        return target;
      }
    }
    return null;
  }

  function addRectOverlay(target, className) {
    const rect = target.getBoundingClientRect();
    if (!isVisibleRect(rect)) {
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
    if (!isVisibleRect(rect)) {
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

  function addArrowOverlay(fromElement, toElement, label) {
    const from = fromElement.getBoundingClientRect();
    const to = toElement.getBoundingClientRect();
    if (!isVisibleRect(from) || !isVisibleRect(to)) {
      return;
    }
    const start = {
      x: from.left + window.scrollX + from.width / 2,
      y: from.top + window.scrollY + from.height / 2,
    };
    const end = {
      x: to.left + window.scrollX + to.width / 2,
      y: to.top + window.scrollY + to.height / 2,
    };
    const padding = 28;
    const left = Math.min(start.x, end.x) - padding;
    const top = Math.min(start.y, end.y) - padding;
    const width = Math.abs(start.x - end.x) + padding * 2;
    const height = Math.abs(start.y - end.y) + padding * 2;
    const sx = start.x - left;
    const sy = start.y - top;
    const ex = end.x - left;
    const ey = end.y - top;
    const cx = (sx + ex) / 2;
    const cy = Math.min(sy, ey) - Math.max(24, Math.abs(sx - ex) * 0.08);
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    const markerId = `harmonies-advisor-arrowhead-${Math.random().toString(36).slice(2)}`;
    svg.classList.add("harmonies-advisor-settle-arrow");
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    svg.style.left = `${left}px`;
    svg.style.top = `${top}px`;
    svg.style.width = `${width}px`;
    svg.style.height = `${height}px`;
    svg.innerHTML = `
      <defs>
        <marker id="${markerId}" markerWidth="8" markerHeight="8" refX="6" refY="3.5" orient="auto">
          <path d="M0,0 L7,3.5 L0,7 Z" class="harmonies-advisor-arrow-head"></path>
        </marker>
      </defs>
      <path class="harmonies-advisor-arrow-path" d="M ${sx} ${sy} Q ${cx} ${cy} ${ex} ${ey}" marker-end="url(#${markerId})"></path>
    `;
    if (label) {
      const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
      text.classList.add("harmonies-advisor-arrow-label");
      text.setAttribute("x", String(cx));
      text.setAttribute("y", String(cy - 4));
      text.textContent = `card ${label} cube`;
      svg.appendChild(text);
    }
    visualLayer().appendChild(svg);
  }

  function isVisibleRect(rect) {
    return rect.width > 0 && rect.height > 0;
  }

  function visualLayer() {
    return document.getElementById(VISUAL_LAYER_ID);
  }

  function clearVisualLayer() {
    visualLayer()?.replaceChildren();
  }

  window.HarmoniesAdvisorOverlay = { createOverlay };
})();
