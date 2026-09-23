// Mock data for the existing-translation flow (mandate §13): the language
// packs a user may already have and the analysis counters of comparing a pack
// against the current mod version. Numbers are fictional and deliberately not
// recomputed from the 60-entry editor corpus — this flow demonstrates a
// real-scale project, same as the wizard mocks.

import type { WizardMod } from './wizard';

export interface LanguagePack {
  id: string;
  /** Mod this pack belongs to (mockMods[].id). */
  modId: string;
  name: string;
  version: string;
  locale: string;
  /** Where the pack comes from (plain-language provenance). */
  origin: 'community' | 'mod-author';
}

/** Existing packs found next to the installed mods. */
export const mockLanguagePacks: LanguagePack[] = [
  {
    id: 'vfe-vanilla-ru',
    modId: 'vfe-furniture',
    name: 'Vanilla Furniture Expanded — RU',
    version: '2.1',
    locale: 'ru',
    origin: 'community'
  },
  {
    id: 'vfe-fan-ru',
    modId: 'vfe-furniture',
    name: 'VFE-RU (fan update)',
    version: '1.9',
    locale: 'ru',
    origin: 'community'
  },
  {
    id: 'rimatomics-ru',
    modId: 'rimatomics',
    name: 'Rimatomics — RU',
    version: '2.2',
    locale: 'ru',
    origin: 'community'
  },
  {
    id: 'hospitality-de',
    modId: 'hospitality',
    name: 'Hospitality — DE',
    version: '1.14',
    locale: 'de',
    origin: 'community'
  },
  {
    id: 'kaizen-ru',
    modId: 'kaizen',
    name: 'Kaizen-RU',
    version: '0.9',
    locale: 'ru',
    origin: 'mod-author'
  }
];

/** Analysis result counters (mandate §13 example). */
export const mockAnalysis = {
  reusable: 1421,
  sourceChanged: 37,
  new: 58,
  obsolete: 21,
  invalid: 4,
  ambiguous: 2
};

export type { WizardMod };
