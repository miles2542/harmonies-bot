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
        <div class="harmonies-advisor-header-actions">
          <label class="harmonies-advisor-auto-analyze-label" title="Automatically analyze when it is your turn">
            <input type="checkbox" id="harmonies-advisor-auto-analyze-cb" />
            Auto-Analyze
          </label>
          <button type="button" data-action="toggle" title="Collapse panel">_</button>
        </div>
      </header>
      <div class="harmonies-advisor-body">
        <div class="harmonies-advisor-score-panel">
          <div class="score-row">
            <strong>Your Score:</strong> <span class="user-score">--</span> <span class="expected-arrow">→</span> <span class="expected-score">--</span>
          </div>
          <div class="score-row">
            <strong>Opponent:</strong> <span class="opp-score">--</span>
          </div>
        </div>
        <div class="harmonies-advisor-status">Starting</div>
        <div class="harmonies-advisor-plans"></div>
      </div>
      <footer class="harmonies-advisor-footer">
        <button type="button" class="harmonies-advisor-footer-btn" data-action="analyze">Analyze</button>
        <button type="button" class="harmonies-advisor-footer-btn" data-action="stop">Stop</button>
        <button type="button" class="harmonies-advisor-footer-btn" data-action="freeze">Freeze Plan</button>
      </footer>
      <div class="harmonies-advisor-resizer"></div>
    `;
    document.documentElement.appendChild(root);

    const visualLayerEl = document.createElement("div");
    visualLayerEl.id = VISUAL_LAYER_ID;
    document.documentElement.appendChild(visualLayerEl);

    // Load saved dimensions/position
    const savedX = localStorage.getItem("harmonies-advisor-panel-x");
    const savedY = localStorage.getItem("harmonies-advisor-panel-y");
    if (savedX && savedY) {
      root.style.right = "auto";
      root.style.left = `${savedX}px`;
      root.style.top = `${savedY}px`;
    }
    const savedWidth = localStorage.getItem("harmonies-advisor-panel-width");
    const savedHeight = localStorage.getItem("harmonies-advisor-panel-height");
    if (savedWidth) root.style.width = `${savedWidth}px`;
    if (savedHeight) root.style.height = `${savedHeight}px`;

    // Draggable header
    const header = root.querySelector(".harmonies-advisor-header");
    let isDragging = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let panelStartX = 0;
    let panelStartY = 0;

    header.addEventListener("mousedown", (e) => {
      if (e.target.closest("button") || e.target.closest("input") || e.target.closest("label")) return;
      isDragging = true;
      dragStartX = e.clientX;
      dragStartY = e.clientY;
      const rect = root.getBoundingClientRect();
      panelStartX = rect.left;
      panelStartY = rect.top;
      root.style.right = "auto";
      root.style.left = `${panelStartX}px`;
      root.style.top = `${panelStartY}px`;
      e.preventDefault();
    });

    // Resizable handle
    const resizer = root.querySelector(".harmonies-advisor-resizer");
    let isResizing = false;
    let resizeStartX = 0;
    let resizeStartY = 0;
    let panelStartWidth = 0;
    let panelStartHeight = 0;

    resizer.addEventListener("mousedown", (e) => {
      isResizing = true;
      resizeStartX = e.clientX;
      resizeStartY = e.clientY;
      const rect = root.getBoundingClientRect();
      panelStartWidth = rect.width;
      panelStartHeight = rect.height;
      e.preventDefault();
      e.stopPropagation();
    });

    document.addEventListener("mousemove", (e) => {
      if (isDragging) {
        const dx = e.clientX - dragStartX;
        const dy = e.clientY - dragStartY;
        const left = panelStartX + dx;
        const top = panelStartY + dy;
        root.style.left = `${left}px`;
        root.style.top = `${top}px`;
        localStorage.setItem("harmonies-advisor-panel-x", String(left));
        localStorage.setItem("harmonies-advisor-panel-y", String(top));
      }
      if (isResizing) {
        const dx = e.clientX - resizeStartX;
        const dy = e.clientY - resizeStartY;
        const width = Math.max(280, panelStartWidth + dx);
        const height = Math.max(200, panelStartHeight + dy);
        root.style.width = `${width}px`;
        root.style.height = `${height}px`;
        localStorage.setItem("harmonies-advisor-panel-width", String(width));
        localStorage.setItem("harmonies-advisor-panel-height", String(height));
      }
    });

    document.addEventListener("mouseup", () => {
      isDragging = false;
      isResizing = false;
    });

    // Config: Auto-Analyze
    const autoAnalyzeCb = root.querySelector("#harmonies-advisor-auto-analyze-cb");
    const autoAnalyzeKey = "harmonies-advisor-auto-analyze";
    autoAnalyzeCb.checked = localStorage.getItem(autoAnalyzeKey) === "true";
    autoAnalyzeCb.addEventListener("change", () => {
      localStorage.setItem(autoAnalyzeKey, String(autoAnalyzeCb.checked));
    });

    // Collapse toggle
    root.querySelector("[data-action='toggle']").addEventListener("click", () => {
      root.classList.toggle("is-collapsed");
    });

    // Freeze Plan state
    let isFrozen = false;
    const freezeBtn = root.querySelector("[data-action='freeze']");
    freezeBtn.addEventListener("click", () => {
      isFrozen = !isFrozen;
      root.classList.toggle("is-plan-frozen", isFrozen);
      freezeBtn.textContent = isFrozen ? "Unlock Plan" : "Freeze Plan";
      freezeBtn.classList.toggle("is-active-frozen", isFrozen);
    });

    let activePlan = null;
    let selectedStepIndex = null; // null means "show all"
    let userScoreVal = 0;
    let opponentScoreVal = 0;

    function selectStep(index) {
      selectedStepIndex = index;
      const stepItems = root.querySelectorAll(".harmonies-advisor-step-item");
      stepItems.forEach((li, idx) => {
        li.classList.toggle("is-active-step", idx === index);
      });
      const showAllBtn = root.querySelector(".show-all-steps-btn");
      if (showAllBtn) {
        showAllBtn.classList.toggle("active", index === null);
      }
      clearVisualLayer();
      if (activePlan) {
        drawPlanOverlays(activePlan.move);
      }
    }

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
      isAutoAnalyzeEnabled() {
        return autoAnalyzeCb.checked;
      },
      setScores(userScore, opponentScore) {
        userScoreVal = userScore;
        opponentScoreVal = opponentScore;
        root.querySelector(".user-score").textContent = `${userScore} VP`;
        root.querySelector(".opp-score").textContent = `${opponentScore} VP`;
      },
      beginAnalysis() {
        if (isFrozen) return;
        activePlan = null;
        selectedStepIndex = null;
        clearVisualLayer();
        root.querySelector(".harmonies-advisor-plans").replaceChildren();
        root.querySelector(".expected-score").textContent = "--";
      },
      renderRecommendationTier(response) {
        if (isFrozen) return;
        this.setStatus(response.status);
        const bestMove = response.bestMove;
        if (!bestMove) {
          root.querySelector(".harmonies-advisor-plans").textContent = "No recommendation";
          root.querySelector(".expected-score").textContent = "--";
          return;
        }

        // Expected score update
        root.querySelector(".expected-score").textContent = `${bestMove.immediateTotal} VP`;

        // We only render one unified best plan
        const plan = {
          key: "best-plan",
          move: bestMove,
          alternatives: response.alternatives || [],
          label: response.isFinal ? "Final Plan" : "Thinking Plan",
        };
        activePlan = plan;

        const plansContainer = root.querySelector(".harmonies-advisor-plans");
        plansContainer.replaceChildren();

        const planSection = document.createElement("section");
        planSection.className = "harmonies-advisor-plan is-selected is-open";

        const stepsHeader = document.createElement("div");
        stepsHeader.className = "harmonies-advisor-steps-header";
        stepsHeader.innerHTML = `<strong>Instructions:</strong>`;
        const showAllBtn = document.createElement("button");
        showAllBtn.type = "button";
        showAllBtn.className = `show-all-steps-btn ${selectedStepIndex === null ? "active" : ""}`;
        showAllBtn.textContent = "Show All Overlays";
        showAllBtn.addEventListener("click", () => selectStep(null));
        stepsHeader.appendChild(showAllBtn);
        planSection.appendChild(stepsHeader);

        const stepsList = document.createElement("ol");
        stepsList.className = "harmonies-advisor-steps-list";

        const numActionSteps = bestMove.actions.length;
        const actionStepsText = bestMove.steps.slice(0, numActionSteps);
        const debugStepsText = bestMove.steps.slice(numActionSteps);

        actionStepsText.forEach((stepText, idx) => {
          const li = document.createElement("li");
          li.className = `harmonies-advisor-step-item ${selectedStepIndex === idx ? "is-active-step" : ""}`;
          li.textContent = stepText;
          li.addEventListener("click", () => selectStep(idx));
          stepsList.appendChild(li);
        });
        planSection.appendChild(stepsList);

        if (plan.alternatives.length) {
          const altsDetails = document.createElement("details");
          altsDetails.className = "harmonies-advisor-plan-alternatives";
          altsDetails.innerHTML = `<summary>Alternative groups</summary>`;
          const altsList = document.createElement("div");
          plan.alternatives.forEach((alt) => {
            const altDiv = document.createElement("div");
            altDiv.className = "harmonies-advisor-alt";
            altDiv.textContent = alt.label;
            altsList.appendChild(altDiv);
          });
          altsDetails.appendChild(altsList);
          planSection.appendChild(altsDetails);
        }

        // Add debug details
        const debugDetails = document.createElement("details");
        debugDetails.className = "harmonies-advisor-debug-details";
        debugDetails.innerHTML = `<summary>Debug Info</summary>`;
        const debugContent = document.createElement("div");
        debugContent.className = "harmonies-advisor-debug-content";
        debugStepsText.forEach((text) => {
          const p = document.createElement("p");
          p.textContent = text;
          debugContent.appendChild(p);
        });
        debugDetails.appendChild(debugContent);
        planSection.appendChild(debugDetails);

        plansContainer.appendChild(planSection);

        // Draw overlays
        clearVisualLayer();
        drawPlanOverlays(bestMove);
      },
    };

    function drawPlanOverlays(bestMove) {
      if (selectedStepIndex === null) {
        // Draw all overlays
        if (bestMove.centralGroupId) {
          drawCentralGroupOverlay(bestMove.centralGroupId);
        }
        drawBoardActionOverlays(bestMove, null);
      } else {
        // Draw only selectedStepIndex overlay
        const action = bestMove.actions[selectedStepIndex];
        if (!action) return;
        if (action.kind === "takeGroup") {
          drawCentralGroupOverlay(String(action.groupIndex + 1));
        } else if (action.kind === "placeToken") {
          const cell = findBoardCell(bestMove.playerId, action.col, action.row);
          if (cell) {
            addRectOverlay(cell, "harmonies-advisor-cell-ring");
            addStepMarker(cell, String(selectedStepIndex + 1), action.kind);
          }
        } else if (action.kind === "draftCard" || action.kind === "chooseSpirit") {
          const card = findCard(action);
          if (card) {
            addRectOverlay(card, "harmonies-advisor-card-ring");
          }
        } else if (action.kind === "settleCard") {
          const cell = findBoardCell(bestMove.playerId, action.col, action.row);
          if (cell) {
            addRectOverlay(cell, "harmonies-advisor-cell-ring");
            addStepMarker(cell, String(selectedStepIndex + 1), action.kind);
            drawSettlementLink(action, cell);
          }
        }
      }
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

    function drawBoardActionOverlays(bestMove, activeIndex = null) {
      const playerId = bestMove.playerId;
      if (!playerId) {
        return;
      }
      (bestMove.actions || []).forEach((action, index) => {
        if (activeIndex !== null && activeIndex !== index) return;
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
      const typeArg = action.typeArg ?? action.type_arg ?? "";
      addArrowOverlay(card, cell, String(typeArg));
    }

    function findCard(action) {
      const cardId = action.cardId ?? action.card_id;
      const selectors = [
        cardId ? `#card_${CSS.escape(String(cardId))}` : "",
        cardId ? `#card-${CSS.escape(String(cardId))}` : "",
        cardId ? `[data-card-id='${CSS.escape(String(cardId))}']` : "",
        cardId ? `[data-cardid='${CSS.escape(String(cardId))}']` : "",
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
        text.textContent = `type ${label} cube`;
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
  }

  window.HarmoniesAdvisorOverlay = { createOverlay };
})();
