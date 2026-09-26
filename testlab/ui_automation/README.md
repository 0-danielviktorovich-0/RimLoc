# ui_automation — background UI journeys for the Tauri GUI

Drives the built `rimloc-gui` (or any similar Tauri/WKWebView app) with the
window parked **fully off-screen**: an automation run never shows a pixel on
the owner's display, never activates the app, and never touches the real
mouse/keyboard. Designed for unattended runs (night shifts, CI on a dev
machine) while a human is working at the console.

```bash
# from the repo root; needs the python env that has pyobjc + pyyaml
python3 -m testlab.ui_automation --config testlab/ui_automation/examples/rimloc-acceptance.yaml
```

On success you get per-step screenshots, a JSON-ish disturbance report, and
exit code 0. Every failing step stops the journey and the report names it.

## Prerequisites (macOS)

- The terminal/agent host running the driver needs **Accessibility** and
  **Screen Recording** permissions (System Settings → Privacy & Security).
  The driver checks AX trust non-prompting and never triggers a permission
  dialog on the owner's screen.
- The app binary must contain the **dev-only off-screen hooks**
  (`gui/tauri-app/src-tauri/src/main.rs`): they read
  `RIMLOC_WINDOW_ORIGIN` / `RIMLOC_WINDOW_MOVE` / `RIMLOC_FAKE` and are
  compiled out of release builds (`cfg!(debug_assertions)`).
- A console session must exist (a locked screen suspends WKWebView
  rendering for good — wait for unlock instead of fighting it).

## How it works

1. The app is launched with env hooks that park the window at e.g.
   `(-3000,-3000)` — off every display:
   - `constrainFrameRect:toScreen:` is replaced with an identity trampoline
     (AppKit otherwise clamps ANY frame change of an ordered window back
     on-screen);
   - `occlusionState` is faked visible, otherwise WebKit freezes the page on
     the first interaction;
   - `acceptsFirstMouse: YES` on the webview class: an app that can never
     become active would have every synthetic click swallowed as an
     "activation click";
   - a 3 s park thread re-asserts the origin (WebKit navigation can nudge
     the frame) and refreshes the app's lazily-hydrated accessibility
     server.
2. Clicks go through **accessibility**: the driver sets
   `AXManualAccessibility=YES` on the web area (re-setting it after every
   page re-render — WebKit resets it), finds the button by accessible name
   and performs `AXPress`. That dispatches a real DOM click with zero user
   -visible effects.
3. Frames come from `CGWindowListCreateImage` (the `screencapture -l` CLI
   refuses windows outside display geometry). Detectors are theme-agnostic
   (dominant-background contrast, not hardcoded colors).
4. Success is asserted by **state**: text-row counters, off-screen
   re-checks, the app's managed-storage directory — not by "the exit code
   was 0".

### Click channels, honestly ranked

| channel | off-screen result |
|---|---|
| `CGEventPost` (HID) | disqualified: nothing to hit off-screen, and it teleports the user's cursor |
| `CGEventPostToPid` mouse | events arrive, CSS `:hover` works, but mousedown/up are swallowed by NSWindow's activation-click logic for an app that is never active |
| **`AXPress` by name** | **works** — real DOM clicks; the primary channel of this tool |

## Journey config

```yaml
app:
  binary: target/debug/rimloc-gui
  origin: [-3000, -3000]        # parking position (points)
  move_mode: swizzle            # constrainFrameRect bypass
  fake: occlusion,firstmouse    # window-visibility lies (see main.rs)
  settle_s: 9                   # let web fonts/layout settle
pid_file: /tmp/rimloc-ui-automation.pid
shots_dir: /tmp/rimloc-ui-journey

steps:
  - press: {name: Русский, timeout: 90, exact: true}  # pin locale first
  - shot: 01-home
  - press: Открыть              # AXPress by accessible name
  - wait_text_rows: 12          # workspace table rendered
  - shot: 02-workspace
  - assert_offscreen: true
  - press: Собрать перевод
  - shot: 03-build
```

Step types: `shot`, `press` (name/timeout/exact), `wait_text_rows`,
`sleep`, `assert_offscreen`. Button names are the app's accessible names
(AVAILABLE IN THE UI LANGUAGE — pin the locale first, as above, because a
locale switch also re-renders the page and resets `AXManualAccessibility`).

`--shots-dir` overrides the config value from the CLI.

## Lifecycle & orphan instances

Every spawned instance is recorded in the `pid_file` and cleaned up by
`atexit` **and** `SIGINT`/`SIGTERM` handlers — even if the driver dies
mid-step, only the pid this run spawned is killed.

- **Reuse**: a run that finds a live pid in the pid file (command line must
  match the configured binary) attaches to it instead of spawning a
  duplicate, and leaves it running at the end.
- **`fresh_instance: true`** ignores the pid file and always spawns clean.
- Cleanup is **never** a name-based `pkill`: that would take down the
  owner's own instances.

If orphans from older (pre-lifecycle) runs exist anyway, clean them by
**explicit pid after inspection** — never by name:

```bash
# 1. list candidates and LOOK at their command lines / parent
pgrep -fl rimloc-gui
# 2. kill only the ones you can attribute (by pid, not by name)
kill <pid>          # SIGTERM is enough; the app exits cleanly
```

## Known limitations

- The AX bridge hydration is lazy; the driver polls (default 60 s per
  press) and the app-side self-query hook makes it near-deterministic, but
  a pathologically slow machine may need a longer `timeout`.
- Locale switches and in-app navigation reset `AXManualAccessibility`; the
  driver re-sets it on every attempt, but keep the locale pinned for the
  whole journey (name lookups are language-dependent).
- A locked console suspends WKWebView rendering; journeys started on a
  locked console produce blank frames. Schedule runs for unlocked windows.
- Entering free-form text is not covered (no keyboard channel off-screen);
  journeys are click/navigation-oriented for now.
