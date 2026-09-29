#!/usr/bin/env python3
"""RimLoc auto-install / run / verify — агентский полный автомат (макОС).

Устанавливает release-сборку RimLoc в /Applications БЕЗ DMG и Finder-диалогов:
rsync в staging-каталог + атомарная замена. Одна версия, обновляется поверх.
Заменяет ручной путь владельца (двойной клик DMG → диалог «Установить?»).

Подкоманды:
  status                 — что установлено, откуда, SHA бинаря, запущено ли
  install <app-bundle>   — атомарная установка/обновление из .app-бандля
                           (аргумент — путь к "RimLoc GUI.app" из сборки)
  run [--args ...]       — запуск установленной копии (open -a)
  launch-bin             — запуск прямым бинарем из установленного bundle
                           (для live-прогонов агента: stderr в лог)
  stop                   — завершить запущенные процессы RimLoc
  uninstall              — удалить в корзину AI-OS (.trash/YYYY-MM-DD/)

Выход: 0 — ок, 1 — ошибка с описанием в stderr. Никогда не трогает
RimWorld, Workshop и пользовательские данные.
"""
from __future__ import annotations

import hashlib
import os
import pathlib
import shutil
import subprocess
import sys
import time

APP_NAME = "RimLoc GUI.app"
DEST = pathlib.Path("/Applications") / APP_NAME
LOG_DIR = pathlib.Path("/tmp/rimloc-autotest")


def sha256(path: pathlib.Path, chunk: int = 1 << 20) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while block := f.read(chunk):
            h.update(block)
    return h.hexdigest()


def running_pids() -> list[str]:
    out = subprocess.run(
        ["pgrep", "-f", "rimloc-gui"], capture_output=True, text=True
    ).stdout.split()
    return out


def cmd_status() -> int:
    print(f"dest: {DEST}")
    if not DEST.exists():
        print("installed: no")
        return 0
    b = DEST / "Contents/MacOS/rimloc-gui"
    print(f"installed: yes  bin_sha256: {sha256(b)[:16]}…  size: {shutil.disk_usage(DEST).free and _size(DEST) >> 20} MB")
    info = (
        subprocess.run(
            ["defaults", "read", str(DEST / "Contents/Info.plist"), "CFBundleShortVersionString"],
            capture_output=True, text=True,
        ).stdout.strip()
        or "?"
    )
    print(f"version: {info}")
    pids = running_pids()
    print(f"running: {'yes pids=' + ','.join(pids) if pids else 'no'}")
    return 0


def _size(p: pathlib.Path) -> int:
    return sum(f.stat().st_size for f in p.rglob("*") if f.is_file())


def cmd_install(src_str: str) -> int:
    src = pathlib.Path(src_str).expanduser().resolve()
    if not src.is_dir() or src.name != APP_NAME:
        print(f"error: {src} не является {APP_NAME}", file=sys.stderr)
        return 1
    if not (src / "Contents/MacOS/rimloc-gui").exists():
        print(f"error: в {src} нет бинаря rimloc-gui", file=sys.stderr)
        return 1

    # Работающий процесс держит старый bundle — завершаем.
    pids = running_pids()
    if pids:
        for pid in pids:
            subprocess.run(["kill", pid], capture_output=True)
        for _ in range(50):
            if not running_pids():
                break
            time.sleep(0.1)

    staging = DEST.with_name(f".{APP_NAME}.staging")
    if staging.exists():
        shutil.rmtree(staging)
    print(f"copy: {src} -> {staging}")
    subprocess.run(
        ["rsync", "-a", "--delete", str(src) + "/", str(staging) + "/"],
        check=True,
    )
    # Локальная сборка quarantine не несёт, но снимаем на всякий случай,
    # чтобы Gatekeeper молчал при первом запуске.
    subprocess.run(
        ["xattr", "-dr", "com.apple.quarantine", str(staging)],
        capture_output=True,
    )
    if DEST.exists():
        old = DEST.with_name(f".{APP_NAME}.old")
        if old.exists():
            shutil.rmtree(old)
        DEST.rename(old)
    staging.rename(DEST)
    if DEST.with_name(f".{APP_NAME}.old").exists():
        shutil.rmtree(DEST.with_name(f".{APP_NAME}.old"))
    print(f"installed: {DEST}  bin_sha256: {sha256(DEST / 'Contents/MacOS/rimloc-gui')[:16]}…")
    return 0


def cmd_run(args: list[str]) -> int:
    if not DEST.exists():
        print("error: не установлено (сначала install)", file=sys.stderr)
        return 1
    LOG_DIR.mkdir(exist_ok=True)
    log = LOG_DIR / "run.log"
    with open(log, "ab") as lf:
        subprocess.run(["open", "-a", str(DEST), "--args", *args], check=True)
    print(f"launched via open, log: {log}")
    return 0


def cmd_launch_bin() -> int:
    if not DEST.exists():
        print("error: не установлено", file=sys.stderr)
        return 1
    LOG_DIR.mkdir(exist_ok=True)
    log = LOG_DIR / "launch-bin.log"
    import datetime

    stamp = datetime.datetime.now().strftime("%H:%M:%S")
    proc = subprocess.Popen(
        [str(DEST / "Contents/MacOS/rimloc-gui")],
        stdout=open(log, "ab"),
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    print(f"launched pid={proc.pid} at {stamp}, log: {log}")
    return 0


def cmd_stop() -> int:
    pids = running_pids()
    for pid in pids:
        subprocess.run(["kill", pid], capture_output=True)
    print("stopped: " + (",".join(pids) if pids else "nothing to stop"))
    return 0


def cmd_uninstall() -> int:
    if not DEST.exists():
        print("nothing to uninstall")
        return 0
    cmd_stop()
    import datetime

    trash = (
        pathlib.Path.home()
        / "AI-OS/.trash"
        / datetime.date.today().isoformat()
        / "Applications"
    )
    trash.mkdir(parents=True, exist_ok=True)
    target = trash / APP_NAME
    n = 1
    while target.exists():
        target = trash / f"{APP_NAME}.{n}"
        n += 1
    DEST.rename(target)
    print(f"moved to trash: {target}")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    cmd, *rest = sys.argv[1:]
    if cmd == "status":
        return cmd_status()
    if cmd == "install":
        if not rest:
            print("error: install <путь к .app>", file=sys.stderr)
            return 1
        return cmd_install(rest[0])
    if cmd == "run":
        return cmd_run(rest)
    if cmd == "launch-bin":
        return cmd_launch_bin()
    if cmd == "stop":
        return cmd_stop()
    if cmd == "uninstall":
        return cmd_uninstall()
    print(f"error: неизвестная подкоманда {cmd}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
