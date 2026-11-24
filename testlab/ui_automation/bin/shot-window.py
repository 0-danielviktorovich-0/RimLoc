#!/usr/bin/env python3
"""shot-window.py <pid> <out-prefix> — native window screenshots, no focus.

Two independent capture paths (T6 #161 evidence requirement):
  1. Quartz CGWindowListCreateImage with kCGWindowImageBoundsIgnoreFraming —
     the driver.py precedent; works even for windows outside the display
     geometry (pixels = 2x window CSS px on Retina);
  2. the `screencapture -x -l<windowid>` CLI (windowid from CGWindowList,
     NOT from AX — ax/README.md).

Writes <out-prefix>.cg.png and <out-prefix>.cli.png (whichever succeed), plus
one machine-readable summary line:
  window=<id> frame=x,y,WxH shot=<path>[,<path>]
Exit 1 when the pid exposes no layer-0 window.
"""
import json
import subprocess
import sys

import Quartz


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: shot-window.py <pid> <out-prefix>", file=sys.stderr)
        return 2
    pid = int(sys.argv[1])
    prefix = sys.argv[2]

    best = None
    for w in Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID
    ):
        if int(w.get("kCGWindowOwnerPID", -1)) != pid:
            continue
        if int(w.get("kCGWindowLayer", 1)) != 0:
            continue
        b = dict(w["kCGWindowBounds"])
        area = b["Width"] * b["Height"]
        if best is None or area > best[0]:
            best = (area, int(w["kCGWindowNumber"]), b)
    if best is None:
        print(f"shot-window: no layer-0 window for pid={pid}", file=sys.stderr)
        return 1
    _, window_id, b = best
    frame = f"{int(b['X'])},{int(b['Y'])},{int(b['Width'])}x{int(b['Height'])}"

    saved = []
    img = Quartz.CGWindowListCreateImage(
        Quartz.CGRectNull,
        Quartz.kCGWindowListOptionIncludingWindow,
        window_id,
        Quartz.kCGWindowImageBoundsIgnoreFraming,
    )
    if img is not None:
        out = f"{prefix}.cg.png"
        url = Quartz.CFURLCreateFromFileSystemRepresentation(
            None, out.encode(), len(out.encode()), False
        )
        dest = Quartz.CGImageDestinationCreateWithURL(url, "public.png", 1, None)
        if dest is not None:
            Quartz.CGImageDestinationAddImage(dest, img, None)
            Quartz.CGImageDestinationFinalize(dest)
            saved.append(out)
    cli_out = f"{prefix}.cli.png"
    try:
        # -x silent, -o no window shadow (driver.py parity); the CLI works
        # wherever the calling context holds Screen Recording permission —
        # the primary capture path (CGWindowListCreateImage needs the same
        # TCC grant in-process and stays best-effort).
        r = subprocess.run(
            ["/usr/sbin/screencapture", "-x", "-o", f"-l{window_id}", cli_out],
            capture_output=True,
            timeout=15,
        )
        if r.returncode == 0:
            saved.append(cli_out)
        else:
            print(
                f"shot-window: screencapture rc={r.returncode} "
                f"{r.stderr.decode(errors='replace').strip()}",
                file=sys.stderr,
            )
    except Exception as exc:  # noqa: BLE001 — diagnostics, never fatal
        print(f"shot-window: screencapture failed: {exc}", file=sys.stderr)

    print(json.dumps({"window": window_id, "frame": frame, "shots": saved}))
    return 0 if saved else 1


if __name__ == "__main__":
    sys.exit(main())
