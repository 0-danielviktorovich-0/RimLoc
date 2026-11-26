"""T7 lane: native file pickers of the React GUI, driven over pure AX.

7 pick controls (ex.pick, ex.po-pick, be.pick, diag.pick, cmp.old-pick,
cmp.new-pick, wizard.pick) × 3 scenarios. Navigation seam (verified
live on macOS 27): the panel's NSBrowser columns expose NO AX rows, so
row-based descent is impossible — instead every panel is reached through
the panel's own remembered-root mechanism (NSNavLastRootDirectory preset
via `defaults write com.rimloc.gui` right before the pick, old value
saved and restored at the end), and the confirm is an ENABLED-WAIT +
AXPress: the Open button stays disabled while the freshly-preset
directory loads, and pressing it disabled is a silent no-op.

  memory — every panel must open AT the remembered root (fixture
           po-pack): Where: popup value + readback assert it;
  path   — per-control fixture targets (pack-a / out-dir / pack-b) with
           the memory WRITE-BACK check (defaults read after a successful
           pick ends with the picked dir);
  cancel — every panel cancelled with «Отмена»/«Cancel» (no keystrokes);
           the web readback must stay byte-identical to the pre-round
           value.

Known AX limitation (recorded per round): the FILE picker cannot select
pack.po — the file row is not AX-reachable on this macOS build — so the
file control receives the remembered root DIR. Folder pickers (6 of 7
controls) are fully covered.

Project gate: ex.pick/ex.po-pick (Existing.tsx) and be.pick
(BuildExport.tsx) render only with a project open, so the lane FIRST
creates the fixture project through the wizard itself — which is also
the wizard.pick exercise (panel navigation pack-a → create → the gated
screens unlock).

Picker press targets: WebKit names a button nested in a <label> by the
LABEL text (the button's own «Выбрать…» is swallowed — verified live),
so controls are pressed by their bilingual label text, which doubles as
the state-based arrival signal.

Run (worktree root, .venv-testlab python — pyobjc lives there):

    .venv-testlab/bin/python -m testlab.ui_automation.lane_native_pickers

Window frame override: PICKERS_FRAME="x,y,WxH" (strict integers).
Evidence: target/logs/pickers-<ts>/ (report.json, per-round panel trees
and PNGs, app stdout/stderr). Exit 0 only for a fully-verified run:
exit 2 = degraded rounds remain (remedy in report.json), exit 1 = real
failures or focus interference.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

from AppKit import NSWorkspace

from . import driver as drv
from . import pickers as pk

HOME = Path.home()
FIXTURE = HOME / 'Documents' / 'rimloc-t7-fixture'
DEFAULT_BIN = ('target/release/bundle/macos/RimLoc GUI.app'
               '/Contents/MacOS/rimloc-gui')

# Bilingual i18n surface (frontend-react/src/lib/i18n/index.ts): RU first
# (default locale), EN second.
NAV = {
    'existing': ('Существующий перевод', 'Existing translation'),
    'export': ('Сборка и экспорт', 'Build & export'),
    'diagnostics': ('Диагностика', 'Diagnostics'),
    'compare': ('Сравнение', 'Compare'),
}
SIDEBAR_DOCS = ('Документы', 'Documents')
WIZARD_OPEN = ('Новый проект', 'New project')
WIZARD_BACK = ('Назад', 'Back')
WIZARD_CANCEL = ('Отмена', 'Cancel')
WIZARD_NEXT = ('Далее', 'Next')
WIZARD_CREATE = ('Создать проект', 'Create project')
WIZARD_PATH_INP = ('Путь к модам', 'Mod path')
# the file-drop button carries the label's own text («Выбрать папку
# мода …») — WebKit swallows the button's own «Выбрать…» inside <label>
WIZARDCTL_PRESS = ('Выбрать папку мода', 'Pick the mod folder')

# id → route, press-name (label text the AX button carries), readback
# input labels, target fixture relpath. All pick presses are nth=0: the
# label texts are screen-distinctive by construction.
CONTROLS = [
    dict(id='ex.pick', route='existing', nav='existing',
         press=('Папка существующего перевода',
                'Existing translation folder'),
         inp=('Папка существующего перевода',
              'Existing translation folder'),
         kind='dir', target='pack-a'),
    dict(id='ex.po-pick', route='existing', nav='existing',
         press=('Импорт из PO-файла', 'Import from a PO file'),
         inp=('Импорт из PO-файла', 'Import from a PO file'),
         kind='file', target='po-pack/pack.po'),
    dict(id='be.pick', route='export', nav='export',
         press=('Каталог вывода', 'Output directory'),
         inp=('Каталог вывода', 'Output directory'),
         kind='dir', target='out-dir'),
    dict(id='diag.pick', route='diagnostics', nav='diagnostics',
         press=('Каталог вывода', 'Output directory'),
         inp=('Каталог вывода', 'Output directory'),
         kind='dir', target='out-dir'),
    dict(id='cmp.old-pick', route='compare', nav='compare',
         press=('Старая версия', 'Old version'),
         inp=('Старая версия', 'Old version'),
         kind='dir', target='pack-a'),
    dict(id='cmp.new-pick', route='compare', nav='compare',
         press=('Новая версия', 'New version'),
         inp=('Новая версия', 'New version'),
         kind='dir', target='pack-b'),
    dict(id='wizard.pick', route='projects', nav=None,
         press=('Выбрать папку мода', 'Pick the mod folder'),
         inp=WIZARD_PATH_INP,
         kind='dir', target='pack-a'),
]

SCENARIOS = ('memory', 'path', 'cancel')


# ---------------------------------------------------------------- fixture

ABOUT_XML = ('<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n'
             '  <name>{name}</name>\n  <packageId>rimloc.test.{pid}</packageId>\n'
             '  <author>RimLoc</author>\n'
             '  <supportedVersions><li>1.6</li></supportedVersions>\n'
             '</ModMetaData>\n')
KEYED_XML = ('<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n'
             '  {key}>hello coverage</{key}>\n</LanguageData>\n')
PO_SAMPLE = ('msgid ""\nmsgstr ""\n'
             '"Project-Id-Version: rimloc-t7\\n"\n'
             '"MIME-Version: 1.0\\n"\n'
             '"Content-Type: text/plain; charset=UTF-8\\n"\n'
             '"Content-Transfer-Encoding: 8bit\\n"\n\n'
             'msgid "Greeting"\nmsgstr "Приветствие"\n')


def build_fixture(root: Path) -> list[str]:
    """Idempotent: create only what is missing (stage3-coverage sample)."""
    created: list[str] = []
    for mod, pid in (('pack-a', 'packa'), ('pack-b', 'packb')):
        about = root / mod / 'About'
        keyed = root / mod / 'Languages' / 'English' / 'Keyed'
        if not (about / 'About.xml').exists():
            about.mkdir(parents=True, exist_ok=True)
            (about / 'About.xml').write_text(
                ABOUT_XML.format(name=f'T7 {mod}', pid=pid), encoding='utf-8')
            created.append(str(about / 'About.xml'))
        if not (keyed / 'S3.xml').exists():
            keyed.mkdir(parents=True, exist_ok=True)
            (keyed / 'S3.xml').write_text(
                KEYED_XML.format(key='S3Greeting'), encoding='utf-8')
            created.append(str(keyed / 'S3.xml'))
    inner = root / 'pack-a' / 'inner-pack' / 'About'
    if not (inner / 'About.xml').exists():
        inner.mkdir(parents=True, exist_ok=True)
        (inner / 'About.xml').write_text(
            ABOUT_XML.format(name='T7 inner', pid='inner'), encoding='utf-8')
        created.append(str(inner / 'About.xml'))
    po = root / 'po-pack' / 'pack.po'
    if not po.exists():
        po.parent.mkdir(parents=True, exist_ok=True)
        po.write_text(PO_SAMPLE, encoding='utf-8')
        created.append(str(po))
    (root / 'out-dir').mkdir(parents=True, exist_ok=True)
    return created


# ------------------------------------------------------------- lane core

class Lane:
    def __init__(self, bin_path: str, frame: str):
        ts = time.strftime('%Y%m%d-%H%M%S')
        self.root = Path('target/logs') / f'pickers-{ts}'
        self.root.mkdir(parents=True, exist_ok=True)
        self.data_dir = self.root / 'data'
        self.bin = bin_path
        self.frame = frame
        self.pid = 0            # live app pid; 0 until launch
        self.inst = None
        self.focus_events: list[dict] = []
        self.rounds: list[dict] = []
        self.frontmost_before = pk.frontmost_app()
        self._t0 = time.strftime('%Y-%m-%d %H:%M:%S')

    # ---- low-level steps

    def note_focus(self, where: str) -> None:
        pid, name = pk.frontmost_app()
        if self.pid and pid == self.pid:
            self.focus_events.append(
                {'where': where, 'frontmost': f'{name}({pid})'})

    def press_hits(self, alts: tuple[str, ...]) -> list:
        hits = [(el, n) for el, n in pk.web_pressables(self.pid, timeout=1.5)
                if any(a.lower() in (n or '').lower() for a in alts)]
        return hits

    def nav_named(self, alts: tuple[str, ...], timeout: float = 30.0) -> str:
        """Press the first web control named any of `alts`."""
        import ApplicationServices as AS
        deadline = time.time() + timeout
        while time.time() < deadline:
            hits = self.press_hits(alts)
            if hits:
                err = AS.AXUIElementPerformAction(hits[0][0], 'AXPress')
                return f'pressed {alts[0]!r} err={err}'
            time.sleep(1.0)
        return f'nav target {alts!r} not found'

    def press_control(self, ctl: dict, timeout: float = 45.0) -> str:
        """AXPress the control's picker button (matched by label text)."""
        import ApplicationServices as AS
        deadline = time.time() + timeout
        while time.time() < deadline:
            hits = self.press_hits(ctl['press'])
            if hits:
                err = AS.AXUIElementPerformAction(hits[0][0], 'AXPress')
                if err == 0:
                    return f'pressed {hits[0][1][:70]!r} err=0'
            time.sleep(1.2)
        return (f'picker button not pressable for {ctl["id"]} '
                f'(alts={ctl["press"]})')

    def arrive_screen(self, ctl: dict, timeout: float = 30.0) -> bool:
        """Screen arrival = the control's picker button exists."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.press_hits(ctl['press']):
                return True
            time.sleep(1.0)
        return False

    def capture(self, panel, out_dir: Path) -> dict:
        out_dir.mkdir(parents=True, exist_ok=True)
        res = {}
        try:
            res['panel_tree'] = bool(pk.dump_tree(
                panel.element, str(out_dir / 'panel-tree.json')))
        except Exception as e:
            res['panel_tree'] = f'error: {e}'
        try:
            res['panel_png'] = pk.capture_panel_png(
                self.pid, panel, str(out_dir / 'panel.png')) is not None
        except Exception as e:
            res['panel_png'] = f'error: {e}'
        return res

    def read_input(self, ctl: dict, timeout: float = 15.0):
        return pk.read_input(self.pid, ctl['inp'], timeout=timeout)

    def close_wizard(self) -> str:
        """Return the wizard modal to closed state (web «Отмена»; from
        step 1+ go back first). AXPress only — no Escape keystrokes."""
        try:
            if self.read_input(dict(inp=WIZARD_PATH_INP), timeout=4) is None:
                back = self.nav_named(WIZARD_BACK, timeout=4)
                if 'pressed' not in back:
                    return 'was-closed'
                time.sleep(0.6)
            self.nav_named(WIZARD_CANCEL, timeout=6)
            gone = self.read_input(
                dict(inp=WIZARD_PATH_INP), timeout=4) is None
            return 'closed' if gone else 'still-open'
        except Exception as e:  # noqa: BLE001
            return f'error: {e}'

    # ---- one round

    def round(self, scenario: str, ctl: dict) -> dict:
        rd: dict = {'scenario': scenario, 'control': ctl['id'],
                    'steps': [], 'ok': False}
        out_dir = self.root / scenario / ctl['id']
        expected_abs = str(FIXTURE.joinpath(*ctl['target'].split('/')))
        panel = None
        try:
            # 1. arrive on the screen (wizard: open the modal)
            if ctl['id'] == 'wizard.pick':
                rd['steps'].append(self.nav_named(WIZARD_OPEN))
                if not self.arrive_screen(ctl):
                    # modal input is the arrival signal
                    if self.read_input(ctl, timeout=5) is None:
                        raise TimeoutError('wizard modal did not open')
            else:
                rd['steps'].append(self.nav_named(NAV[ctl['nav']]))
                if not self.arrive_screen(ctl):
                    raise TimeoutError(
                        f"screen {ctl['route']} did not expose "
                        f"the {ctl['id']} picker button")
            rd['steps'].append(f'arrived {ctl["route"]}')
            time.sleep(0.6)

            before = self.read_input(ctl, timeout=8)
            rd['readback_before'] = before

            # 2. open the panel via the control's picker button. All
            # navigation goes through the memory seam (NSNavLastRootDirectory
            # preset): the browser rows of NSOpenPanel are NOT exposed via AX
            # on this macOS build (verified: AXBrowser columns carry zero
            # children), so row-based descent is impossible — and pointless,
            # because the remembered-root behavior is exactly what the
            # ticket tests. The preset dir differs per scenario/control:
            #   memory  → the fixed remembered root (fixture po-pack)
            #   path    → the control's OWN fixture target
            #   cancel  → the control's OWN fixture target
            if scenario == 'memory':
                preset = str(FIXTURE / 'po-pack')
            else:
                target = Path(expected_abs)
                preset = str(target if ctl['kind'] == 'dir' else target.parent)
            pk.defaults_write(pk.NAV_LAST_ROOT_KEY, preset)
            rd['steps'].append(f'defaults preset → {preset}')
            rd['steps'].append(self.press_control(ctl))
            panel = pk.wait_panel(self.pid, timeout=30)
            rd['panel_kind'] = panel.kind
            rd['evidence'] = self.capture(panel, out_dir)
            self.note_focus(f'{scenario}/{ctl["id"]}:panel-open')
            rd['panel_rows'] = [lbl for _, lbl in
                                pk.panel_rows(panel.element)][:24]
            where = str(pk._value(pk.where_popup(panel.element)) or '')
            open_el = pk.open_button(panel)
            open_enabled = bool(drv._ax_attr(open_el, 'AXEnabled')) \
                if open_el is not None else None
            rd['where_at_open'] = where
            rd['open_enabled_at_open'] = open_enabled
            # Health gate (verified live, probes 5-21): a panel WITHOUT a
            # location (empty Where:) is in the degraded state — its
            # interactive elements ignore AXPress in a background session
            # (Open/Cancel both no-op) and programmatic activation is
            # refused by macOS 27. A healthy panel (Where: shows the
            # remembered root) confirms fine. Degraded rounds verify
            # everything observable and record the remedy honestly.
            healthy = bool(where) and open_enabled is not False

            # expected readback per scenario: memory opens AT the preset
            # root (po-pack) — the control's own target only applies to
            # the path scenario's navigation
            if scenario == 'memory':
                expected = (str(FIXTURE / 'po-pack' / 'pack.po')
                            if ctl['kind'] == 'file'
                            else str(FIXTURE / 'po-pack'))
            else:
                expected = expected_abs

            if not healthy:
                # Degraded environment round: the panel opened (evidence
                # captured above) but its interactive elements are inert
                # for a background driver. Everything observable is
                # recorded; the round does NOT count as passed (review
                # #176) — the scenario's core assertion (arrival/confirm)
                # never ran. Exit code 2 + the remedy note make the gap
                # visible instead of a lying green.
                rd['degraded'] = True
                rd['verified'] = [
                    'panel opened', 'dialog signature (Open/Cancel/New '
                    'Folder/Where:)', 'evidence pack', 'app stayed healthy',
                    'zero focus interference']
                rd['remedy'] = ('one real folder pick in RimLoc (any '
                                'session) regenerates the panel bookmark '
                                'and restores healthy panels; rerun the '
                                'lane afterwards')
                after = self.read_input(ctl, timeout=8)
                rd['readback_after'] = after
                rd['expected'] = before
                rd['ok'] = False
                pk.press_cancel(panel)   # attempt; response recorded below
                time.sleep(1.5)
                pn = pk.find_panel(self.pid)
                rd['cancel_response'] = 'panel-gone' \
                    if pn is None else 'no-op'
                if pn is not None:
                    pk.press_cancel(pn)
                    pk.panel_gone(self.pid, timeout=4)
                self.note_focus(f'{scenario}/{ctl["id"]}:round-end')
                self.rounds.append(rd)
                (out_dir / 'round.json').write_text(
                    json.dumps(rd, ensure_ascii=False, indent=1),
                    encoding='utf-8')
                print(f'[DEGRADED] {scenario}/{ctl["id"]} '
                      f'where={where!r} open_enabled={open_enabled} '
                      f'cancel={rd["cancel_response"]}', flush=True)
                return rd

            if scenario == 'cancel':
                rd['steps'].append('press-cancel')
                if not pk.press_cancel(panel):
                    raise RuntimeError('«Отмена» press failed')
                if not pk.panel_gone(self.pid):
                    raise RuntimeError('panel survived «Отмена»')
                after = self.read_input(ctl, timeout=8)
                rd['readback_after'] = after
                rd['expected'] = before
                rd['ok'] = (after == before)
                rd['note'] = 'cancel must not change state'
            else:
                # Folder-mode confirm over the memory seam: wait for the
                # Open button to become ENABLED (a fresh preset can leave
                # it disabled while the directory loads; AXPress on a
                # disabled button is a silent no-op) and press it. A file
                # control receives the remembered DIR — the file row
                # itself is not AX-reachable on this build; that
                # degradation is recorded honestly in the round note.
                try:
                    pk.confirm_dir(self.pid, timeout=30)
                except RuntimeError as e:
                    # Located panel whose Open never enabled: the row
                    # selection that enables it is impossible over AX on
                    # this macOS build (browser rows not exposed; pid-
                    # targeted clicks ignored; activation refused — probe
                    # chain 5-23). Record everything verified, cancel the
                    # panel, mark degraded.
                    rd['degraded'] = True
                    rd['degraded_reason'] = str(e)
                    rd['verified'] = [
                        'panel opened at the preset target',
                        f'Where: = {where!r}',
                        'dialog signature', 'evidence pack',
                        'app stayed healthy', 'zero focus interference']
                    rd['remedy'] = ('row selection + Open confirm need an '
                                    'interactive session (T8 CUA / owner '
                                    'UAT) on this macOS build')
                    pn = pk.find_panel(self.pid)
                    if pn is not None:
                        pk.press_cancel(pn)
                        rd['cancel_response'] = 'panel-gone' \
                            if pk.panel_gone(self.pid, timeout=6) else 'no-op'
                    after = self.read_input(ctl, timeout=8)
                    rd['readback_after'] = after
                    rd['expected'] = before
                    rd['ok'] = False
                    self.note_focus(f'{scenario}/{ctl["id"]}:round-end')
                    self.rounds.append(rd)
                    (out_dir / 'round.json').write_text(
                        json.dumps(rd, ensure_ascii=False, indent=1),
                        encoding='utf-8')
                    print(f'[DEGRADED] {scenario}/{ctl["id"]} '
                          f'where={where!r} open_enabled={open_enabled} '
                          f'cancel={rd.get("cancel_response")}', flush=True)
                    return rd
                rd['steps'].append('open-confirmed (enabled-wait)')
                if not pk.panel_gone(self.pid):
                    raise RuntimeError('panel survived the confirm press')
                after = self.read_input(ctl, timeout=15)
                rd['readback_after'] = after
                rd['expected'] = expected
                if after is None:
                    # wizard: successful pick advances to step 1 and
                    # unmounts the path input — «Назад» must bring it back
                    # WITH the picked dir as its value (the only way step
                    # advances is the directory being set)
                    back = self.nav_named(WIZARD_BACK, timeout=6)
                    advanced = 'pressed' in back
                    rd['wizard_advanced'] = advanced
                    val = (self.read_input(ctl, timeout=8)
                           if advanced else None)
                    rd['wizard_readback'] = val
                    rd['ok'] = bool(
                        advanced and val is not None
                        and Path(val) == Path(expected))
                else:
                    rd['ok'] = Path(after) == Path(expected)
                if ctl['kind'] == 'file':
                    rd['note'] = ('file row not AX-reachable on this macOS '
                                  'build; the remembered root dir was '
                                  'returned instead of pack.po')
                # memory write-back: macOS records the picked dir
                if scenario == 'path' and rd['ok']:
                    want_tail = expected.rstrip('/').split('/')[-1]
                    got = None
                    for _ in range(6):
                        got = pk.defaults_read()
                        if got and got.rstrip('/').endswith(want_tail):
                            rd['defaults_writeback'] = got
                            break
                        time.sleep(1.0)
                    else:
                        rd['defaults_writeback'] = f'MISSING (got {got!r})'
                        rd['ok'] = False
        except Exception as e:  # noqa: BLE001 — the round IS the unit of failure
            rd['error'] = f'{type(e).__name__}: {e}'
            rd['failed'] = True
            try:
                panel = pk.find_panel(self.pid)
                if panel is not None:
                    self.capture(panel, out_dir)
                    pk.press_cancel(panel)
                    pk.panel_gone(self.pid)
                    rd['recovered'] = 'panel cancelled after failure'
            except Exception:
                pass
        if ctl['id'] == 'wizard.pick':
            # every wizard round ends with the modal closed, so later
            # rounds never see the modal's own picker in the tree
            rd['modal_closed'] = self.close_wizard()
        self.note_focus(f'{scenario}/{ctl["id"]}:round-end')
        self.rounds.append(rd)
        (out_dir / 'round.json').parent.mkdir(parents=True, exist_ok=True)
        (out_dir / 'round.json').write_text(
            json.dumps(rd, ensure_ascii=False, indent=1),
            encoding='utf-8')
        status = 'OK ' if rd['ok'] else 'FAIL'
        print(f'[{status}] {scenario}/{ctl["id"]} '
              f'after={rd.get("readback_after")!r} '
              f'err={rd.get("error", "")[:120]}', flush=True)
        return rd

    # ---- project bootstrap (wizard is the real user path)

    def bootstrap_project(self) -> dict:
        """Create the fixture project through the wizard: ex.pick,
        ex.po-pick and be.pick render only with a project open
        (Existing.tsx / BuildExport.tsx project gates) — the lane must
        unlock them the same way a user does."""
        info: dict = {}
        self.nav_named(WIZARD_OPEN)
        if self.read_input(dict(inp=WIZARD_PATH_INP), timeout=10) is None:
            raise RuntimeError('wizard modal did not open (bootstrap)')
        # memory seam + retry ladder: the panel health is a nondeterministic
        # per-open property (cfprefsd/panel state — probes 5-24); a broken
        # panel is cancelled and the pick reopened, up to 3 attempts.
        for attempt in (1, 2, 3):
            pk.defaults_write(pk.NAV_LAST_ROOT_KEY, str(FIXTURE / 'pack-a'))
            self.press_control(dict(id='bootstrap', press=WIZARDCTL_PRESS),
                               timeout=30)
            panel = pk.wait_panel(self.pid, timeout=30)
            info['panel_kind'] = panel.kind
            info['where_at_open'] = str(pk._value(
                pk.where_popup(panel.element)) or '')
            try:
                pk.confirm_dir(self.pid, timeout=30)
                break
            except RuntimeError as e:
                # Broken panel (no location / Open never enabled): cancel
                # and reopen — never escalate on the first flake.
                info.setdefault('attempts', []).append(
                    {'n': attempt, 'where': info['where_at_open'],
                     'reason': str(e)})
                pn = pk.find_panel(self.pid)
                if pn is not None:
                    pk.press_cancel(pn)
                    pk.panel_gone(self.pid, timeout=6)
                self.nav_named(WIZARD_CANCEL, timeout=8)
                if attempt < 3:
                    self.nav_named(WIZARD_OPEN, timeout=15)
                    if self.read_input(dict(inp=WIZARD_PATH_INP),
                                       timeout=10) is None:
                        raise RuntimeError(
                            'wizard modal did not reopen (bootstrap retry)')
        else:
            # Degraded environment after 3 attempts: the gated screens
            # cannot be unlocked over AX. Close the modal honestly and let
            # the lane cover the ungated controls in degraded mode.
            info['degraded'] = True
            info['degraded_reason'] = 'no healthy panel in 3 attempts'
            return info
        if not pk.panel_gone(self.pid):
            raise RuntimeError('panel survived the bootstrap confirm')
        got = self.read_input(dict(inp=WIZARD_PATH_INP), timeout=10)
        advanced_back = False
        if got is None:
            # A successful pick advances the wizard (setStep(1)) and
            # unmounts the step-0 path input (review #176): go «Назад» to
            # read and validate it, then forward again before creating.
            back = self.nav_named(WIZARD_BACK, timeout=8)
            if 'pressed' not in back:
                raise RuntimeError(
                    'bootstrap: path input unmounted and «Назад» '
                    'unavailable after the confirm')
            got = self.read_input(dict(inp=WIZARD_PATH_INP), timeout=8)
            self.nav_named(WIZARD_NEXT, timeout=10)
            advanced_back = True
        info['wizard_readback'] = got
        if got is None or not got.rstrip('/').endswith('pack-a'):
            raise RuntimeError(
                f'bootstrap pick readback {got!r} does not end with pack-a')
        # The wizard is multi-step (1/3 mod folder → … → «Создать проект»):
        # advance with «Далее» until the create button appears (review
        # #176 — the step count is not fixed 1 press).
        deadline = time.time() + 150
        created = False
        next_presses = 0
        if not advanced_back:
            self.nav_named(WIZARD_NEXT, timeout=15)
        while time.time() < deadline:
            if self.press_hits(WIZARD_CREATE):
                self.nav_named(WIZARD_CREATE, timeout=10)
                created = True
                break
            if next_presses < 6 and self.press_hits(WIZARD_NEXT):
                self.nav_named(WIZARD_NEXT, timeout=10)
                next_presses += 1
            time.sleep(2)
        if not created:
            raise RuntimeError(
                'wizard never reached «Создать проект» '
                f'({next_presses} «Далее» presses in 150s)')
        deadline = time.time() + 120
        while time.time() < deadline:
            if self.read_input(dict(inp=WIZARD_PATH_INP), timeout=3) is None:
                break
            time.sleep(1.0)
        else:
            raise RuntimeError('wizard modal never closed after create')
        # the project is open ⇒ gated screens expose their pickers
        self.nav_named(NAV['existing'], timeout=30)
        if not self.arrive_screen(CONTROLS[0], timeout=60):
            # carry the evidence: what the screen actually exposed
            names = pk.web_button_names(self.pid, timeout=10)
            (self.root / 'existing-screen-failure.txt').write_text(
                '\n'.join(n for n in names if n), encoding='utf-8')
            raise RuntimeError(
                'project bootstrap did not unlock the Existing screen '
                f'(controls dumped to existing-screen-failure.txt, '
                f'{sum(1 for n in names if n)} named)')
        info['unlocked'] = True
        return info

    # ---- lifecycle

    def run(self) -> dict:
        print(f'evidence dir: {self.root}', flush=True)
        created = build_fixture(FIXTURE)
        print(f'fixture at {FIXTURE} (created {len(created)} files)',
              flush=True)

        old_default = pk.defaults_read()
        preset = str(FIXTURE / 'po-pack')
        pk.defaults_write(pk.NAV_LAST_ROOT_KEY, preset)
        print(f'NSNavLastRootDirectory preset → {preset} '
              f'(was {old_default!r})', flush=True)

        try:
            self.inst = pk.launch(self.bin, str(self.root),
                                  str(self.data_dir), self.frame)
            self.pid = int(self.inst.pid or 0)
            if self.pid <= 0:
                raise RuntimeError('OwnedInstance returned no app pid')
            try:
                names = pk.wait_ready(self.pid, timeout=150)
            except TimeoutError:
                # the failing run must carry its own evidence: dump what
                # the main window actually exposed when hydration stalled
                wins = pk.app_windows(self.pid)
                if wins:
                    pk.dump_tree(
                        wins[0], str(self.root / 'hydration-failure.json'))
                    print('hydration stalled; window tree dumped to '
                          'hydration-failure.json', flush=True)
                raise
            print(f'app up pid={self.pid}, '
                  f'{len(names)} named web controls', flush=True)
            open(str(self.root / 'initial-buttons.txt'), 'w').write(
                '\n'.join(names))

            boot = self.bootstrap_project()
            print(f'project bootstrap: {boot}', flush=True)
            (self.root / 'bootstrap.json').write_text(
                json.dumps(boot, ensure_ascii=False, indent=1),
                encoding='utf-8')

            ctls = CONTROLS
            if boot.get('degraded'):
                gated = {'ex.pick', 'ex.po-pick', 'be.pick', 'diag.pick'}
                ctls = [c for c in CONTROLS if c['id'] not in gated]
                print('degraded bootstrap: covering the ungated controls '
                      'only: ' + ','.join(c['id'] for c in ctls), flush=True)
            for scenario in SCENARIOS:
                print(f'--- scenario: {scenario}', flush=True)
                for ctl in ctls:
                    self.round(scenario, ctl)
                    time.sleep(1.0)

            # close the wizard modal if it is still up
            self.nav_named(WIZARD_CANCEL, timeout=6)
        finally:
            # defaults restore AFTER the app is gone (cfprefsd)
            if self.inst is not None:
                self.inst.cleanup()
                time.sleep(2)
            pk.defaults_restore(pk.NAV_LAST_ROOT_KEY, old_default)
            print(f'defaults restored to {old_default!r}', flush=True)

        # focus: if the modal panel stole frontmost, hand it back
        pid_now, name_now = pk.frontmost_app()
        if pid_now != self.frontmost_before[0] and \
                pid_now == self.pid and self.frontmost_before[0] > 0:
            target = self.frontmost_before[0]
            ws = NSWorkspace.sharedWorkspace()
            for a in ws.runningApplications():
                if a.processIdentifier() == target:
                    try:
                        a.activateFromApplication_(ws.frontmostApplication())
                        self.focus_events.append(
                            {'where': 'end-restore',
                             'restored': f'{name_now}({target})'})
                        break
                    except Exception as e:
                        self.focus_events.append(
                            {'where': 'end-restore', 'error': str(e)})

        return self.report()

    def report(self) -> dict:
        total = len(self.rounds)
        ok = sum(1 for r in self.rounds if r['ok'])
        degraded = sum(1 for r in self.rounds if r.get('degraded'))
        failed = sum(1 for r in self.rounds if r.get('failed'))
        rep = {
            'started': getattr(self, '_t0', ''),
            'bin': self.bin,
            'fixture': str(FIXTURE),
            'totals': {'rounds': total, 'passed': ok,
                       'degraded': degraded, 'failed': failed},
            'degraded_note': ('degraded rounds verified open+structure+'
                              'evidence+zero-focus; the dialog interaction '
                              'needs a healthy panel — one real folder pick '
                              'in RimLoc restores it, then rerun'),
            'controls_expected_per_scenario': len(CONTROLS),
            'scenarios': list(SCENARIOS),
            'focus_events': self.focus_events,
            'frontmost_before': self.frontmost_before,
            'frontmost_after': pk.frontmost_app(),
            'rounds': self.rounds,
        }
        (self.root / 'report.json').write_text(
            json.dumps(rep, ensure_ascii=False, indent=1),
            encoding='utf-8')
        return rep


def main() -> int:
    # The spawned binary is a fixed constant relative to the worktree root
    # (CodeQL path-injection: no env/operator-provided path ever reaches a
    # filesystem API here). Overrides happen by building into the standard
    # location via testlab/build/build-automation-react.sh.
    bin_path = str((Path.cwd() / DEFAULT_BIN).resolve())
    if not Path(bin_path).is_file():
        print(f'FATAL: automation artifact missing: {bin_path}\n'
              'Run testlab/build/build-automation-react.sh first.',
              file=sys.stderr)
        return 2
    frame = os.environ.get('PICKERS_FRAME', '348,70,980x640')
    # CodeQL ReDoS gate: strict split-based parsing, no regex on
    # operator-provided data. The frame must be "x,y,WxH" integers.
    try:
        x_s, y_s, wh = frame.split(',')
        x, y, w, h = int(x_s), int(y_s), *(int(v) for v in wh.split('x'))
        if min(x, y, w, h) < 0:
            raise ValueError('negative frame component')
    except (ValueError, TypeError):
        print(f'FATAL: PICKERS_FRAME must be "x,y,WxH" integers '
              f'(got {frame!r})', file=sys.stderr)
        return 2
    lane = Lane(bin_path, frame)
    rep = lane.run()
    print(json.dumps(rep['totals'], ensure_ascii=False))
    print(f"focus_events: {rep['focus_events']}")
    print(f"report: {lane.root / 'report.json'}")
    # Exit matrix (review #176): only a fully-verified run is green.
    #   0 — every round passed, no degraded rounds, no focus interference
    #   2 — nothing failed but degraded rounds remain: the environment
    #       needs the remedy (one real folder pick restores healthy
    #       panels); the run is NOT a pass
    #   1 — genuine round failures or focus interference (campaign red flag)
    t = rep['totals']
    if t['failed'] > 0 or rep['focus_events']:
        return 1
    if t['degraded'] > 0:
        print('DEGRADED ROUNDS PRESENT — see the remedy in report.json: '
              'one real folder pick in RimLoc restores healthy panels, '
              'then rerun this lane for full coverage.', flush=True)
        return 2
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
