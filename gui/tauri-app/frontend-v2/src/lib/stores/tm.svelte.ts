// Mock translation-memory store (mandate §11 — TM editor). Local-only state;
// import/export and "apply" are UI mocks. Each entry carries a similarity
// score, the project it came from (origin) and provenance (who produced the
// pair) so the editor can show both dimensions separately, as spec §20 does.
//
// The mock corpus holds 14 rows. A production project runs to tens of
// thousands — the editor must render such corpora through a virtualized
// table (the note on the screen says exactly that).

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
  addedAt: string
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
    addedAt
  };
}

// Mock corpus: EN→RU pairs from three source projects. "Hydroponics basin →
// Гидропонный бассейн" appears twice (two projects) so the duplicate view has
// a real case; the editor flags duplicates only, it never silently merges.
const INITIAL: TmEntry[] = [
  entry('Hydroponics basin', 'Гидропонный бассейн', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-14'),
  entry('Hydroponics basin', 'Гидробассейн', 92, 'Vanilla Expanded RU', 'tm', '2026-06-02'),
  entry('Research the solar panel to begin power generation.', 'Чтобы начать выработку энергии, изучите солнечную панель.', 100, 'Core UI RU', 'human', '2026-05-21'),
  entry('A colonist is having a mental break.', 'У поселенца нервный срыв.', 96, 'Core UI RU', 'imported', '2026-05-21'),
  entry('Raid incoming from the {0} direction.', 'Надвигается рейд с направления {0}.', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-14'),
  entry('Beware: this building may catch fire.', 'Осторожно: это здание может загореться.', 88, 'Vanilla Expanded RU', 'ai', '2026-06-11'),
  entry('Tame the animal to add it to your colony.', 'Приручите животное, чтобы добавить его в колонию.', 97, 'Core UI RU', 'human', '2026-04-30'),
  entry('The caravan has arrived at its destination.', 'Караван прибыл к месту назначения.', 95, 'Core UI RU', 'imported', '2026-04-30'),
  entry('Medical tend quality: {0}', 'Качество лечения: {0}', 100, 'EdB Prepare Carefully RU', 'human', '2026-07-15'),
  entry('Not enough power to run this workbench.', 'Недостаточно энергии для работы этого верстака.', 93, 'Vanilla Expanded RU', 'tm', '2026-06-12'),
  entry('This door must be constructed before the winter.', 'Эту дверь нужно построить до зимы.', 81, 'Vanilla Expanded RU', 'ai', '2026-06-20'),
  entry('Plague has struck your colonist.', 'Вашего поселенца поразила чума.', 99, 'Core UI RU', 'human', '2026-05-02'),
  entry('Slaughter the animal?', 'Забить животное?', 100, 'Core UI RU', 'imported', '2026-05-03'),
  entry('Mood bonus from a comfortable environment.', 'Бонус к настроению от комфортной обстановки.', 90, 'EdB Prepare Carefully RU', 'ai', '2026-07-20')
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

  add(e: Omit<TmEntry, 'id'>): TmEntry {
    const created: TmEntry = { ...e, id: nextId() };
    this.list = [...this.list, created];
    return created;
  }

  update(id: string, patch: Partial<Omit<TmEntry, 'id'>>) {
    this.list = this.list.map((e) => (e.id === id ? { ...e, ...patch, id } : e));
  }

  remove(id: string) {
    this.list = this.list.filter((e) => e.id !== id);
  }
}

export const tm = new TmStore();
