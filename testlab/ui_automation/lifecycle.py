"""Instance lifecycle: PID-file reuse and guaranteed cleanup of OWN pids.

A background-automation run must never leave orphan instances behind (a
killed agent used to skip cleanup steps and litter the machine). This module
owns exactly the instance IT spawned:

  - instance PID is recorded in a pid file (default /tmp/<name>.instance.pid);
  - a later run may REUSE a live instance recorded there instead of spawning
    a new one (the recorded command line must match the configured binary);
  - cleanup is registered via atexit AND SIGINT/SIGTERM handlers and kills
    ONLY the pid this run spawned — never a name-based pkill, which would
    take down the owner's own instances.
"""

from __future__ import annotations

import atexit
import os
import signal
import subprocess
import time


def _cmdline(pid: int) -> str:
    try:
        out = subprocess.run(['ps', '-p', str(pid), '-o', 'command='],
                             capture_output=True, text=True)
        return out.stdout.strip()
    except Exception:
        return ''


def pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except (ProcessLookupError, PermissionError):
        return False
    except OSError:
        return False


def find_reusable(pid_file: str, binary: str) -> int | None:
    """Return the recorded pid if it is alive AND runs `binary`."""
    try:
        with open(pid_file, encoding='utf-8') as f:
            pid = int(f.read().strip())
    except (OSError, ValueError):
        return None
    if not pid_alive(pid):
        return None
    if os.path.basename(binary) not in _cmdline(pid):
        return None
    return pid


class OwnedInstance:
    """A child process this run spawned; cleanup kills exactly this pid.

    `attach()` wraps an externally-reused pid: it is NOT owned, so cleanup
    leaves it running (it belonged to a previous run on purpose).
    """

    def __init__(self, pid_file: str | None = None):
        self.pid_file = pid_file
        self.pid: int | None = None
        self.proc: subprocess.Popen | None = None
        self.attached = False
        self._cleaned = False

    def spawn(self, cmd: list[str], env: dict) -> None:
        self.proc = subprocess.Popen(cmd, env=env,
                                     stdout=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL)
        self.pid = self.proc.pid
        self._register()

    def attach(self, pid: int) -> None:
        self.pid = pid
        self.attached = True
        self._register()

    def _register(self) -> None:
        if self.pid_file:
            with open(self.pid_file, 'w', encoding='utf-8') as f:
                f.write(f'{self.pid}\n')
        atexit.register(self.cleanup)
        for sig in (signal.SIGINT, signal.SIGTERM):
            try:
                prev = signal.getsignal(sig)

                def handler(signum, frame, _prev=prev, _self=self):
                    _self.cleanup()
                    if callable(_prev):
                        _prev(signum, frame)
                    else:
                        signal.signal(signum, signal.SIG_DFL)
                        os.kill(os.getpid(), signum)

                signal.signal(sig, handler)
            except (ValueError, OSError):
                pass  # not the main thread / unsupported

    def cleanup(self) -> None:
        if self._cleaned or self.attached or self.pid is None:
            return
        self._cleaned = True
        try:
            if self.proc is not None:
                self.proc.terminate()
                try:
                    self.proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.proc.kill()
            elif pid_alive(self.pid):
                os.kill(self.pid, signal.SIGTERM)
                time.sleep(1)
                if pid_alive(self.pid):
                    os.kill(self.pid, signal.SIGKILL)
        except OSError:
            pass
        if self.pid_file:
            try:
                with open(self.pid_file, encoding='utf-8') as f:
                    recorded = int(f.read().strip())
                if recorded == self.pid:
                    os.unlink(self.pid_file)
            except (OSError, ValueError):
                pass
