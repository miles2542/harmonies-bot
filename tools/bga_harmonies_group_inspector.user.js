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
      const tokenNodes = [1, 2, 3]
        .map((tokenIndex) => document.getElementById(`hole-${groupId}-token-${tokenIndex}`))
        .filter(Boolean);
      groups[String(groupId)] = (tokenNodes.length
        ? tokenNodes
        : Array.from(hole.querySelectorAll(".hole-token, .colored-token")))
        .map((node) => {
          const typeArg = domTypeArg(node);
          return { typeArg, color: colorName(typeArg), id: node.id || "" };
        })
        .filter((token) => token.color);
    }
    return groups;
  }

  function domTypeArg(node) {
    const match = /(?:^|\s)color-(\d)(?:\s|$)/.exec(String(node.className || ""));
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
    hole.style.position = hole.style.position || "relative";
    const label = document.createElement("div");
    label.className = LABEL_CLASS;
    label.textContent = `${groupId}: ${colors.join(" / ") || "?"}`;
    label.style.cssText = [
      "position:absolute",
      "left:0",
      "top:-28px",
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
    hole.appendChild(label);
  }

  function clearLabels() {
    document.querySelectorAll(`.${LABEL_CLASS}`).forEach((node) => node.remove());
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
