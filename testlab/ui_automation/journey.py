"""Declarative journey runner: launch the app off-screen, execute YAML steps,
collect screenshots and a no-disturb report.

Step types (YAML):
  - shot: <name>                    capture the window frame to shots_dir
  - press: <name>                   AXPress a web button by accessible name
      {press: {name: X, timeout: 60, exact: false}}
  - wait_text_rows: <min>           poll until the frame has >= min text rows
      {wait_text_rows: {min: 12, timeout: 20}}
  - sleep: <seconds>
  - assert_offscreen: true          re-check the window is parked off-screen
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from dataclasses import dataclass, field

import yaml

from . import driver
from .lifecycle import OwnedInstance, find_reusable


@dataclass
class StepResult:
    step: dict
    ok: bool
    detail: str = ''


@dataclass
class JourneyReport:
    config_path: str
    steps: list[StepResult] = field(default_factory=list)
    disturb: dict | None = None
    ok: bool = False

    def summary(self) -> str:
        lines = [f'journey {"OK" if self.ok else "FAILED"}: {self.config_path}']
        for s in self.steps:
            mark = '+' if s.ok else '!'
            lines.append(f'  [{mark}] {s.step} {s.detail}')
        if self.disturb:
            lines.append(f'  disturb: {self.disturb}')
        return '\n'.join(lines)


def _run_steps(cfg: dict, pid: int, win: driver.Window,
               log, work_dir: str) -> list[StepResult]:
    shots_dir = cfg.get('shots_dir', '/tmp/ui-automation-shots')
    results: list[StepResult] = []
    last_img = None
    for raw in cfg.get('steps', []):
        step = raw if isinstance(raw, dict) else {'raw': raw}
        ok, detail = True, ''
        try:
            if 'shot' in step:
                img = driver.capture_window(win, str(step['shot']), shots_dir)
                if img is None:
                    ok, detail = False, 'capture failed'
                else:
                    last_img = img
                    detail = (f'{img.size} bg={driver.dominant_bg(img)} '
                              f'nonblank={driver.nonblank_samples(img)}')
            elif 'press' in step:
                spec = step['press']
                if isinstance(spec, dict):
                    res = driver.ax_press(
                        pid, str(spec['name']),
                        timeout=float(spec.get('timeout', 60)),
                        exact=bool(spec.get('exact', False)))
                else:
                    res = driver.ax_press(pid, str(spec))
                ok = 'not found' not in res
                detail = res
            elif 'wait_text_rows' in step:
                spec = step['wait_text_rows']
                min_rows, timeout = (float(spec), 20.0) if isinstance(spec, (int, float)) \
                    else (float(spec.get('min', 1)), float(spec.get('timeout', 20)))
                deadline = time.time() + timeout
                rows = -1
                while time.time() < deadline:
                    img = driver.capture_window(win, 'wait-frame.png', work_dir)
                    if img is not None:
                        rows = driver.text_rows(img)
                        if rows >= min_rows:
                            last_img = img
                            break
                    time.sleep(1.5)
                ok = rows >= min_rows
                detail = f'text_rows={rows} (min={min_rows:.0f})'
            elif 'sleep' in step:
                time.sleep(float(step['sleep']))
            elif 'assert_offscreen' in step and step['assert_offscreen']:
                fresh = driver.find_main_window(pid)
                driver.assert_offscreen(fresh)
                win = fresh
                detail = f'bounds={fresh.bounds}'
            else:
                ok, detail = False, f'unknown step: {step}'
        except Exception as exc:  # noqa: BLE001 - report and stop the journey
            ok, detail = False, f'exception: {exc!r}'
        results.append(StepResult(step=step, ok=ok, detail=detail))
        log(f'step {step} -> {"ok" if ok else "FAIL"} {detail}')
        if not ok:
            break
    return results


def run(config_path: str, log=print) -> JourneyReport:
    with open(config_path, encoding='utf-8') as f:
        cfg = yaml.safe_load(f)
    app = cfg['app']
    shots_dir = cfg.get('shots_dir', '/tmp/ui-automation-shots')
    work_dir = cfg.get('work_dir', '/tmp/ui-automation-work')
    os.makedirs(shots_dir, exist_ok=True)
    os.makedirs(work_dir, exist_ok=True)

    report = JourneyReport(config_path=config_path)
    binary = app['binary']
    env = driver.offscreen_env(
        origin=tuple(app.get('origin', (-3000, -3000))),
        move_mode=app.get('move_mode', 'swizzle'),
        fake=app.get('fake', 'occlusion,firstmouse'))

    # lifecycle: reuse a live instance from the pid file, else spawn our own
    # (guaranteed cleanup kills only the pid this run spawned).
    inst = OwnedInstance(pid_file=cfg.get('pid_file'))
    pid_file = cfg.get('pid_file')
    if pid_file and not cfg.get('fresh_instance', False):
        reusable = find_reusable(pid_file, binary)
        if reusable:
            inst.attach(reusable)
            log(f'reusing live instance pid={reusable} from {pid_file}')
    if inst.pid is None:
        inst.spawn([binary], env)
        log(f'launched pid={inst.pid} origin={app.get("origin")} '
            f'mode={app.get("move_mode")} fake={app.get("fake")}')
    pid = inst.pid
    try:
        max_relaunch = int(cfg.get('max_relaunch', 2))
        watch = None
        for attempt in range(max_relaunch + 1):
            win = driver.wait_main_window(
                pid, timeout=float(app.get('window_timeout', 30)))
            # the initial move is async on the app's main queue — poll for it
            deadline = time.time() + 15
            while time.time() < deadline:
                try:
                    driver.assert_offscreen(win)
                    break
                except AssertionError:
                    time.sleep(1)
                    win = driver.find_main_window(pid) or win
            driver.assert_offscreen(win)
            log(f'window {win.window_id} bounds={win.bounds}')
            time.sleep(float(app.get('settle_s', 9)))

            watch = driver.DisturbWatch(work_dir)
            report.steps = _run_steps(cfg, pid, win, log, work_dir)
            if report.steps and all(s.ok for s in report.steps):
                break
            if attempt < max_relaunch:
                # self-heal: the app-side AX bridge hydrates lazily and some
                # instances never hydrate at all; a fresh instance fixes it
                log(f'attempt {attempt} failed — relaunching a fresh instance')
                inst.cleanup()
                inst = OwnedInstance(pid_file=inst.pid_file)
                inst.spawn([binary], env)
                pid = inst.pid
                log(f'relaunched pid={pid}')
        time.sleep(float(cfg.get('telemetry_settle_s', 1)))
        rep = watch.finish() if watch else None
        # refresh the window snapshot so the report shows the parked position
        fresh = driver.find_main_window(pid)
        if fresh:
            try:
                driver.assert_offscreen(fresh)
            except AssertionError:
                pass  # a failed journey may leave the window wherever it is
        report.disturb = rep.summary() if rep else {'error': 'no telemetry'}
        report.disturb['window_bounds_final'] = fresh.bounds if fresh else None
        report.ok = all(s.ok for s in report.steps) and bool(report.steps)
    finally:
        if inst.attached:
            log(f'leaving attached instance pid={pid} running')
        else:
            inst.cleanup()
            log(f'instance {pid} terminated')
    return report
