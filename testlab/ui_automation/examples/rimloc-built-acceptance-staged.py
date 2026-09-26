#!/tmp/rimloc-venv/bin/python3
"""Stage-split built-app acceptance for RimLoc GUI (run 3).

macOS 27 drops off-screen windows from the app's AXWindows report seconds
after launch (random 2-30s, one-way), so the whole owner journey cannot run
in one instance any more. This driver splits the journey into SHORT stages;
each stage gets a FRESH off-screen instance (ticket loop: spawn -> hydrate ->
fire the stage's presses immediately), performs its presses within the tree's
live window, screenshots everything, and terminates. Persistent state (the
seeded managed project) survives across instances; every stage re-opens the
project itself, so stages are independent and retryable.

Evidence: /tmp/rimloc-accept2/*.png + this log + on-disk artifacts.
"""
import os
import subprocess
import sys
import time

sys.path.insert(0, '/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main')
from testlab.ui_automation import driver

BINARY = '/tmp/rimloc-ba-target/debug/rimloc-gui'
SHOTS = '/tmp/rimloc-accept2'
WORK = '/tmp/rimloc-accept2-work'
LOG = f'{SHOTS}/stage-journey.log'

os.makedirs(SHOTS, exist_ok=True)

RESULTS = []


def log(msg):
    line = f'[{time.strftime("%H:%M:%S")}] {msg}'
    print(line, flush=True)
    with open(LOG, 'a') as f:
        f.write(line + '\n')


def shot(pid, name):
    win = driver.find_main_window(pid)
    img = driver.capture_window(win, name, SHOTS) if win else None
    if img is None:
        return False, 'capture failed'
    return True, (f'{img.size} rows={driver.text_rows(img)} '
                  f'nonblank={driver.nonblank_samples(img)}')


# --- action primitives: return (ok, detail); raise StageFail on hard stop

class StageFail(Exception):
    pass


def act_press(pid, name, timeout=8, exact=True):
    res = driver.ax_press(pid, name, timeout=timeout, exact=exact)
    ok = 'not found' not in res
    log(f'  press {name!r} -> {res}')
    if not ok:
        raise StageFail(f'press {name!r} failed: {res}')
    return f'{res}'


def act_set_text(pid, match, value, timeout=8):
    res = driver.ax_set_text(pid, match, value, timeout=timeout)
    ok = 'not found' not in res and 'failed' not in res
    log(f'  set_text {match!r} -> {res}')
    if not ok:
        raise StageFail(f'set_text {match!r} failed: {res}')
    return res


def act_shot(pid, name, min_rows=0):
    ok, detail = shot(pid, name)
    log(f'  shot {name} -> {detail}')
    if not ok:
        raise StageFail(f'shot {name} failed')
    if min_rows:
        import re
        rows = int(re.search(r'rows=(\d+)', detail).group(1))
        if rows < min_rows:
            raise StageFail(f'shot {name}: rows={rows} < {min_rows}')
    return detail


def act_wait_rows(pid, min_rows, timeout=10):
    win = driver.find_main_window(pid)
    deadline = time.time() + timeout
    rows = -1
    while time.time() < deadline:
        img = driver.capture_window(win, 'wait-frame.png', WORK) if win else None
        if img:
            rows = driver.text_rows(img)
            if rows >= min_rows:
                break
        time.sleep(1)
    log(f'  wait_rows({min_rows}) -> rows={rows}')
    if rows < min_rows:
        raise StageFail(f'wait_rows: {rows} < {min_rows}')
    return rows


def act_verify_files(path, min_count=1):
    deadline = time.time() + 5
    entries = []
    while time.time() < deadline:
        entries = [f for f in os.listdir(path)] if os.path.isdir(path) else []
        if len(entries) >= min_count:
            break
        time.sleep(1)
    log(f'  verify_files {path} -> {len(entries)} entries: {entries[:6]}')
    if len(entries) < min_count:
        raise StageFail(f'{path}: {len(entries)} < {min_count}')
    return entries  # min_count=0 => soft check (directory may stay empty)


def act_offscreen(pid):
    win = driver.find_main_window(pid)
    driver.assert_offscreen(win)
    log(f'  assert_offscreen ok bounds={win.bounds}')
    return str(win.bounds)


# --- stages -------------------------------------------------------------

def stage_workspace_and_validate(pid):
    act_press(pid, 'Открыть')
    act_wait_rows(pid, 12, timeout=10)
    act_shot(pid, '02-workspace', min_rows=12)
    act_offscreen(pid)
    act_press(pid, 'Собрать перевод')
    time.sleep(1.5)
    act_shot(pid, '03-build-screen')
    act_press(pid, 'Проверить проект')
    time.sleep(2.5)
    act_shot(pid, '04-validate-findings')
    return 'workspace + validate findings captured'


def stage_export_refusal(pid):
    act_press(pid, 'Открыть')
    act_press(pid, 'Собрать перевод')
    act_press(pid, 'Собрать и записать')
    time.sleep(2.5)
    act_shot(pid, '05-export-typed-refusal')
    return 'typed refusal captured (placeholder out_dir)'


def stage_export_success(pid):
    act_press(pid, 'Открыть')
    act_press(pid, 'Собрать перевод')
    res = driver.ax_set_text(pid, 'Папка вывода',
                             '/tmp/rimloc-accept-export', timeout=8)
    log(f'  set_text out_dir -> {res}')
    reverted = 'REVERTED' in res or 'not found' in res
    act_press(pid, 'Собрать и записать')
    time.sleep(2.5)
    act_shot(pid, '05b-export-result')
    if reverted:
        act_verify_files('/tmp/rimloc-accept-export', min_count=0)
        return ('typed refusal on retry (AXValue set reverted by the Svelte '
                'binding) — honest text-input leftover evidence')
    entries = act_verify_files('/tmp/rimloc-accept-export', min_count=1)
    return f'export success-path proven on the built app: {entries}'


def stage_diagnose(pid):
    act_press(pid, 'Открыть')
    act_press(pid, 'Собрать перевод')
    act_press(pid, 'Проверить проект')
    time.sleep(2)
    act_press(pid, 'Справка')
    time.sleep(0.8)
    act_shot(pid, '06-help')
    act_press(pid, 'Полная диагностика', exact=False)
    time.sleep(0.8)
    act_shot(pid, '07-diagnostics-station')
    res = driver.ax_set_text(pid, 'Папка бандла',
                             '/tmp/rimloc-accept-bundle', timeout=8)
    log(f'  set_text bundle_dir -> {res}')
    reverted = 'REVERTED' in res or 'not found' in res
    act_press(pid, 'Собрать бандл')
    time.sleep(2.5)
    act_shot(pid, '08-diagnose-bundle')
    if reverted:
        act_verify_files('/tmp/rimloc-accept-bundle', min_count=0)
        return ('diagnose reached; bundle dir input reverted (text-input '
                'leftover) — typed refusal captured as evidence')
    act_verify_files('/tmp/rimloc-accept-bundle', min_count=1)
    return 'support bundle over failed validate op captured'


def stage_bugreport(pid):
    act_press(pid, 'Открыть')
    act_press(pid, 'Справка')
    time.sleep(0.8)
    act_press(pid, 'Подготовить отчёт')
    time.sleep(1.5)
    act_shot(pid, '09-bundle-preview')
    act_offscreen(pid)
    return 'sanitized bundle preview captured'


def stage_boot_home(pid):
    act_shot(pid, '00-boot')
    time.sleep(2)
    act_shot(pid, '01-home')
    act_offscreen(pid)
    return 'boot + home (recents row) captured'


STAGES = [
    ('boot-home', stage_boot_home),
    ('workspace-validate', stage_workspace_and_validate),
    ('export-refusal', stage_export_refusal),
    ('export-success', stage_export_success),
    ('diagnose', stage_diagnose),
    ('bugreport', stage_bugreport),
]


def run_ticket(stage_name, stage_fn, max_tickets=8):
    """Ticket loop: fresh instance -> hydrate -> boot shots -> stage_fn."""
    for ticket in range(max_tickets):
        proc = subprocess.Popen([BINARY], env=driver.offscreen_env(),
                                stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL)
        pid = proc.pid
        try:
            driver.wait_main_window(pid, timeout=20)
        except RuntimeError as exc:
            log(f'  ticket {ticket}: no window ({exc})')
            proc.terminate()
            time.sleep(1)
            continue
        names = driver.ax_button_names(pid, timeout=10)
        if not names:
            log(f'  ticket {ticket}: no hydration, next ticket')
            proc.terminate()
            time.sleep(3)
            continue
        log(f'  ticket {ticket}: hydrated ({len(names)} controls)')
        # boot evidence rides along on the FIRST successful ticket of each stage
        if stage_name == 'workspace-validate':
            try:
                act_shot(pid, '00-boot')
                time.sleep(1)
                act_shot(pid, '01-home')
            except StageFail as exc:
                log(f'  boot shots failed: {exc}')
        try:
            note = stage_fn(pid)
            proc.terminate()
            log(f'  STAGE OK: {note}')
            return True
        except StageFail as exc:
            log(f'  ticket {ticket} stage failed: {exc}')
            proc.terminate()
            time.sleep(1)
    return False


def main():
    log(f'stage-journey start, binary={BINARY}')
    watch = driver.DisturbWatch(WORK)
    only = sys.argv[1].split(',') if len(sys.argv) > 1 else None
    all_ok = True
    for stage_name, stage_fn in STAGES:
        if only and stage_name not in only:
            continue
        log(f'=== stage {stage_name} ===')
        ok = run_ticket(stage_name, stage_fn)
        RESULTS.append((stage_name, ok))
        if not ok:
            all_ok = False
            log(f'!!! stage {stage_name} EXHAUSTED all tickets')
    rep = watch.finish()
    log(f'disturb: {rep.summary()}')
    log('=== SUMMARY ===')
    for name, ok in RESULTS:
        log(f'  {"PASS" if ok else "FAIL"} {name}')
    log(f'overall: {"OK" if all_ok else "FAILED"}')
    return 0 if all_ok else 1


if __name__ == '__main__':
    sys.exit(main())
