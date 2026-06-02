(function harmoniesAdvisorOverlayModule() {
  const ROOT_ID = "harmonies-advisor-root";
  const HIGHLIGHT_CLASS = "harmonies-advisor-highlight";

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
        <button type="button" data-action="toggle" title="Collapse panel">_</button>
      </header>
      <div class="harmonies-advisor-body">
        <div class="harmonies-advisor-status">Starting</div>
        <div class="harmonies-advisor-best"></div>
        <ol class="harmonies-advisor-steps"></ol>
        <div class="harmonies-advisor-alternatives"></div>
      </div>
    `;
    document.documentElement.appendChild(root);

    root.querySelector("[data-action='toggle']").addEventListener("click", () => {
      root.classList.toggle("is-collapsed");
    });

    return {
      setStatus(message) {
        root.querySelector(".harmonies-advisor-status").textContent = message;
      },
      renderRecommendation(response) {
        clearHighlights();
        this.setStatus(response.status);

        const best = root.querySelector(".harmonies-advisor-best");
        const steps = root.querySelector(".harmonies-advisor-steps");
        const alternatives = root.querySelector(".harmonies-advisor-alternatives");
        best.textContent = response.bestMove?.title || "No recommendation";
        steps.replaceChildren(...(response.bestMove?.steps || []).map(renderStep));
        alternatives.replaceChildren(...response.alternatives.map(renderAlternative));

        if (response.bestMove?.centralGroupId) {
          highlightCentralGroup(response.bestMove.centralGroupId);
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
      `#token_group_${CSS.escape(groupId)}`,
      `#tokens_${CSS.escape(groupId)}`,
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

  function clearHighlights() {
    document
      .querySelectorAll(`.${HIGHLIGHT_CLASS}`)
      .forEach((node) => node.classList.remove(HIGHLIGHT_CLASS));
  }

  window.HarmoniesAdvisorOverlay = { createOverlay };
})();
