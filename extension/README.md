# Harmonies BGA Advisor Extension

Firefox temporary extension scaffold.

## Load

1. Open `about:debugging#/runtime/this-firefox`.
2. Click `Load Temporary Add-on`.
3. Select `extension/manifest.json`.

## Behavior

- Injects `pageBridge.js` into BGA page context.
- Reads `window.gameui.gamedatas`.
- Posts snapshots to content script.
- Shows read-only overlay with mock recommendation.
- Never clicks, never calls `ajaxcall`, never sends BGA action requests.
