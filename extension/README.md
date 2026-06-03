# Harmonies BGA Advisor Extension

Firefox temporary extension scaffold.

## Load

1. Open `about:debugging#/runtime/this-firefox`.
2. Click `Load Temporary Add-on`.
3. Select `extension/manifest.json`.

## Native Engine

Run local advisor service before loading/using the extension:

```powershell
cargo run -p harmonies-service
```

Default endpoint: `http://127.0.0.1:17848/advise`.

## Behavior

- Injects `pageBridge.js` into BGA page context.
- Reads `window.gameui.gamedatas`.
- Posts snapshots to content script.
- Sends normalized snapshot to local Rust service when available.
- Falls back to mock recommendation when local service is unavailable.
- Never clicks, never calls `ajaxcall`, never sends BGA action requests.
