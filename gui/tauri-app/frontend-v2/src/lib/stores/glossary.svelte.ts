// Mock glossary store (mandate §10 — glossary editor, spec §12/§17). Purely
// local state: add/edit/remove mutate this store, never a backend. Terms bind
// a source term to its approved target with scope (project vs user library),
// accepted variants and case-sensitivity — the fields the deterministic
// glossary check (spec §8) needs on the output side.
//
// Every term carries a mutability provenance (W4.5 requirement #2):
// user-created / project-created / imported / reference-read-only.
// Reference corpora are immutable — update()/remove() refuse them here so
// the guarantee holds even if an editor forgets to disable the button.
//
// Import/export are UI mocks: no file is read or written, the buttons only
// flash a status line on the editor screen. The mock batch-import appends a
// canned record marked `imported` so the provenance is visible.

import { isReadOnly, type Mutability } from '../mutability';

export type GlossaryScope = 'project' | 'user';

export interface GlossaryTerm {
  id: string;
  /** Source-language term (what may appear in SOURCE text). */
  source: string;
  /** Approved translation for TARGET. */
  target: string;
  note: string;
  sourceLang: string;
  targetLang: string;
  scope: GlossaryScope;
  /** Accepted alternative spellings of the source term. */
  variants: string[];
  caseSensitive: boolean;
  /** Where the term came from (see mutability.ts). */
  mutability: Mutability;
}

let seq = 0;
function nextId(): string {
  seq += 1;
  return `glossary-${String(seq).padStart(3, '0')}`;
}

// Mock corpus: EN→RU RimWorld terminology, 10 entries. Two entries share the
// normalized source "hydroponics" so the conflict view has a real case to show.
// Mutability is assigned to match the mock story: user-scope terms are
// user-created, project terms project-created, two imported from an earlier
// pack and two canonical reference terms are read-only (W4.5 #2 demo corpus).
const INITIAL: GlossaryTerm[] = [
  { id: nextId(), source: 'hydroponics', target: 'гидропоника', note: 'Базовый термин для всех растений в basins.', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: [], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'Hydroponics', target: 'Гидропоника', note: 'Капитализированное написание в заголовках исследований.', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: [], caseSensitive: true, mutability: 'reference-read-only' },
  { id: nextId(), source: 'power', target: 'энергия', note: 'Не «мощность» и не «электричество».', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: ['electrical power'], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'solar panel', target: 'солнечная панель', note: '', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: ['solar generator'], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'raid', target: 'рейд', note: 'Существительное; «атаковать рейдом» — глагольные формы свободны.', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: ['enemy raid'], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'plague', target: 'чума', note: '', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: ['disease'], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'mood', target: 'настроение', note: 'Параметр пешки, не «настроение игры».', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: [], caseSensitive: false, mutability: 'project-created' },
  { id: nextId(), source: 'mortal error', target: 'смертельная ошибка', note: 'Шанс из WordInfo; сохранять регистр метки.', sourceLang: 'en', targetLang: 'ru', scope: 'user', variants: ['mortal threat'], caseSensitive: true, mutability: 'user-created' },
  { id: nextId(), source: 'caravan', target: 'караван', note: 'Справочный термин базовой игры.', sourceLang: 'en', targetLang: 'ru', scope: 'user', variants: ['traveling caravan'], caseSensitive: false, mutability: 'reference-read-only' },
  { id: nextId(), source: 'tamed animal', target: 'приручённое животное', note: 'Ё — обязательно: «прирученое» — опечатка.', sourceLang: 'en', targetLang: 'ru', scope: 'project', variants: ['trained animal'], caseSensitive: false, mutability: 'imported' }
];

class GlossaryStore {
  list = $state<GlossaryTerm[]>(structuredClone(INITIAL));

  /** Substring search over source, target, note and variants (case-insensitive). */
  search(q: string): GlossaryTerm[] {
    const needle = q.trim().toLowerCase();
    if (!needle) return this.list;
    return this.list.filter(
      (t) =>
        t.source.toLowerCase().includes(needle) ||
        t.target.toLowerCase().includes(needle) ||
        t.note.toLowerCase().includes(needle) ||
        t.variants.some((v) => v.toLowerCase().includes(needle))
    );
  }

  /**
   * Terms whose match-sets overlap — the editor's conflict view. Two terms
   * conflict when both can match the same source text: a case-insensitive
   * term matches any casing, so it collides with everything in its lowercase
   * group; case-sensitive terms only collide on the exact same spelling.
   */
  conflicts(): GlossaryTerm[][] {
    const groups = new Map<string, GlossaryTerm[]>();
    for (const t of this.list) {
      const keys = [t.source, ...t.variants].map((s) => s.toLowerCase());
      for (const k of keys) {
        const arr = groups.get(k) ?? [];
        if (!arr.some((x) => x.id === t.id)) arr.push(t);
        groups.set(k, arr);
      }
    }
    return [...groups.values()].filter((g) => {
      if (g.length < 2) return false;
      const allCaseSensitive = g.every((t) => t.caseSensitive);
      if (!allCaseSensitive) return true; // one loose term swallows the rest
      return new Set(g.map((t) => t.source)).size < g.length;
    });
  }

  /**
   * Manual creation derives mutability from the scope (behavior-review 008
   * #5): a term created into the PROJECT scope is project-created, one into
   * the PERSONAL library is user-created — the provenance vocabulary stays
   * consistent with the mock corpus's scope split.
   */
  add(term: Omit<GlossaryTerm, 'id' | 'mutability'>): GlossaryTerm {
    const mutability: Mutability = term.scope === 'project' ? 'project-created' : 'user-created';
    const created: GlossaryTerm = { ...term, mutability, id: nextId() };
    this.list = [...this.list, created];
    return created;
  }

  /** Batch import provenance: every imported record is marked `imported`. */
  addImported(term: Omit<GlossaryTerm, 'id' | 'mutability'>): GlossaryTerm {
    const created: GlossaryTerm = { ...term, mutability: 'imported', id: nextId() };
    this.list = [...this.list, created];
    return created;
  }

  update(id: string, patch: Partial<Omit<GlossaryTerm, 'id'>>) {
    const term = this.list.find((t) => t.id === id);
    // Reference corpora are immutable by design (W4.5 #2): refuse the write
    // here, not only in the UI.
    if (!term || isReadOnly(term.mutability)) return;
    this.list = this.list.map((t) => (t.id === id ? { ...t, ...patch, id } : t));
  }

  remove(id: string) {
    const term = this.list.find((t) => t.id === id);
    if (!term || isReadOnly(term.mutability)) return;
    this.list = this.list.filter((t) => t.id !== id);
  }
}

export const glossary = new GlossaryStore();
