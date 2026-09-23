// Mock data for the Quick Translate wizard (mandate §4), the Home recent list
// (§3), the review overview (§12) and the glossary glimpse. Numbers are
// fictional but internally consistent and deliberately not recomputed from the
// 60-entry editor corpus: the wizard demonstrates a real-scale project.

export interface WizardMod {
  id: string;
  name: string;
  author: string;
  version: string;
  defs: number;
}

/** Auto-detected installed mods for wizard step 2 (mandate §4). */
export const mockMods: WizardMod[] = [
  { id: 'vfe-furniture', name: 'Vanilla Furniture Expanded', author: 'Oskar Potocki', version: '1.6.2', defs: 842 },
  { id: 'rimatomics', name: 'Rimatomics', author: 'Dubwise', version: '2.4.0', defs: 417 },
  { id: 'hospitality', name: 'Hospitality', author: 'Orion', version: '1.15.3', defs: 631 },
  { id: 'kaizen', name: '改善 Kaizen Module', author: 'Hoshino', version: '0.9.1', defs: 154 }
];

/** Wizard step 5 preflight counters (mandate §4 example). */
export const mockPreflight = {
  entries: 1842,
  reusable: 1219,
  needTranslation: 623,
  attention: 14
};

/** Wizard step 7 result counters (mandate §4 example). */
export const mockResult = {
  validated: 4603,
  review: 173,
  errors: 36
};

/** Wizard step 6 pipeline phases; weights sum to 100. */
export interface WizardPhase {
  id: 'scan' | 'tm' | 'ai' | 'validate';
  weight: number;
}

export const wizardPhases: WizardPhase[] = [
  { id: 'scan', weight: 10 },
  { id: 'tm', weight: 30 },
  { id: 'ai', weight: 45 },
  { id: 'validate', weight: 15 }
];

export interface RecentProject {
  id: string;
  name: string;
  source: string;
  target: string;
  /** Completed percent (0-100). */
  progress: number;
  sourceChanged: number;
  issues: number;
  modified: string;
}

/** Home returning-user list (mandate §3). */
export const mockProjects: RecentProject[] = [
  {
    id: 'p1',
    name: 'Vanilla Furniture Expanded — RU',
    source: 'en',
    target: 'ru',
    progress: 87,
    sourceChanged: 12,
    issues: 4,
    modified: '2026-09-22T18:40:00Z'
  },
  {
    id: 'p2',
    name: 'Rimatomics — RU',
    source: 'en',
    target: 'ru',
    progress: 64,
    sourceChanged: 3,
    issues: 0,
    modified: '2026-09-19T09:12:00Z'
  },
  {
    id: 'p3',
    name: 'Hospitality — DE',
    source: 'en',
    target: 'de',
    progress: 41,
    sourceChanged: 0,
    issues: 9,
    modified: '2026-09-11T21:05:00Z'
  }
];

/** Review overview counters (mandate §12 example). */
export const mockReviewOverview = {
  needsReview: 173,
  errors: 36,
  sourceChanged: 24,
  glossaryConflicts: 11
};

/** Review issue categories with mock counts (mandate §12). */
export const mockReviewCategories = [
  { id: 'placeholder_mismatch', count: 14 },
  { id: 'untranslated_suspect', count: 9 },
  { id: 'glossary', count: 11 },
  { id: 'wordinfo', count: 6 },
  { id: 'ambiguity', count: 2 },
  { id: 'ai_concern', count: 131 }
] as const;

/** Glossary glimpse for the Workspace Glossary tab. */
export const mockGlossary = [
  { en: 'colonist', ru: 'колонист' },
  { en: 'raid', ru: 'рейд' },
  { en: 'research', ru: 'исследование' },
  { en: 'mood', ru: 'настроение' },
  { en: 'hay', ru: 'сено' }
];

/** TM glimpse for the Workspace TM tab. */
export const mockTm = [
  { source: 'A solar flare has knocked out your power.', target: 'Солнечная вспышка вывела энергосистему из строя.', from: 'Core 1.5', match: '100%' },
  { source: 'Research finished: {0}', target: 'Исследование завершено: {0}', from: 'Core 1.5', match: '98%' }
];
