"""Background UI-automation driver for Tauri/WKWebView apps on macOS.

Everything here works with the window parked FULLY OFF-SCREEN (see the
app-side dev-only env hooks: RIMLOC_WINDOW_ORIGIN / RIMLOC_WINDOW_MOVE /
RIMLOC_FAKE), so an automation run never shows a single pixel on the
owner's display and never activates the app.

Channels, by what actually works off-screen (verified 2026-09-26):

  HID events (CGEventPost)      — do NOT use: no display-geometry hit target
                                  off-screen, and it teleports the user's
                                  cursor.
  CGEventPostToPid mouse events — events reach the app; CSS :hover works,
                                  but mousedown/up are swallowed by NSWindow's
                                  "activation click" for an app that never
                                  becomes active. Hover-only in practice;
                                  kept for probes.
  AXPress (accessibility)       — THE working click channel. The native AX
                                  tree exposes the window; setting
                                  AXManualAccessibility=YES on the AXWebArea
                                  exposes web content; AXPress on a web
                                  button dispatches a real DOM click.

Frames come from CGWindowListCreateImage (the `screencapture -l` CLI refuses
windows whose global rect lies outside display geometry).
"""

from __future__ import annotations

import hashlib
import os
import subprocess
import time
from dataclasses import dataclass, field

import ApplicationServices as AS
import Quartz
from AppKit import NSWorkspace
from PIL import Image

# Single-display geometry is measured at import when possible; these are
# conservative fallbacks (a window parked at negative thousands of points is
# off-screen for any realistic setup).
_FALLBACK_DISPLAY = (1352, 878)


def _display_size() -> tuple[int, int]:
    try:
        main = Quartz.CGDisplayBounds(Quartz.CGMainDisplayID())
        return int(main.size.width), int(main.size.height)
    except Exception:
        return _FALLBACK_DISPLAY


# --------------------------------------------------------------------- env

def offscreen_env(origin: tuple[int, int] = (-3000, -3000),
                  move_mode: str = 'swizzle',
                  fake: str = 'occlusion,firstmouse') -> dict[str, str]:
    """Env for launching the app parked off-screen (requires the dev-only
    hooks built into the app; release builds ignore them)."""
    env = dict(os.environ)
    env['RIMLOC_WINDOW_ORIGIN'] = f'{origin[0]},{origin[1]}'
    env['RIMLOC_WINDOW_MOVE'] = move_mode
    env['RIMLOC_FAKE'] = fake
    return env


# ------------------------------------------------------------------ window

@dataclass
class Window:
    pid: int
    window_id: int
    bounds: dict


def find_main_window(pid: int) -> Window | None:
    """Largest window of the process (skips tiny helper windows)."""
    best = None
    for w in Quartz.CGWindowListCopyWindowInfo(
            Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID):
        if int(w.get('kCGWindowOwnerPID', -1)) != pid:
            continue
        b = dict(w['kCGWindowBounds'])
        area = b['Width'] * b['Height']
        if best is None or area > best[0]:
            best = (area, int(w['kCGWindowNumber']), b)
    if best is None:
        return None
    return Window(pid=pid, window_id=best[1], bounds=best[2])


def wait_main_window(pid: int, timeout: float = 30.0) -> Window:
    deadline = time.time() + timeout
    while time.time() < deadline:
        win = find_main_window(pid)
        if win:
            return win
        time.sleep(1)
    raise RuntimeError(f'no CGWindow appeared for pid={pid} within {timeout}s')


def assert_offscreen(win: Window, margin: int = 100) -> None:
    dw, dh = _display_size()
    b = win.bounds
    overlap = not (b['X'] + b['Width'] <= 0 or b['Y'] + b['Height'] <= 0
                   or b['X'] >= dw or b['Y'] >= dh)
    if overlap or b['X'] > -margin or b['Y'] > -margin:
        raise AssertionError(
            f'window is not fully off-screen: bounds={b} display={dw}x{dh}')


# ------------------------------------------------------------------ frames

def capture_window(win: Window, name: str, out_dir: str) -> Image.Image | None:
    """Off-screen window frame; image px == 2x window CSS px (no shadow)."""
    import os
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, name if name.endswith('.png') else f'{name}.png')
    img = Quartz.CGWindowListCreateImage(
        Quartz.CGRectNull, Quartz.kCGWindowListOptionIncludingWindow,
        win.window_id, Quartz.kCGWindowImageBoundsIgnoreFraming)
    if img is None:
        return None
    url = Quartz.CFURLCreateFromFileSystemRepresentation(
        None, out.encode(), len(out.encode()), False)
    dest = Quartz.CGImageDestinationCreateWithURL(url, 'public.png', 1, None)
    if dest is None:
        return None
    Quartz.CGImageDestinationAddImage(dest, img, None)
    Quartz.CGImageDestinationFinalize(dest)
    return Image.open(out).convert('RGB')


# ------------------------------------------------ theme-agnostic detectors

def dominant_bg(img: Image.Image) -> tuple[int, int, int]:
    from collections import Counter
    px = img.load()
    c: Counter = Counter()
    for x in range(0, img.width, 24):
        for y in range(0, img.height, 24):
            r, g, b = px[x, y]
            c[(r // 16, g // 16, b // 16)] += 1
    (r16, g16, b16), _ = c.most_common(1)[0]
    return (r16 * 16 + 8, g16 * 16 + 8, b16 * 16 + 8)


def color_dist(p, q) -> int:
    return abs(p[0] - q[0]) + abs(p[1] - q[1]) + abs(p[2] - q[2])


def text_rows(img: Image.Image, bg=None) -> int:
    """Count text-like horizontal bands in the lower 2/3 of the frame.
    Works in light and dark themes (contrast against the dominant bg)."""
    bg = bg or dominant_bg(img)
    px = img.load()
    rows = 0
    for y in range(img.height // 3, int(img.height * 0.92), 4):
        n = sum(1 for x in range(450, min(1900, img.width), 4)
                if color_dist(px[x, y], bg) > 90)
        rows += 1 if n > 5 else 0
    return rows


def nonblank_samples(img: Image.Image, bg=None) -> int:
    bg = bg or dominant_bg(img)
    px = img.load()
    return sum(1 for x in range(0, img.width, 12)
               for y in range(0, img.height, 12)
               if color_dist(px[x, y], bg) > 90)


def frame_sha(img: Image.Image) -> str:
    return hashlib.sha256(img.tobytes()).hexdigest()[:12]


# ------------------------------------------------------------ no-disturb

@dataclass
class DisturbReport:
    cursor_before: tuple = field(default_factory=tuple)
    cursor_after: tuple = field(default_factory=tuple)
    frontmost_before: str = ''
    frontmost_after: str = ''
    display_sha_before: str = ''
    display_sha_after: str = ''
    display_sha_noise: str = ''

    @property
    def cursor_unchanged(self) -> bool:
        return bool(self.cursor_before) and self.cursor_before == self.cursor_after

    @property
    def frontmost_unchanged(self) -> bool:
        return bool(self.frontmost_before) and \
            self.frontmost_before == self.frontmost_after

    def summary(self) -> dict:
        return {
            'cursor_unchanged': self.cursor_unchanged,
            'frontmost_unchanged': self.frontmost_unchanged,
            'display_before_after_identical':
                self.display_sha_before == self.display_sha_after,
            'display_noise_baseline_identical':
                self.display_sha_before == self.display_sha_noise,
            'note': 'the driver posts zero HID events; screen-hash differences '
                    'come from the owner working (see the noise baseline)',
        }


def _cursor() -> tuple:
    p = Quartz.CGEventGetLocation(Quartz.CGEventCreate(None))
    return (round(p.x, 1), round(p.y, 1))


def _frontmost() -> str:
    app = NSWorkspace.sharedWorkspace().frontmostApplication()
    return f'{app.localizedName()}({app.processIdentifier()})' if app else 'None'


def _display_sha(out_path: str) -> str:
    r = subprocess.run(['screencapture', '-x', out_path], capture_output=True)
    if r.returncode != 0:
        return f'rc={r.returncode}'
    return hashlib.sha256(open(out_path, 'rb').read()).hexdigest()[:16]


class DisturbWatch:
    """Captures before/after evidence that the run disturbed nobody."""

    def __init__(self, work_dir: str):
        import os
        self.work_dir = work_dir
        os.makedirs(work_dir, exist_ok=True)
        self.noise = _display_sha(f'{work_dir}/display-noise-a.png')
        time.sleep(2)
        b = _display_sha(f'{work_dir}/display-noise-b.png')
        # a differing A/B pair means the owner is actively using the machine
        self.noise_differs = b != self.noise
        self.before = DisturbReport()
        self.before.display_sha_before = _display_sha(
            f'{work_dir}/display-pre.png')
        self.before.cursor_before = _cursor()
        self.before.frontmost_before = _frontmost()

    def finish(self) -> DisturbReport:
        rep = self.before
        rep.display_sha_noise = self.noise
        rep.display_sha_after = _display_sha(f'{self.work_dir}/display-post.png')
        rep.cursor_after = _cursor()
        rep.frontmost_after = _frontmost()
        return rep


# --------------------------------------------------------------- AX channel

def _ax_attr(el, name: str):
    err, val = AS.AXUIElementCopyAttributeValue(el, name, None)
    return val if err == 0 else None


def _ax_walk_set_manual(el, depth: int = 0) -> bool:
    """Set AXManualAccessibility=YES on the AXWebArea (enables the web a11y
    tree). Returns True when the web area was found."""
    if depth > 10:
        return False
    if _ax_attr(el, 'AXRole') == 'AXWebArea':
        AS.AXUIElementSetAttributeValue(el, 'AXManualAccessibility',
                                        Quartz.kCFBooleanTrue)
        return True
    for k in (_ax_attr(el, 'AXChildren') or []):
        if _ax_walk_set_manual(k, depth + 1):
            return True
    return False


def _ax_solicit(wins) -> None:
    """Actively solicit the WKWebView accessibility tree.

    The web tree hydrates lazily and per-instance unreliably off-screen
    (2026-09-27: three fresh instances in a row never exposed an AXWebArea
    with passive polling alone). Poking AXManualAccessibility on the window
    and AXEnhancedUserInterface on the app element each poll — even where
    the setters report kAXErrorAttributeUnsupported — coincides with the
    tree appearing (verified by /tmp/rimloc-ax-diagnose*.py). Errors are
    deliberately ignored: these are best-effort nudges, the real signal is
    the walk below finding an AXWebArea.
    """
    if not wins:
        return
    AS.AXUIElementSetAttributeValue(wins[0], 'AXManualAccessibility',
                                    Quartz.kCFBooleanTrue)
    app_el = AS.AXUIElementCreateApplication(_solicit_pid)
    AS.AXUIElementSetAttributeValue(app_el, 'AXEnhancedUserInterface',
                                    Quartz.kCFBooleanTrue)


# pid of the app under drive, set by ax_press/ax_button_names/ax_set_text
_solicit_pid = 0


def _ax_buttons(root):
    """Pressable web elements. HTML buttons carrying aria-pressed are mapped
    by WebKit to AXCheckBox (not AXButton), so collect the pressable roles."""
    found = []

    def rec(el, depth=0):
        if depth > 25:
            return
        if _ax_attr(el, 'AXRole') in ('AXButton', 'AXCheckBox', 'AXRadioButton'):
            name = (_ax_attr(el, 'AXTitle') or _ax_attr(el, 'AXDescription')
                    or _ax_attr(el, 'AXLabel') or '')
            found.append((el, str(name)))
        for k in (_ax_attr(el, 'AXChildren') or []):
            rec(k, depth + 1)

    rec(root)
    return found


def ax_press(pid: int, wanted: str, timeout: float = 60.0,
             exact: bool = False) -> str:
    """AXPress the first web AXButton whose accessible name matches.

    Robustness notes:
      - The app-side AX bridge is lazy right after launch: AXWindows may read
        empty for a while, and the web tree may never hydrate on passive
        polling alone. Re-create the app element each attempt, actively
        solicit the tree (_ax_solicit) and keep polling until hydration.
      - WebKit RESETS AXManualAccessibility whenever the page re-renders
        (locale switch, navigation), so re-set it on every attempt instead of
        caching the flag.
    """
    global _solicit_pid
    _solicit_pid = pid
    deadline = time.time() + timeout
    attempts = 0
    while time.time() < deadline:
        attempts += 1
        app = AS.AXUIElementCreateApplication(pid)
        wins = _ax_attr(app, 'AXWindows') or []
        if wins:
            _ax_solicit(wins)
            _ax_walk_set_manual(wins[0])  # idempotent; survives page reloads
            for el, name in _ax_buttons(wins[0]):
                ok = name == wanted if exact else wanted.lower() in name.lower()
                if ok:
                    err = AS.AXUIElementPerformAction(el, 'AXPress')
                    return f'pressed "{name}" err={err} (attempts={attempts})'
        time.sleep(1.5)
    return f'button "{wanted}" not found (attempts={attempts})'


def ax_button_names(pid: int, timeout: float = 20.0) -> list[str]:
    """Best-effort snapshot of visible web-button names (diagnostics)."""
    global _solicit_pid
    _solicit_pid = pid
    deadline = time.time() + timeout
    while time.time() < deadline:
        app = AS.AXUIElementCreateApplication(pid)
        wins = _ax_attr(app, 'AXWindows') or []
        if wins:
            _ax_solicit(wins)
            _ax_walk_set_manual(wins[0])  # re-set: page loads reset the flag
            names = [n for _, n in _ax_buttons(wins[0]) if n]
            if names:
                return names
        time.sleep(1.5)
    return []


_TEXT_ROLES = ('AXTextField', 'AXTextArea', 'AXComboBox')


def _ax_text_fields(root):
    """Editable web text fields with their naming attributes joined."""
    found = []

    def rec(el, depth=0):
        if depth > 25:
            return
        if _ax_attr(el, 'AXRole') in _TEXT_ROLES:
            name = ' '.join(
                str(_ax_attr(el, attr) or '')
                for attr in ('AXTitle', 'AXDescription', 'AXLabel',
                             'AXPlaceholderValue', 'AXValue'))
            found.append((el, name))
        for k in (_ax_attr(el, 'AXChildren') or []):
            rec(k, depth + 1)

    rec(root)
    return found


def ax_set_text(pid: int, match: str, value: str, timeout: float = 30.0) -> str:
    """Set AXValue on the first web text field whose label/description/value
    contains `match`. WebKit's AX value-set replaces the DOM value; whether
    the framework binding (Svelte bind:value) observes it is verified by the
    caller through a subsequent UI consequence (e.g. the export result)."""
    global _solicit_pid
    _solicit_pid = pid
    deadline = time.time() + timeout
    attempts = 0
    while time.time() < deadline:
        attempts += 1
        app = AS.AXUIElementCreateApplication(pid)
        wins = _ax_attr(app, 'AXWindows') or []
        if wins:
            _ax_solicit(wins)
            _ax_walk_set_manual(wins[0])
            for el, name in _ax_text_fields(wins[0]):
                if match.lower() in name.lower():
                    # focus the field first: some web frameworks only commit
                    # an AX value-set onto a focused input
                    AS.AXUIElementSetAttributeValue(
                        el, 'AXFocused', Quartz.kCFBooleanTrue)
                    time.sleep(0.2)
                    err = AS.AXUIElementSetAttributeValue(
                        el, 'AXValue', value)
                    if err != 0:
                        return (f'set "{match}" failed err={err} '
                                f'(attempts={attempts})')
                    time.sleep(0.6)
                    readback = _ax_attr(el, 'AXValue')
                    note = (f'set "{match}" -> {value!r} '
                            f'readback={readback!r} (attempts={attempts})')
                    if readback != value:
                        note += (' [REVERTED: framework binding overwrote '
                                 'the AX value]')
                    return note
        time.sleep(1.5)
    return f'text field "{match}" not found (attempts={attempts})'


# ------------------------------------------------------- synthetic mouse

def post_to_pid_click(pid: int, x_global: float, y_global: float,
                      window_id: int | None = None) -> None:
    """Synthetic mouse click delivered ONLY to the app process (no HID, the
    user's cursor never moves). NOTE: off-screen this drives CSS :hover but
    NSWindow swallows the actual click for an inactive app; use ax_press.
    `window_id` fills the windowUnderMousePointer event fields so the event
    routes to the window at all."""
    q = Quartz
    for etype, click_state in ((q.kCGEventMouseMoved, 0),
                               (q.kCGEventLeftMouseDown, 1),
                               (q.kCGEventLeftMouseUp, 1)):
        ev = q.CGEventCreateMouseEvent(None, etype, (x_global, y_global),
                                       q.kCGMouseButtonLeft)
        q.CGEventSetIntegerValueField(ev, q.kCGMouseEventClickState, click_state)
        if window_id:
            q.CGEventSetIntegerValueField(ev, 91, window_id)  # windowUnderMousePointer
            q.CGEventSetIntegerValueField(ev, 92, window_id)  # ...ThatCanHandleThisEvent
        q.CGEventPostToPid(pid, ev)
        time.sleep(0.12)
