// Template-honesty gate (M-7, live audit 2026-09-30): semantic snapshots
// must NEVER carry mock template markers — not in live mode (the finding:
// the Project tab rendered `/Users/<user>/...` on a real contract project)
// and, since the mock transport now emits real-shaped fixture paths, not in
// demo mode either. Fails with file + marker; exit 1 blocks the gate.
//
// Usage: node e2e/scenes/check-template-honesty.mjs [snapshot-dir]
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

const MARKERS = ['/Users/<user>', '<mod>', '<publishedfileid>'];
const dir =
  process.argv[2] ??
  process.env.RIMLOC_SNAPSHOT_DIR ??
  '/Users/danielviktorovich/Developing/RimLoc-evidence/semantic-snapshots';

let files;
try {
  files = (await readdir(dir)).filter((f) => f.endsWith('.json'));
} catch (e) {
  console.error(`template-honesty: cannot read snapshot dir ${dir}: ${e.message}`);
  process.exit(2);
}
if (files.length === 0) {
  console.error(`template-honesty: no snapshots in ${dir} — run snapshot:semantic first`);
  process.exit(2);
}

const violations = [];
for (const f of files) {
  const text = await readFile(join(dir, f), 'utf8');
  for (const marker of MARKERS) {
    if (text.includes(marker)) {
      // Укажем контекст первого вхождения — без вываливания всего файла.
      const at = text.indexOf(marker);
      const ctx = text.slice(Math.max(0, at - 80), at + marker.length + 40).replace(/\s+/g, ' ');
      violations.push(`${f}: «${marker}» → …${ctx}…`);
    }
  }
}

if (violations.length > 0) {
  console.error(`template-honesty: FAIL — ${violations.length} маркер(ов) в ${dir}:`);
  for (const v of violations) console.error('  ' + v);
  process.exit(1);
}
console.log(`template-honesty: PASS — ${files.length} снимков без шаблонных маркеров`);
