#!/usr/bin/env python3
import re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
targets = [
    ROOT / 'gui' / 'tauri-app' / 'frontend' / 'index.js',
    ROOT / 'gui' / 'tauri-app' / 'frontend' / 'index.html',
]

hardcoded = []

patterns = [
    re.compile(r"showToast\(\s*\"(?!\s*\$\{|\s*\+|\s*`|.*tr\()"),
    re.compile(r"runAction\(\s*\"(?!.*tr\()"),
]

for t in targets:
    if not t.exists():
        continue
    with t.open('r', encoding='utf-8') as f:
        for i, line in enumerate(f, 1):
            s = line.strip()
            for pat in patterns:
                if pat.search(s):
                    hardcoded.append((str(t.relative_to(ROOT)), i, s))
                    break

if hardcoded:
    print('Hardcoded UI strings detected (use i18n tr()/data-i18n):')
    for path, ln, s in hardcoded:
        print(f" - {path}:{ln}: {s}")
    sys.exit(2)
else:
    print('i18n lint: OK')
