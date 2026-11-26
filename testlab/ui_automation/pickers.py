"""Native-AX driver for RimLoc's file pickers (NSOpenPanel).

The app's Browse buttons («Выбрать…»/«Browse…») go through
tauri-plugin-dialog → rfd → NSOpenPanel: a NATIVE panel the web-side AX
tree knows nothing about. This module drives that panel with pure AX
value/action manipulation — no keystrokes, no app activation, no
synthetic mouse (owner may be working; see rimloc-gui-testing skill).

Mechanics (all verified against the live panel by the T7 lane):

  - find the panel: an AXSheet attached to the main window OR an app
    window with a dialog subrole (AXSystemDialog/AXFloatingWindow) —
    in-process NSOpenPanel, the app is not sandboxed;
  - navigate: select AXRow elements (sidebar favourites and browser
    rows) by their flattened static text, then AXPress the panel's
    confirm button by bilingual name («Открыть»/«Open»). Folder-mode
    semantics differ across macOS builds (selecting a folder + Open may
    return it or descend into it), so `walk_to` adapts: after each open
    press it checks whether the panel is still up;
  - cancel: AXPress «Отмена»/«Cancel» (never keystroke Escape);
  - readback: the web text inputs' AXValue is the state-based assertion
    surface — what the frontend stored after the panel closed.

Every inspection primitive can dump its subtree as JSON so a failing
round carries its own evidence into target/logs/.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path

import ApplicationServices as AS
import Quartz
from AppKit import NSWorkspace

from . import driver as drv
from .lifecycle import OwnedInstance

# ------------------------------------------------------------------ const

OPEN_NAMES = ('Открыть', 'Open')
CANCEL_NAMES = ('Отмена', 'Cancel')
DIALOG_SUBROLES = ('AXSheet', 'AXSystemDialog', 'AXFloatingWindow')
WEB_PRESS_ROLES = ('AXButton', 'AXCheckBox', 'AXRadioButton', 'AXLink')

DEFAULTS_DOMAIN = 'com.rimloc.gui'
NAV_LAST_ROOT_KEY = 'NSNavLastRootDirectory'


# --------------------------------------------------------------- AX utils

def _app_el(pid: int):
    return AS.AXUIElementCreateApplication(pid)


def _role(el) -> str:
    return str(drv._ax_attr(el, 'AXRole') or '')


def _subrole(el) -> str:
    return str(drv._ax_attr(el, 'AXSubrole') or '')


def _el_name(el) -> str:
    """Flatten every naming attribute WebKit/AppKit may carry."""
    return ' '.join(
        str(drv._ax_attr(el, attr) or '')
        for attr in ('AXTitle', 'AXDescription', 'AXLabel')
    ).strip()


def _value(el):
    return drv._ax_attr(el, 'AXValue')


def _ax_point(v) -> tuple:
    ok, pt = AS.AXValueGetValue(v, AS.kAXValueCGPointType, None)
    return (pt.x, pt.y) if ok else (None, None)


def _ax_size(v) -> tuple:
    ok, sz = AS.AXValueGetValue(v, AS.kAXValueCGSizeType, None)
    return (sz.width, sz.height) if ok else (None, None)


def _frame(el) -> dict | None:
    """AXPosition+AXSize; the attributes come back as raw AXValue refs,
    not dicts — unwrap via AXValueGetValue (pyobjc)."""
    err, pos = AS.AXUIElementCopyAttributeValue(el, 'AXPosition', None)
    err2, size = AS.AXUIElementCopyAttributeValue(el, 'AXSize', None)
    if err != 0 or err2 != 0:
        return None
    x, y = _ax_point(pos)
    w, h = _ax_size(size)
    if x is None or w is None:
        return None
    return {'x': x, 'y': y, 'w': w, 'h': h}


def _walk(el, fn, depth: int = 0, max_depth: int = 25) -> None:
    if depth > max_depth:
        return
    fn(el, depth)
    for child in (drv._ax_attr(el, 'AXChildren') or []):
        _walk(child, fn, depth + 1, max_depth)


def _flatten_text(el) -> str:
    """All static text under an element (row labels, browser entries)."""
    parts: list[str] = []

    def visit(e, _d):
        if _role(e) in ('AXStaticText', 'AXTextField'):
            t = _value(e)
            if t:
                parts.append(str(t))
    _walk(el, visit, max_depth=8)
    return ' '.join(parts)


# ----------------------------------------------------------------- panel

@dataclass
class PanelHandle:
    element: object          # AXSheet or AXWindow of the open panel
    kind: str                # 'sheet' | 'window'
    window_element: object   # parent AXWindow (== element for windows)
    frame: dict | None
    appeared_after_s: float


def app_windows(pid: int) -> list:
    """AX window handles of a BACKGROUND app. AXWindows is routinely
    empty when the app has never been focused (ax/README lesson) while
    AXMainWindow/AXFocusedWindow still resolve — merge all three,
    deduped by frame (AX elements do not compare by identity)."""
    app = _app_el(pid)
    found: list = []
    frames: list[tuple] = []
    NONE_KEY = (None, None, None, None)

    def add(el) -> None:
        if el is None:
            return
        fr = _frame(el) or {}
        key = (fr.get('x'), fr.get('y'), fr.get('w'), fr.get('h'))
        if key != NONE_KEY and key in frames:
            return
        frames.append(key)
        found.append(el)

    for w in (drv._ax_attr(app, 'AXWindows') or []):
        add(w)
    add(drv._ax_attr(app, 'AXMainWindow'))
    add(drv._ax_attr(app, 'AXFocusedWindow'))
    return found


def window_sheets(win) -> list:
    return drv._ax_attr(win, 'AXSheets') or []


def find_panel(pid: int) -> PanelHandle | None:
    """The open NSOpenPanel: attached sheet or dialog-class window.

    Heuristic, in priority order:
      1. AXSheets on any app window (role AXSheet);
      2. an app window whose AXSubrole marks a dialog/floating panel;
      3. an app window that carries the panel's button signature
         (an AXButton named Открыть/Open) but is NOT the main window —
         last-resort for macOS builds that expose sheets oddly.
    """
    wins = app_windows(pid)
    # (1) sheets attached to any window
    for win in wins:
        for sheet in window_sheets(win):
            return PanelHandle(sheet, 'sheet', win, _frame(sheet), -1.0)
    # (2) dialog-subrole windows
    for win in wins:
        if _subrole(win) in DIALOG_SUBROLES:
            return PanelHandle(win, 'window', win, _frame(win), -1.0)
    # (3) button-signature window (native part only — a web «Отмена» in the
    # main window's AXWebArea must never pass for the panel)
    for win in wins:
        btns = native_buttons(win)
        if any(_matches(bn, OPEN_NAMES) or _matches(bn, CANCEL_NAMES)
               for _, bn in btns):
            return PanelHandle(win, 'window', win, _frame(win), -1.0)
    return None


def native_buttons(root) -> list[tuple[object, str]]:
    """AXButtons OUTSIDE any AXWebArea subtree (native AppKit buttons)."""
    found: list[tuple[object, str]] = []

    def visit(el, in_web, _d):
        role = _role(el)
        if role == 'AXWebArea':
            in_web = True
        if role == 'AXButton' and not in_web:
            found.append((el, _el_name(el)))
        for child in (drv._ax_attr(el, 'AXChildren') or []):
            visit(child, in_web, _d + 1)

    visit(root, False, 0)
    return found


def wait_panel(pid: int, timeout: float = 25.0) -> PanelHandle:
    t0 = time.time()
    while time.time() - t0 < timeout:
        panel = find_panel(pid)
        if panel is not None:
            panel.appeared_after_s = time.time() - t0
            return panel
        time.sleep(0.5)
    raise TimeoutError(f'native open panel did not appear within {timeout}s')


def panel_gone(pid: int, timeout: float = 15.0) -> bool:
    t0 = time.time()
    while time.time() - t0 < timeout:
        if find_panel(pid) is None:
            return True
        time.sleep(0.4)
    return False


def _matches(name: str, wanted: tuple) -> bool:
    n = (name or '').strip().lower()
    return any(w.lower() == n or n.startswith(w.lower()) for w in wanted)


def panel_buttons(panel) -> list[tuple[object, str]]:
    found: list[tuple[object, str]] = []

    def visit(el, _d):
        if _role(el) == 'AXButton':
            found.append((el, _el_name(el)))
    _walk(panel, visit)
    return found


def press_button(el) -> int:
    return AS.AXUIElementPerformAction(el, 'AXPress')


def press_open(panel: PanelHandle) -> bool:
    for el, name in panel_buttons(panel.element):
        if _matches(name, OPEN_NAMES):
            return press_button(el) == 0
    # fallback: the panel's default button IS Открыть
    default = drv._ax_attr(panel.element, 'AXDefaultButton')
    if default is not None:
        return press_button(default) == 0
    return False


def press_cancel(panel: PanelHandle) -> bool:
    for el, name in panel_buttons(panel.element):
        if _matches(name, CANCEL_NAMES):
            return press_button(el) == 0
    cancel = drv._ax_attr(panel.element, 'AXCancelButton')
    if cancel is not None:
        return press_button(cancel) == 0
    return False


# ------------------------------------------------------------- panel rows

def panel_rows(panel) -> list[tuple[object, str]]:
    """Selectable rows (sidebar + file browser) with flattened labels."""
    found: list[tuple[object, str]] = []

    def visit(el, _d):
        if _role(el) in ('AXRow', 'AXListRow'):
            label = _flatten_text(el)
            if label.strip():
                found.append((el, label.strip()))
    _walk(panel, visit)
    return found


def find_row(panel: PanelHandle, name: str) -> object | None:
    """Row for `name`. List-view rows flatten to e.g.
    'pack-a  вчера 14:02  --  папка', so exact full-label match first,
    then first-token match, then shortest containment ('pack-a' must not
    hit a row that merely mentions it)."""
    rows = panel_rows(panel)
    for el, label in rows:
        if label == name:
            return el
    for el, label in rows:
        if label.split(' ')[0] == name:
            return el
    candidates = [(el, label) for el, label in rows if name in label]
    return (min(candidates, key=lambda t: len(t[1]))[0]) if candidates else None


def select_row(row_el) -> bool:
    """AXSelected on the row; fallback: AXSelectedRows on the ancestor
    outline/table (some macOS builds only honour the container attr)."""
    ok = AS.AXUIElementSetAttributeValue(
        row_el, 'AXSelected', Quartz.kCFBooleanTrue) == 0
    if ok:
        readback = drv._ax_attr(row_el, 'AXSelected')
        if readback:
            return True
    parent = drv._ax_attr(row_el, 'AXParent')
    for _ in range(8):
        if parent is None:
            break
        if _role(parent) in ('AXOutline', 'AXTable', 'AXList', 'AXBrowser'):
            arr = Quartz.CFArrayCreate(
                None, (row_el,), 1, Quartz.kCFTypeArrayCallBacks)
            if AS.AXUIElementSetAttributeValue(
                    parent, 'AXSelectedRows', arr) == 0:
                return True
        parent = drv._ax_attr(parent, 'AXParent')
    return ok


def sidebar_select(panel: PanelHandle, names: tuple[str, ...]) -> bool:
    """Select a sidebar favourite (Документы/Documents, home, …)."""
    for el, label in panel_rows(panel.element):
        first = label.split(' ')[0].strip()
        for want in names:
            if label == want or label.startswith(want) or first == want:
                if select_row(el):
                    time.sleep(0.8)  # browser content refresh
                    return True
    return False


# ------------------------------------------------------------ navigation

def walk_to(pid: int, panel: PanelHandle, folders: list[str]) -> str:
    """Descend into `folders` in order by select-row + Открыть.

    Returns 'returned-early' when the panel already CLOSED after one of
    the open presses (folder-mode variant where selecting a folder and
    confirming returns it instead of descending) — the caller then treats
    the LAST folder as the picked directory. 'descended' means every
    folder was entered and the panel is still up.
    """
    for i, name in enumerate(folders):
        row = find_row(panel, name)
        if row is None:
            raise LookupError(f'row {name!r} not in panel '
                              f'(visible rows: {[l for _, l in panel_rows(panel.element)]})')
        if not select_row(row):
            raise RuntimeError(f'AXSelected refused on row {name!r}')
        time.sleep(0.3)
        if not press_open(panel):
            raise RuntimeError(f'Открыть press failed at {name!r}')
        time.sleep(0.8)
        next_panel = find_panel(pid)
        if next_panel is None:
            # panel closed: this macOS build returned the folder we had
            # selected instead of descending into it
            return f'returned-at-{i}'
        panel = next_panel
    return 'descended'


def finish_folder_pick(pid: int) -> None:
    """After full descent: confirm the current directory (nothing
    selected → Открыть returns it)."""
    panel = find_panel(pid)
    if panel is None:
        return
    press_open(panel)
    time.sleep(0.6)


def where_popup(el):
    """The panel's «Where:» path popup (AXPopUpButton) — its AXValue is
    the current directory the panel displays."""
    found = []

    def visit(e, _d):
        if _role(e) == 'AXPopUpButton' and 'where' in _el_name(e).lower():
            found.append(e)

    _walk(el, visit, max_depth=25)
    return found[0] if found else None


def open_button(panel: PanelHandle):
    for el, name in panel_buttons(panel.element):
        if _matches(name, OPEN_NAMES):
            return el
    return drv._ax_attr(panel.element, 'AXDefaultButton')


def wait_open_enabled(panel: PanelHandle, timeout: float = 25.0) -> bool:
    """Poll the confirm button's AXEnabled before pressing it.

    On this macOS build the panel's Open button stays disabled until the
    file-provider/browser finishes loading the freshly-preset directory
    (observed live: Where: already shows the preset while Open is still
    disabled; AXPress on a disabled button returns err=0 and does
    NOTHING — the silent no-op that failed the first lane runs).
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        el = open_button(panel)
        if el is not None:
            enabled = drv._ax_attr(el, 'AXEnabled')
            if enabled:
                return True
        time.sleep(0.5)
    return False


def confirm_dir(pid: int, timeout: float = 30.0) -> None:
    """Folder-mode confirm for the memory-preset seam: wait for the Open
    button to become enabled, then press it. Raises if it never enables."""
    panel = find_panel(pid)
    if panel is None:
        return
    if not wait_open_enabled(panel, timeout=timeout):
        el = open_button(panel)
        state = drv._ax_attr(el, 'AXEnabled') if el is not None else None
        raise RuntimeError(
            f'Open button still disabled after {timeout}s (enabled={state!r})')
    if not press_open(panel):
        raise RuntimeError('Открыть press failed (memory-preset confirm)')


def choose_file(pid: int, panel: PanelHandle, filename: str,
                timeout: float = 12.0) -> None:
    """Select the file row and confirm. The browser content may still be
    loading right after the panel opens — poll for the row."""
    row = None
    deadline = time.time() + timeout
    while time.time() < deadline:
        row = find_row(panel, filename)
        if row is not None:
            break
        time.sleep(0.6)
    if row is None:
        raise LookupError(f'file row {filename!r} not in panel '
                          f'(rows: {[l for _, l in panel_rows(panel.element)][:20]})')
    if not select_row(row):
        raise RuntimeError(f'AXSelected refused on file row {filename!r}')
    time.sleep(0.3)
    if not press_open(panel):
        raise RuntimeError('Открыть press failed on file selection')


# ----------------------------------------------------------------- dumps

def dump_tree(el, out_path: str, max_depth: int = 16) -> dict:
    def build(e, depth):
        node = {
            'role': _role(e),
            'subrole': _subrole(e),
            'name': _el_name(e)[:160],
        }
        val = _value(e)
        if val is not None and not isinstance(val, (list, tuple)):
            node['value'] = str(val)[:160]
        if _role(e) in ('AXButton', 'AXLink'):
            acts = drv._ax_attr(e, 'AXActions') or []
            node['actions'] = [str(a) for a in acts][:6]
        if depth < max_depth:
            kids = []
            for c in (drv._ax_attr(e, 'AXChildren') or []):
                kids.append(build(c, depth + 1))
            if kids:
                node['children'] = kids
        return node

    tree = build(el, 0)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(tree, f, ensure_ascii=False, indent=1)
    return tree


def capture_panel_png(pid: int, panel: PanelHandle, out_path: str) -> str | None:
    """Screenshot the panel window by matching its AX frame to a
    CGWindow (CGWindowListCreateImage works for background windows)."""
    fr = panel.frame or _frame(panel.window_element)
    if fr is None:
        return None
    for w in Quartz.CGWindowListCopyWindowInfo(
            Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID):
        if int(w.get('kCGWindowOwnerPID', -1)) != pid:
            continue
        b = dict(w['kCGWindowBounds'])
        if (abs(b['X'] - fr['x']) <= 3 and abs(b['Y'] - fr['y']) <= 3
                and abs(b['Width'] - fr['w']) <= 3
                and abs(b['Height'] - fr['h']) <= 3):
            win_id = int(w['kCGWindowNumber'])
            img = Quartz.CGWindowListCreateImage(
                Quartz.CGRectNull,
                Quartz.kCGWindowListOptionIncludingWindow,
                win_id, Quartz.kCGWindowImageBoundsIgnoreFraming)
            if img is None:
                return None
            os.makedirs(os.path.dirname(out_path), exist_ok=True)
            url = Quartz.CFURLCreateFromFileSystemRepresentation(
                None, out_path.encode(), len(out_path.encode()), False)
            dest = Quartz.CGImageDestinationCreateWithURL(
                url, 'public.png', 1, None)
            Quartz.CGImageDestinationAddImage(dest, img, None)
            Quartz.CGImageDestinationFinalize(dest)
            return out_path
    return None


# ------------------------------------------------------------- web side

def _solicit_web(pid: int, wins) -> None:
    drv._solicit_pid = pid
    if wins:
        drv._ax_solicit(wins)
        drv._ax_walk_set_manual(wins[0])


def web_pressables(pid: int, timeout: float = 30.0) -> list[tuple[object, str]]:
    """Pressable web elements (buttons incl. aria-pressed checkboxes and
    nav links) with accessible names — tree order."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        wins = app_windows(pid)
        if wins:
            _solicit_web(pid, wins)
            found: list[tuple[object, str]] = []

            def visit(el, _d):
                if _role(el) in WEB_PRESS_ROLES:
                    found.append((el, _el_name(el)))
            for w in wins:
                _walk(w, visit)
            if any(n for _, n in found):
                return found
        time.sleep(1.0)
    return []


def press_named(pid: int, name: str, nth: int = 0,
                timeout: float = 60.0) -> str:
    """AXPress the nth web control named `name` (tree order). The two
    «Выбрать…» buttons of a screen are told apart by nth + a readback."""
    deadline = time.time() + timeout
    attempts = 0
    hits: list = []
    while time.time() < deadline:
        attempts += 1
        hits = [(el, n) for el, n in web_pressables(pid, timeout=2.0)
                if name.lower() in (n or '').lower()]
        if len(hits) > nth:
            err = AS.AXUIElementPerformAction(hits[nth][0], 'AXPress')
            if err == 0:
                return (f'pressed {name!r}[{nth}] err=0 '
                        f'(attempts={attempts})')
        time.sleep(1.2)
    return (f'button {name!r}[{nth}] not pressable '
            f'(attempts={attempts}, hits={len(hits)})')


def text_inputs(pid: int, timeout: float = 20.0) -> list[tuple[object, str, str]]:
    """(element, joined-name, AXValue) of editable web text fields."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        wins = app_windows(pid)
        if wins:
            _solicit_web(pid, wins)
            found: list[tuple[object, str, str]] = []

            def visit(el, _d):
                if _role(el) in drv._TEXT_ROLES:
                    name = drv._ax_attr(el, 'AXTitle') or drv._ax_attr(
                        el, 'AXDescription') or drv._ax_attr(el, 'AXLabel')
                    joined = ' '.join(
                        str(drv._ax_attr(el, a) or '')
                        for a in ('AXTitle', 'AXDescription', 'AXLabel',
                                  'AXPlaceholderValue'))
                    found.append((el, str(name or joined), str(_value(el) or '')))
            for w in wins:
                _walk(w, visit)
            if found:
                return found
        time.sleep(1.0)
    return []


def read_input(pid: int, candidates: tuple[str, ...],
               timeout: float = 15.0) -> str | None:
    """AXValue of the first web text field whose name mentions any of
    `candidates` (bilingual i18n labels). None = field not found."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        for el, name, value in text_inputs(pid, timeout=2.0):
            if any(c.lower() in name.lower() for c in candidates):
                return value
        time.sleep(1.0)
    return None


def web_button_names(pid: int, timeout: float = 20.0) -> list[str]:
    return [n for _, n in web_pressables(pid, timeout) if n]


# ------------------------------------------------------------ app launch

def launch(bin_path: str, evidence_dir: str, data_dir: str,
           frame: str = '348,70,980x640') -> OwnedInstance:
    """Spawn the automation build per the spawn-wrapper-release.sh
    pattern (direct binary, RIMLOC_AUTOMATION=1, ephemeral frame, zero
    activations) with an isolated RIMLOC_DATA_DIR. App stdout/stderr go
    to the evidence dir, not /tmp."""
    # The app is spawned with cwd=<evidence dir>; relative paths would
    # break the binary lookup AND the broker's managed-root check, which
    # refuses non-absolute RIMLOC_DATA_DIR values.
    bin_path = str(Path(bin_path).resolve())
    evidence_dir = str(Path(evidence_dir).resolve())
    data_dir = str(Path(data_dir).resolve())
    os.makedirs(evidence_dir, exist_ok=True)
    os.makedirs(data_dir, exist_ok=True)
    env = dict(os.environ)
    env.update({
        'RUST_BACKTRACE': '1',
        'RIMLOC_AUTOMATION': '1',
        'RIMLOC_TRACE': '1',
        'RIMLOC_WINDOW_FRAME': frame,
        'RIMLOC_DATA_DIR': data_dir,
    })
    out_log = open(os.path.join(evidence_dir, 'app-stdout.log'), 'wb')
    err_log = open(os.path.join(evidence_dir, 'app-stderr.log'), 'wb')
    inst = OwnedInstance(pid_file=os.path.join(evidence_dir, 'app.instance.pid'))
    inst.proc = subprocess.Popen(
        [bin_path], env=env, stdout=out_log, stderr=err_log,
        cwd=evidence_dir)
    inst.pid = inst.proc.pid
    inst._register()
    return inst


def wait_ready(pid: int, timeout: float = 120.0) -> list[str]:
    """Main window visible + web AX tree hydrated (button names non-empty)."""
    drv.wait_main_window(pid, timeout=timeout)
    deadline = time.time() + timeout
    while time.time() < deadline:
        names = web_button_names(pid, timeout=2.0)
        if names:
            return names
        time.sleep(1.5)
    raise TimeoutError('web AX tree never hydrated (no named buttons)')


# ------------------------------------------------------- defaults memory

def _safe_defaults_value(value: str) -> str:
    """Guard the values that round-trip through `defaults` (CodeQL:
    uncontrolled data must not reach a command line). A panel location is
    always an absolute path without control characters; anything else is
    refused before it can flow back into a subprocess argv."""
    v = str(value)
    if not v.startswith('/') or any(c in v for c in '\x00\n\r;|&$`'):
        raise ValueError(f'refusing unsafe defaults value: {v!r}')
    return v


def defaults_read(key: str = NAV_LAST_ROOT_KEY) -> str | None:
    r = subprocess.run(['defaults', 'read', DEFAULTS_DOMAIN, key],
                       capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def defaults_write(key: str, value: str) -> None:
    value = _safe_defaults_value(value)
    subprocess.run(['defaults', 'write', DEFAULTS_DOMAIN, key,
                    '-string', value], check=True)


def defaults_delete(key: str = NAV_LAST_ROOT_KEY) -> None:
    subprocess.run(['defaults', 'delete', DEFAULTS_DOMAIN, key],
                   capture_output=True)


def defaults_restore(key: str, old: str | None) -> None:
    if old is None:
        defaults_delete(key)
    else:
        defaults_write(key, old)


# ------------------------------------------------------------ disturb

def frontmost_app() -> tuple[int, str]:
    app = NSWorkspace.sharedWorkspace().frontmostApplication()
    if app is None:
        return (-1, 'None')
    return (int(app.processIdentifier()), str(app.localizedName()))


def restore_frontmost(pid: int) -> str:
    """Hand focus back to `pid`'s app if the modal panel stole it — a
    net restore for the owner, not a drive action."""
    app = NSWorkspace.sharedWorkspace().frontmostApplication()
    if app is None or int(app.processIdentifier()) == pid:
        return 'no-restore-needed'
    target = NSWorkspace.sharedWorkspace().runningApplications()
    for a in target:
        if int(a.processIdentifier()) == pid:
            try:
                a.activateFromApplication_(NSWorkspace.sharedWorkspace(
                    ).frontmostApplication())
                return f'reactivated-pid-{pid}'
            except Exception:
                pass
            try:
                a.activate()
                return f'reactivated-pid-{pid}-plain'
            except Exception:
                return 'restore-failed'
    return 'target-not-running'
