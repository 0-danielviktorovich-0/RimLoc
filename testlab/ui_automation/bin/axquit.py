#!/usr/bin/env python3
"""axquit <pid> — AX-press the close button of the app's main window.

Background shutdown path for window-state checks: the AX close action lets
Tauri exit gracefully so the window-state plugin persists the user frame
(SIGTERM skips the save). Zero activations, zero keystrokes — a pure
AX-value/action manipulation (owner P0 rule, 2026-10-01).

Retry ~6x300ms: the AX bridge flaps for background apps right after launch
(testlab/ui_automation/ax/README.md). Exit 0 on success, 1 on failure.
"""
import sys
import time

import ApplicationServices as AX
import Quartz


def _ax_attr(el, name):
    err, val = AX.AXUIElementCopyAttributeValue(el, name, None)
    return val if err == 0 else None


def quit_app(pid: int, timeout: float = 2.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        app = AX.AXUIElementCreateApplication(pid)
        wins = _ax_attr(app, "AXWindows") or []
        for win in wins:
            btn = _ax_attr(win, "AXCloseButton")
            if btn is None:
                continue
            err = AX.AXUIElementPerformAction(btn, "AXPress")
            if err == 0:
                return True
        time.sleep(0.3)
    return False


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: axquit <pid>", file=sys.stderr)
        return 2
    try:
        pid = int(sys.argv[1])
    except ValueError:
        print("usage: axquit <pid>", file=sys.stderr)
        return 2
    return 0 if quit_app(pid) else 1


if __name__ == "__main__":
    sys.exit(main())
