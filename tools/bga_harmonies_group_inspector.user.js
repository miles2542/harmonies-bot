// ==UserScript==
// @name         Harmonies BGA Group Inspector
// @namespace    harmonies-bga-advisor-local
// @version      0.1.0
// @description  Read-only central-token group inspector for Harmonies parser QA.
// @match        https://boardgamearena.com/*
// @match        https://*.boardgamearena.com/*
// @grant        unsafeWindow
// ==/UserScript==

(function harmoniesGroupInspector() {
  const ROOT_ID = "harmonies-group-inspector";
  const LAYER_ID = "harmonies-group-inspector-layer";
  const LABEL_CLASS = "harmonies-group-inspector-label";
  const COLORS = {
    1: "Water",
    2: "Mountain",
    3: "Trunk",
    4: "Foliage",
    5: "Field",
    6: "Building",
    7: "Building",
  };

  function readPageWindow() {
    if (typeof unsafeWindow !== "undefined" && unsafeWindow) {
      return unsafeWindow;
    }
    return window;
  }

  function readGamedatasGroups() {
    const groups = readPageWindow().gameui?.gamedatas?.tokensOnCentralBoard || {};
    return Object.fromEntries(
      Object.entries(groups)
        .sort(([left], [right]) => Number.parseInt(left, 10) - Number.parseInt(right, 10))
        .map(([groupId, tokens]) => [
          groupId,
          (Array.isArray(tokens) ? tokens : []).map((token) => ({
            id: token.id,
            typeArg: Number(token.type_arg),
            color: colorName(token.type_arg),
            locationArg: token.location_arg,
          })),
        ]),
    );
  }

  function readDomGroups() {
    const groups = {};
    for (let groupId = 1; groupId <= 5; groupId += 1) {
      const hole = document.getElementById(`hole-${groupId}`);
      if (!hole) {
        continue;
      }
      const tokenNodes = centralTokenNodes(hole, groupId);
      groups[String(groupId)] = tokenNodes
        .map((node) => {
          const typeArg = domTypeArg(node);
          return { typeArg, color: colorName(typeArg), id: node.id || "" };
        })
        .filter((token) => token.color);
    }
    return groups;
  }

  function centralTokenNodes(hole, groupId) {
    const orderedIds = [1, 2, 3]
      .map((tokenIndex) => document.getElementById(`hole-${groupId}-token-${tokenIndex}`))
      .filter((node) => node && hole.contains(node) && isVisibleElement(node));
    const candidates = orderedIds.length
      ? orderedIds
      : Array.from(
          hole.querySelectorAll(
            ".hole-token, .colored-token, [class*='color-'], [id^='hole-'][id*='-token-']",
          ),
        ).filter(isVisibleElement);
    const unique = [];
    const seen = new Set();
    candidates.forEach((node) => {
      if (!seen.has(node)) {
        seen.add(node);
        unique.push(node);
      }
    });
    unique.sort((left, right) => tokenNodeSortKey(left) - tokenNodeSortKey(right));
    return unique.filter((node) => Number.isFinite(domTypeArg(node))).slice(0, 3);
  }

  function tokenNodeSortKey(node) {
    const match = /-token-(\d+)/.exec(String(node.id || ""));
    return match ? Number.parseInt(match[1], 10) : 99;
  }

  function isVisibleElement(node) {
    const style = window.getComputedStyle(node);
    const rect = node.getBoundingClientRect();
    return (
      style.display !== "none" &&
      style.visibility !== "hidden" &&
      Number(style.opacity || 1) > 0.01 &&
      rect.width > 2 &&
      rect.height > 2
    );
  }

  function domTypeArg(node) {
    const className = [node, ...node.querySelectorAll("*")]
      .map((item) => String(item.className || ""))
      .join(" ");
    const match = /(?:^|\s)color-(\d)(?:\s|$)/.exec(className);
    return match ? Number(match[1]) : Number.NaN;
  }

  function colorName(typeArg) {
    return COLORS[Number(typeArg)] || "";
  }

  function inspect() {
    clearLabels();
    const gamedatas = readGamedatasGroups();
    const dom = readDomGroups();
    const rows = [];
    for (let groupId = 1; groupId <= 5; groupId += 1) {
      const key = String(groupId);
      const domColors = (dom[key] || []).map((token) => token.color);
      const gamedatasColors = (gamedatas[key] || []).map((token) => token.color);
      const row = {
        group: key,
        dom: domColors.join(", "),
        gamedatas: gamedatasColors.join(", "),
        match: sameMultiset(domColors, gamedatasColors),
        domRaw: dom[key] || [],
        gamedatasRaw: gamedatas[key] || [],
      };
      rows.push(row);
      labelHole(key, domColors, row.match);
    }
    console.table(rows.map(({ group, dom, gamedatas, match }) => ({ group, dom, gamedatas, match })));
    console.log("Harmonies group inspector detail", rows);
    return rows;
  }

  function sameMultiset(left, right) {
    if (left.length !== right.length) {
      return false;
    }
    return left.slice().sort().join("|") === right.slice().sort().join("|");
  }

  function labelHole(groupId, colors, matched) {
    const hole = document.getElementById(`hole-${groupId}`);
    if (!hole) {
      return;
    }
    const rect = hole.getBoundingClientRect();
    const label = document.createElement("div");
    label.className = LABEL_CLASS;
    label.textContent = `${groupId}: ${colors.join(" / ") || "?"}`;
    label.style.cssText = [
      "position:fixed",
      `left:${Math.round(rect.left)}px`,
      `top:${Math.max(0, Math.round(rect.top - 28))}px`,
      "z-index:99999",
      "max-width:180px",
      "padding:2px 5px",
      `background:${matched ? "rgba(6,78,59,.82)" : "rgba(127,29,29,.88)"}`,
      "color:#fff",
      "font:700 11px/14px system-ui,sans-serif",
      "border-radius:4px",
      "pointer-events:none",
      "white-space:nowrap",
    ].join(";");
    visualLayer().appendChild(label);
  }

  function clearLabels() {
    visualLayer().replaceChildren();
  }

  function visualLayer() {
    let layer = document.getElementById(LAYER_ID);
    if (!layer) {
      layer = document.createElement("div");
      layer.id = LAYER_ID;
      layer.style.cssText = [
        "position:fixed",
        "inset:0",
        "z-index:99998",
        "pointer-events:none",
      ].join(";");
      document.documentElement.appendChild(layer);
    }
    return layer;
  }

  function installPanel() {
    if (document.getElementById(ROOT_ID)) {
      return;
    }
    const root = document.createElement("section");
    root.id = ROOT_ID;
    root.innerHTML = `
      <strong>Group Inspector</strong>
      <button type="button" data-action="inspect">Inspect</button>
      <button type="button" data-action="clear">Clear</button>
    `;
    root.style.cssText = [
      "position:fixed",
      "right:12px",
      "bottom:54px",
      "z-index:99999",
      "display:flex",
      "gap:6px",
      "align-items:center",
      "padding:8px",
      "background:#111",
      "color:#fff",
      "font:12px sans-serif",
      "border:1px solid #555",
    ].join(";");
    document.documentElement.appendChild(root);
    root.querySelector("[data-action='inspect']").addEventListener("click", inspect);
    root.querySelector("[data-action='clear']").addEventListener("click", clearLabels);
  }

  readPageWindow().HarmoniesGroupInspector = { inspect, clearLabels };
  window.HarmoniesGroupInspector = { inspect, clearLabels };
  installPanel();
})();
