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

// Content-branch mocks (QA mandate §2/§14): wizard step 2 branches by content
// type — base game needs installations, DLC needs installed expansions and a
// language pack needs its language mapping. Numbers are fictional but shape-
// faithful to RimWorld content.

export interface WizardInstallation {
  id: string;
  /** Human label: store + platform. */
  label: string;
  /** Game build version, shown as the Core version for this installation. */
  version: string;
  path: string;
}

/** Auto-detected game installations for the base-game/DLC branches. */
export const mockInstallations: WizardInstallation[] = [
  {
    id: 'steam-win',
    label: 'Steam · Windows',
    version: '1.6.4537',
    path: 'C:/Program Files (x86)/Steam/steamapps/common/RimWorld'
  },
  {
    id: 'gog-mac',
    label: 'GOG · macOS',
    version: '1.5.4409',
    path: '/Applications/RimWorld/RimWorldMac.app'
  }
];

export interface WizardDlc {
  id: string;
  name: string;
  version: string;
  /** Only installed DLC can be selected for translation. */
  installed: boolean;
}

/** Known RimWorld DLC with the installed flag per mock state. */
export const mockDlc: WizardDlc[] = [
  { id: 'royalty', name: 'Royalty', version: '1.6.4537', installed: true },
  { id: 'ideology', name: 'Ideology', version: '1.6.4537', installed: true },
  { id: 'biotech', name: 'Biotech', version: '1.6.4537', installed: true },
  { id: 'anomaly', name: 'Anomaly', version: '1.6.4537', installed: true },
  { id: 'odyssey', name: 'Odyssey', version: '1.6.4537', installed: false }
];

export interface WizardLanguagePack {
  id: string;
  name: string;
  /** Source language the pack translates from. */
  from: string;
  /** Language the pack delivers. */
  to: string;
  /** Pack source: shipped with the game or community-maintained. */
  origin: 'official' | 'community';
  version: string;
  entries: number;
}

/** Detected language packs for the language-pack branch. */
export const mockLanguagePacks: WizardLanguagePack[] = [
  {
    id: 'core-ru',
    name: 'RimWorld Core — Русский',
    from: 'en',
    to: 'ru',
    origin: 'official',
    version: '1.6.4537',
    entries: 4096
  },
  {
    id: 'core-de',
    name: 'Deutsch (Community)',
    from: 'en',
    to: 'de',
    origin: 'community',
    version: '1.5.4409',
    entries: 3877
  },
  {
    id: 'core-ja',
    name: '日本語 (Community)',
    from: 'en',
    to: 'ja',
    origin: 'community',
    version: '1.5.4409',
    entries: 3712
  }
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
