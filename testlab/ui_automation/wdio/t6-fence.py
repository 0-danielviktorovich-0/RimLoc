#!/usr/bin/env python3
"""T6 write fence + identity guard (owner §2/§3, 2026-10-01).

Fence rules for every T6 GUI journey:
  1. IDENTITY: after project create/open, the harness inspects the ISOLATED
     data dir and requires exactly ONE managed project whose source_root
     equals the T6 fixture copy. Owner managed roots are never read.
  2. WRITE ROOT: every output path used in a spec must resolve under
     T6_ALLOWED_WRITE_ROOT. The harness verifies the value BEFORE clicking
     run; a spec that tries anything outside is aborted pre-click.
  3. ADVERSARIAL REGRESSION (self-test, `--selftest`): attempt to make the
     journey target an owner-style path → the fence must refuse pre-click,
     and the protected root bytes must be unchanged (canonical hash).

Usage:
  t6-fence.py check-identity <data-dir> <expected-src-root>
  t6-fence.py check-outdir <allowed-root> <candidate-out>
  t6-fence.py selftest
"""
import hashlib
import json
import os
import sys
from pathlib import Path

FAIL = 0


def fail(msg: str) -> None:
    global FAIL
    FAIL = 1
    print(f"[fence] FAIL: {msg}")


def ok(msg: str) -> None:
    print(f"[fence] ok: {msg}")


def canonical_tree(root: str) -> str:
    lines = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames[:] = sorted(d for d in dirnames if d != ".DS_Store" and not d.startswith("._"))
        for name in sorted(filenames):
            if name == ".DS_Store" or name.startswith("._"):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root).replace(os.sep, "/")
            h = hashlib.sha256(open(full, "rb").read()).hexdigest()
            lines.append(f"{h}  {rel}")
    return hashlib.sha256("\n".join(sorted(lines)).encode()).hexdigest()


def check_identity(data_dir: str, expected_src: str) -> None:
    managed = Path(data_dir) / "managed"
    if not managed.is_dir():
        fail(f"isolated managed dir missing: {managed}")
        return
    projects = list(managed.glob("*.rimloc.json"))
    if len(projects) != 1:
        fail(f"isolated data dir must contain exactly 1 managed project, found {len(projects)}")
        return
    d = json.load(open(projects[0], encoding="utf-8"))
    src = d.get("source_root") or ""
    if os.path.realpath(src) != os.path.realpath(expected_src):
        fail(f"project source_root {src!r} != expected fixture {expected_src!r}")
        return
    ok(f"identity verified: {d.get('project_id')} ← {src}")


def outdir_allowed(allowed_root: str, candidate: str) -> bool:
    root = os.path.realpath(allowed_root)
    cand = os.path.realpath(candidate)
    return cand != root and cand.startswith(root + os.sep)


def check_outdir(allowed_root: str, candidate: str) -> None:
    if outdir_allowed(allowed_root, candidate):
        ok(f"outdir under allowed root: {candidate}")
    else:
        fail(f"outdir OUTSIDE allowed root (or equals root): {candidate} not strictly under {allowed_root}")


def selftest() -> None:
    # Adversarial regression (§3): attempt to point T6 at an owner-style
    # path; the fence must refuse pre-write and protected bytes stay same.
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        owner = os.path.join(tmp, "owner")
        os.makedirs(os.path.join(owner, "nested"), exist_ok=True)
        protected = os.path.join(owner, "nested", "proj.production.rimloc.json")
        open(protected, "w").write('{"marker":"owner-bytes"}')
        before = canonical_tree(owner)

        # adversarial: outdir внутри owner-корня — обязан быть ОТКАЗ
        # adversarial: кандидаты под OWNER-корнем должны быть ОТКЛОНЕНЫ
        # фенсом T6-корня (семантика §3: только T6_ALLOWED_WRITE_ROOT)
        t6_root = os.path.join(tmp, "t6-root")
        os.makedirs(t6_root, exist_ok=True)
        owner_candidates = [os.path.join(owner, "export"), owner]
        refused = sum(0 if outdir_allowed(t6_root, c) else 1 for c in owner_candidates)
        if refused != len(owner_candidates):
            fail(f"adversarial outdir attempts were NOT refused ({refused}/{len(owner_candidates)})")
        else:
            ok("adversarial owner-root candidates refused by the T6 fence")
        # корректный кейс: внутри T6-корня — разрешено
        before_fail = FAIL
        check_outdir(t6_root, os.path.join(t6_root, "mod-package"))
        if FAIL != before_fail:
            fail("legitimate outdir was refused")

        after = canonical_tree(owner)
        if before != after:
            fail("PROTECTED BYTES CHANGED during adversarial attempts")
        else:
            ok("protected owner bytes unchanged (canonical hash)")
        if FAIL:
            print("[fence] SELFTEST FAILED")
            sys.exit(1)
        print("[fence] SELFTEST PASS")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "check-identity":
        check_identity(sys.argv[2], sys.argv[3])
    elif cmd == "check-outdir":
        check_outdir(sys.argv[2], sys.argv[3])
    elif cmd == "selftest":
        selftest()
    else:
        print(__doc__)
        sys.exit(2)
    sys.exit(FAIL)
