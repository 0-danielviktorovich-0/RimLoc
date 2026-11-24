#!/usr/bin/env python3
"""winframe <pid> — native window frame + display geometry, CG coordinates.

Machine-readable output consumed by testlab window checks (window-state-check.sh
frame_of/main_frame_of) and the narrow-window lane (T6 #161):

  frame=x,y,WxH                   largest layer-0 window of <pid>
  screens=[x,y,WxH|x,y,WxH...]    per-display visibleFrame

Both lines use the SAME global top-left (CG) coordinate space: CGWindowBounds
are reported that way natively, AppKit visibleFrames are converted with the
primary-screen flip cg_y = primary_height - (y + h), so "window inside a
visible frame" checks compare like with like. Exit 1 (silent) when the pid has
no layer-0 window yet.

Run with a python that has pyobjc (the lane bootstraps /tmp/rimloc-t6-venv).
"""
import sys

import AppKit
import Quartz


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: winframe <pid>", file=sys.stderr)
        return 2
    try:
        pid = int(sys.argv[1])
    except ValueError:
        print("usage: winframe <pid>", file=sys.stderr)
        return 2

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
        return 1
    b = best[2]
    print(f"frame={int(b['X'])},{int(b['Y'])},{int(b['Width'])}x{int(b['Height'])}")

    screens = AppKit.NSScreen.screens()
    h0 = int(screens[0].frame().size.height)
    parts = []
    for s in screens:
        vf = s.visibleFrame()
        sw, sh = int(vf.size.width), int(vf.size.height)
        cg_y = h0 - (int(vf.origin.y) + sh)
        parts.append(f"{int(vf.origin.x)},{cg_y},{sw}x{sh}")
    print("screens=[" + "|".join(parts) + "]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
