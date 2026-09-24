// Mock translation-memory store (mandate §11 — TM editor). Local-only state;
// import/export and "apply" are UI mocks. Each entry carries a similarity
// score, the project it came from (origin) and provenance (who produced the
// pair) so the editor can show both dimensions separately, as spec §20 does.
//
// Every entry also carries a mutability provenance (W4.5 requirement #2):
// user-created / project-created / imported / reference-read-only. Reference
// corpora are immutable — update()/remove() refuse them at store level.
//
// The mock corpus holds 14 rows. A production project runs to tens of
// thousands — the editor must render such corpora through a virtualized
// table (the note on the screen says exactly that).

import { isReadOnly, type Mutability } from '../mutability';

export type TmOrigin = 'human' | 'tm' | 'ai' | 'imported';

export interface TmEntry {
  id: string;
  source: string;
  target: string;
  sourceLang: string;
  targetLang: string;
  /** Best-match quality 0-100. */
  similarity: number;
  /** Project the pair came from ("library" = user-wide TM). */
  originProject: string;
  /** Who/what produced the pair (spec §9 provenance vocabulary). */
  provenance: TmOrigin;
  /** Where the record came from and whether it may change (mutability.ts). */
  mutability: Mutability;
  addedAt: string;
}

let seq = 0;
function nextId(): string {
  seq += 1;
  return `tm-${String(seq).padStart(3, '0')}`;
}

function entry(
  source: string,
  target: string,
  similarity: number,
  originProject: string,
  provenance: TmOrigin,
  addedAt: string,
  mutability: Mutability
): TmEntry {
  return {
    id: nextId(),
    source,
    target,
    sourceLang: 'en',
    targetLang: 'ru',
    similarity,
    originProject,
    provenance,
    mutability,
    addedAt
  };
}

// Mock corpus: EN→RU pairs from three source projects. "Hydroponics basin →
// Гидропонный бассейн" appears twice (two projects) so the duplicate view has
// a real case; the editor flags duplicates only, it never silently merges.
// Mutability (W4.5 #2): imported-pack rows are `imported`, two canonical
// base-game reference pairs are read-only, the rest follow their provenance.
const INITIAL: TmEntry[] = [
  entry('Hydroponics basin', 'Гидропонный бассейн', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-14', 'project-created'),
  entry('Hydroponics basin', 'Гидробассейн', 92, 'Vanilla Expanded RU', 'tm', '2026-06-02', 'project-created'),
  entry('Research the solar panel to begin power generation.', 'Чтобы начать выработку энергии, изучите солнечную панель.', 100, 'Core UI RU', 'human', '2026-05-21', 'reference-read-only'),
  entry('A colonist is having a mental break.', 'У поселенца нервный срыв.', 96, 'Core UI RU', 'imported', '2026-05-21', 'imported'),
  entry('Raid incoming from the {0} direction.', 'Надвигается рейд с направления {0}.', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-14', 'project-created'),
  entry('Beware: this building may catch fire.', 'Осторожно: это здание может загореться.', 88, 'Vanilla Expanded RU', 'ai', '2026-06-11', 'project-created'),
  entry('Tame the animal to add it to your colony.', 'Приручите животное, чтобы добавить его в колонию.', 97, 'Core UI RU', 'human', '2026-04-30', 'project-created'),
  entry('The caravan has arrived at its destination.', 'Караван прибыл к месту назначения.', 95, 'Core UI RU', 'imported', '2026-04-30', 'imported'),
  entry('Medical tend quality: {0}', 'Качество лечения: {0}', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-15', 'reference-read-only'),
  entry('Not enough power to run this workbench.', 'Недостаточно энергии для работы этого верстака.', 93, 'Vanilla Expanded RU', 'tm', '2026-06-12', 'project-created'),
  entry('This door must be constructed before the winter.', 'Эту дверь нужно построить до зимы.', 81, 'Vanilla Expanded RU', 'ai', '2026-06-20', 'project-created'),
  entry('Plague has struck your colonist.', 'Вашего поселенца поразила чума.', 99, 'Core UI RU', 'human', '2026-05-02', 'project-created'),
  entry('Slaughter the animal?', 'Забить животное?', 100, 'Core UI RU', 'imported', '2026-05-03', 'imported'),
  entry('Mood bonus from a comfortable environment.', 'Бонус к настроению от комфортной обстановки.', 90, 'EdB Prepare Carefully RU', 'ai', '2026-07-20', 'project-created')
];

class TmStore {
  list = $state<TmEntry[]>(structuredClone(INITIAL));

  /** Substring search over source, target and origin project (case-insensitive). */
  search(q: string): TmEntry[] {
    const needle = q.trim().toLowerCase();
    if (!needle) return this.list;
    return this.list.filter(
      (e) =>
        e.source.toLowerCase().includes(needle) ||
        e.target.toLowerCase().includes(needle) ||
        e.originProject.toLowerCase().includes(needle)
    );
  }

  /** Duplicate source pairs: same normalized source text, different target. */
  duplicateIds(): Set<string> {
    const seen = new Map<string, Set<string>>();
    for (const e of this.list) {
      const k = e.source.toLowerCase();
      const targets = seen.get(k) ?? new Set<string>();
      targets.add(e.target);
      seen.set(k, targets);
    }
    const dups = new Set<string>();
    for (const e of this.list) {
      if ((seen.get(e.source.toLowerCase())?.size ?? 0) > 1) dups.add(e.id);
    }
    return dups;
  }

  /** Manual creation is a user-created record. */
  add(e: Omit<TmEntry, 'id' | 'mutability'>): TmEntry {
    const created: TmEntry = { ...e, mutability: 'user-created', id: nextId() };
    this.list = [...this.list, created];
    return created;
  }

  /** Batch import provenance: imported records are marked `imported`. */
  addImported(e: Omit<TmEntry, 'id' | 'mutability'>): TmEntry {
    const created: TmEntry = { ...e, mutability: 'imported', id: nextId() };
    this.list = [...this.list, created];
    return created;
  }

  update(id: string, patch: Partial<Omit<TmEntry, 'id'>>) {
    const entry = this.list.find((e) => e.id === id);
    // Reference corpora are immutable by design (W4.5 #2): refuse here,
    // not only in the UI.
    if (!entry || isReadOnly(entry.mutability)) return;
    this.list = this.list.map((e) => (e.id === id ? { ...e, ...patch, id } : e));
  }

  remove(id: string) {
    const entry = this.list.find((e) => e.id === id);
    if (!entry || isReadOnly(entry.mutability)) return;
    this.list = this.list.filter((e) => e.id !== id);
  }
}

export const tm = new TmStore();
