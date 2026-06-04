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

Optional tuned weights:

```powershell
$env:HARMONIES_WEIGHTS='docs\weights.baseline.json'
cargo run -p harmonies-service
```

## Behavior

- Injects `pageBridge.js` into BGA page context.
- Reads `window.gameui.gamedatas`.
- Posts snapshots to content script.
- Sends normalized snapshot to local Rust service when available.
- Falls back to mock recommendation when local service is unavailable.
- Never clicks, never calls `ajaxcall`, never sends BGA action requests.

## Safety Check

```powershell
python -m tools.extension_safety_check
```

The check fails on synthetic clicks/events, BGA action helpers, non-local advisor endpoints, and
unexpected manifest permissions.
