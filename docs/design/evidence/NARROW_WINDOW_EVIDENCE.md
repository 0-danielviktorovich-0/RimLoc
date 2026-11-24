# Narrow-window evidence (T6, #161): 980×640 layout

Date: 2026-11-25 · Lane: `testlab/ui_automation/wdio/narrow-window-run.sh`
Build: `test/native-ax-window` automation bundle (React flavor,
`automation-bridge`), sha256 `61ca5673…`, isolated data dir.

The minimum window (980×640 — the `minWidth`/`minHeight` of
`tauri.conf.json`) is placed through the `RIMLOC_WINDOW_FRAME` env hook in
`main.rs` (compile-time gated to `automation-bridge`; browser viewport APIs
cannot resize a native Tauri window). Every check is a measured DOM fact
(`getBoundingClientRect` through `browser.execute`, the react-geometry
UI-04 precedent) plus native CGWindowList captures of the same build.

## Result: 100/100 probes, §11A-F 5/5

- 17 surfaces × 5 probes (document overflow, root scroll, offscreen
  interactive controls, clipped controls, sibling-block overlaps) in light
  theme + workspace/settings in dark: **100/100 PASS**
  (`target/logs/t6-evidence/narrow-window-report.json`).
- Window-state regression §11A-F: **5/5 PASS** (user frame restored,
  automation frame ephemeral 980×640, fully inside the display's visible
  frame, not fullscreen-sized, owner stays frontmost — zero activations,
  zero keystrokes).
- Native window verified from CGWindowList: frame `348,71,980x640`,
  content area 980×608 (32 px title bar), fully on-display.

## Defects found at 980×640 → fixed in this branch

| Defect | Symptom | Root cause | Fix | Evidence |
|---|---|---|---|---|
| Sidebar bottom unreachable | Settings/About links fell past the window edge; no scroll container anywhere on the chain | `.app-sidebar` is a fixed full-height flex column; natural content ~822px overflows the 608px content area with `overflow:visible` and is painted outside the window | `.app-sidebar { overflow-y: auto }` (thin scrollbar, the `.entry-list` pattern) | before: 15 surfaces flagged `#shell.about-link`; after: 0 |
| Workspace footer («На главную») unreachable | The only back-navigation control of the workspace sat ~611px below the visible box | `.ws-page` is an unstyled wrapper; the editor grid floors at `minmax(530px, …)`, so the page (~1105px) outgrew the 489px visible box and `route-content { overflow: hidden }` clipped it | `.ws-page { flex:1; min-height:0; overflow-y:auto }` (the `.page-content` pattern) | before: workspace + dark-workspace flagged; after: 0 |

False-positive tuning of the lane itself (documented, no app change):
below-the-fold controls inside scrollable containers are exempt — the
reachability probe walks ancestors for `overflow: auto/scroll` with real
overflow extent; parked skip-links (left ≈ −9999px) are exempt as an
intentional a11y pattern; the viewport probe expects the 32px title bar.

## Honest notes

- Horizontal overflow, sibling overlaps and panel containment were already
  clean at 980×640 (zero findings) — the react-geometry invariants hold at
  the minimum window.
- The two defects above were NOT caught by earlier lanes because their
  probes checked horizontal edges only; this lane adds vertical
  reachability as a first-class probe.
- Frames come from `screencapture -x -o -l<windowid>` (no shadow) and
  `CGWindowListCreateImage` (kCGWindowImageBoundsIgnoreFraming); pixels =
  2× CSS px (dpr 2). `screencapture` is the primary path — the in-process
  Quartz capture needs the same Screen Recording grant and stays
  best-effort.
- §11A/B still move/verify window frames through System Events
  (`set position` — an AX-value manipulation, no activation, no
  keystrokes), per the window-correction directive; the resize itself is
  the env hook, never AppleScript.

## Screens (committed)

- `docs/screenshots/narrow-window/workspace.png` — workspace at 980×640
  after both fixes (nav complete, no overlaps, page scrolls to the footer)
- `docs/screenshots/narrow-window/settings.png` — settings, narrow column
- `docs/screenshots/narrow-window/dark-workspace.png` — dark theme parity

Reproduce (one command, background-only):

```bash
bash testlab/build/build-automation-react.sh
bash testlab/ui_automation/wdio/narrow-window-run.sh \
  target/release/bundle/macos/RimLoc\ GUI.app
```
