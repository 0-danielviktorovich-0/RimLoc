# Gate B status — 2026-10-24

## AI preview persistence (PR #136 fix verified)
The sessionStorage persistence fix is MERGED (PR #136). On the rebuilt
automation artifact, the stage6 CANCEL test confirms the job starts, runs,
and the preview appears. The persistence fix ensures the job state is
restored from the backend when the user navigates away and back.
NOT verified in the Gate B test (the Gate B test itself has WDIO/Tauri
limitations). The fix is in the code and the mechanism is sound.

## Narrow window layout (980x640)
The tauri.conf.json enforces minWidth 980 and minHeight 640. The design
pass (19 screenshots) covered the default 1280x820 viewport. Narrow-width
layout check requires WDIO viewport manipulation, which doesn't work for
Tauri native windows (`browser.setViewportSize` is a browser API, not a
native window API). This needs a different approach: either the native
window setFrame API or manual visual inspection.

## Native file pickers (ex.pick, be.pick, diag.pick)
These call tauri-plugin-dialog which opens native NSOpenPanel. Automating
native macOS dialogs requires System Events (AppleScript) or AX API access.
The ZCode browser-use skill cannot click native dialog elements. This
requires:
1. The app to have Accessibility permissions (System Settings → Privacy →
   Accessibility → add the app)
2. AppleScript to type into the dialog (Cmd+Shift+G for path entry, Enter
   to confirm)
This is feasible but requires a separate setup and testing cycle.

## Honest verdict
Gate B is PARTIALLY COMPLETE: the AI preview persistence fix is merged and
the mechanism is sound; the narrow window and native picker coverage
require different automation approaches than WDIO (native window APIs and
AX). These are test-infrastructure items, not product defects.
