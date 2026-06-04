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

Service smoke:

```powershell
python -m tools.service_smoke
```

This starts a temporary local service, checks `/health`, `/advise`, and `/ws`, then stops it.

## Behavior

- Injects `pageBridge.js` into BGA page context.
- Reads `window.gameui.gamedatas`.
- Posts snapshots to content script.
- Caches latest visible table state.
- Sends normalized snapshot to local Rust service only when `Analyze` is pressed.
- Falls back to mock recommendation when local service is unavailable.
- Never clicks, never calls `ajaxcall`, never sends BGA action requests.

## Safety Check

```powershell
python -m tools.extension_safety_check
```

The check fails on synthetic clicks/events, BGA action helpers, non-local advisor endpoints, and
unexpected manifest permissions.

## Live QA Checklist

Use a real active 2-player Side A Nature Spirit table.

1. Run `cargo run -p harmonies-service`.
2. Load `extension/manifest.json` from `about:debugging#/runtime/this-firefox`.
3. Open table on your turn before any action.
4. Press `Analyze`.
5. Confirm panel status changes from analyzing to ready.
6. Confirm first-turn plans show `Choose Spirit` before `Take group`.
7. Confirm chosen central group `#hole-N` has yellow outline.
8. Confirm recommended board cells have teal outline and corner step badges matching text steps.
9. Press `Stop` during search; button should change to retry behavior, with no BGA action performed.
10. Watch network/devtools: no BGA action requests, only localhost advisor traffic.

## Group Inspector

For central-token parser QA, install `tools\bga_harmonies_group_inspector.user.js` in
ScriptCat/Tampermonkey. It works in active and spectated games.

- `Inspect`: labels `#hole-1..5` and logs DOM vs `gamedatas.tokensOnCentralBoard` to console.
- Green label: DOM and `gamedatas` match by token multiset.
- Red label: mismatch; capture/share screenshot plus console table.
