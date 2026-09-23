// Static mock dataset: 30 fictional RimWorld-like entries covering all three
// record kinds (Keyed / DefInjected / TKey) and all six statuses, including
// TKey rows for every serialization strategy. Zero backend calls — this is
// the only data source of the mock phase.

import type { Entry } from './types';

const F_KEYED = 'Languages/English/Keyed/Misc_Gameplay.xml';
const F_KEYED_UI = 'Languages/English/Keyed/Dialogs.xml';
const F_DI_WEAPONS = 'Languages/English/DefInjected/ThingDef/Things_Weapons.xml';
const F_DI_APPAREL = 'Languages/English/DefInjected/ThingDef/Things_Apparel.xml';
const F_DI_MISC = 'Languages/English/DefInjected/Misc/Defs_Misc.xml';
const F_TK_SCENARIO = 'TKey/Scenarios/Scenarios.xml';
const F_TK_QUEST = 'TKey/Quests/Quests.xml';

export const mockEntries: Entry[] = [
  // --- Keyed (11) ---
  {
    id: 'keyed-01',
    kind: 'Keyed',
    key: 'MessageLetterArrived',
    source: '{0}: A letter has arrived.',
    target: '{0}: Пришло письмо.',
    status: 'translated',
    file: F_KEYED,
    line: 12,
    origin: 'human',
    editedAt: '2026-09-20T10:14:00Z'
  },
  {
    id: 'keyed-02',
    kind: 'Keyed',
    key: 'AncientComplexWarning',
    source: 'Warning: the ancient complex is unstable.',
    target: '',
    status: 'untranslated',
    file: F_KEYED,
    line: 18
  },
  {
    id: 'keyed-03',
    kind: 'Keyed',
    key: 'ResearchTabTitle',
    source: 'Research',
    target: 'Исследования',
    status: 'translated',
    file: F_KEYED_UI,
    line: 4,
    origin: 'human',
    editedAt: '2026-09-18T08:02:00Z'
  },
  {
    id: 'keyed-04',
    kind: 'Keyed',
    key: 'ConfirmBanish',
    source: 'Banish {PAWN_nameDef}?',
    target: '',
    status: 'todo',
    file: F_KEYED_UI,
    line: 41,
    note: 'Проверить падеж имени после подстановки.'
  },
  {
    id: 'keyed-05',
    kind: 'Keyed',
    key: 'SolarFlare',
    source: 'A solar flare has knocked out your power.',
    target: 'Солнечная вспышка вывела энергосистему из строя.',
    status: 'translated',
    file: F_KEYED,
    line: 27,
    origin: 'TM',
    editedAt: '2026-09-19T15:30:00Z'
  },
  {
    id: 'keyed-06',
    kind: 'Keyed',
    key: 'RaidIncomingEnemy',
    source: '{ENEMYPAWN_nameFull} from {ENEMYFACTION_name} are raiding!',
    target: 'На вас напали!',
    status: 'sourceChanged',
    file: F_KEYED,
    line: 33,
    origin: 'human',
    editedAt: '2026-08-11T12:00:00Z'
  },
  {
    id: 'keyed-07',
    kind: 'Keyed',
    key: 'TradeBeaconLabel',
    source: 'Orbital trade beacon',
    target: 'Орбитальный торговый маяк',
    status: 'translated',
    file: F_KEYED,
    line: 52,
    origin: 'human',
    editedAt: '2026-09-15T09:41:00Z'
  },
  {
    id: 'keyed-08',
    kind: 'Keyed',
    key: 'DoorLabelOld',
    source: 'Door',
    target: 'Дверь',
    status: 'orphan',
    file: F_KEYED,
    line: 60,
    note: 'Ключ удалён из исходника в 1.6.'
  },
  {
    id: 'keyed-09',
    kind: 'Keyed',
    key: 'MechClusterIncoming',
    source: 'A mechanoid cluster is landing nearby.',
    target: '',
    status: 'untranslated',
    file: F_KEYED,
    line: 71
  },
  {
    id: 'keyed-10',
    kind: 'Keyed',
    key: 'PsychicRitualBegin',
    source: 'Begin psychic ritual',
    target: 'Начать психический ритуал',
    status: 'pending_review',
    file: F_KEYED_UI,
    line: 88,
    origin: 'LLM',
    editedAt: '2026-09-21T17:22:00Z'
  },
  {
    id: 'keyed-11',
    kind: 'Keyed',
    key: 'AlertHypothermia',
    source: 'Hypothermia',
    target: '',
    status: 'todo',
    file: F_KEYED,
    line: 95
  },

  // --- DefInjected (11) ---
  {
    id: 'di-01',
    kind: 'DefInjected',
    key: 'Gun_AssaultRifle.label',
    source: 'assault rifle',
    target: 'штурмовая винтовка',
    status: 'translated',
    file: F_DI_WEAPONS,
    line: 3,
    origin: 'human',
    editedAt: '2026-09-10T11:05:00Z'
  },
  {
    id: 'di-02',
    kind: 'DefInjected',
    key: 'Gun_AssaultRifle.description',
    source: 'A gas-operated military rifle. Good middle ground between range, damage and accuracy.',
    target: 'Армейская винтовка с газоотводной автоматикой. Хороший баланс дальности, урона и точности.',
    status: 'translated',
    file: F_DI_WEAPONS,
    line: 8,
    origin: 'human',
    editedAt: '2026-09-10T11:20:00Z'
  },
  {
    id: 'di-03',
    kind: 'DefInjected',
    key: 'MeleeWeapon_LongSword.label',
    source: 'longsword',
    target: '',
    status: 'todo',
    file: F_DI_WEAPONS,
    line: 19
  },
  {
    id: 'di-04',
    kind: 'DefInjected',
    key: 'Apparel_FlakVest.label',
    source: 'flak vest',
    target: 'бронежилет',
    status: 'translated',
    file: F_DI_APPAREL,
    line: 3,
    origin: 'TM',
    editedAt: '2026-09-08T14:12:00Z'
  },
  {
    id: 'di-05',
    kind: 'DefInjected',
    key: 'Apparel_FlakVest.description',
    source: 'A vest layered with plates of devilskin. Protects well, weighs little.',
    target: '',
    status: 'untranslated',
    file: F_DI_APPAREL,
    line: 7
  },
  {
    id: 'di-06',
    kind: 'DefInjected',
    key: 'Building_Sarcophagus.label',
    source: 'ancient sarcophagus',
    target: 'Саркофаг',
    status: 'sourceChanged',
    file: F_DI_MISC,
    line: 15,
    origin: 'human',
    editedAt: '2026-07-30T10:00:00Z'
  },
  {
    id: 'di-07',
    kind: 'DefInjected',
    key: 'Plant_RicePlant.label',
    source: 'rice plant',
    target: 'рис',
    status: 'translated',
    file: F_DI_MISC,
    line: 22,
    origin: 'imported',
    editedAt: '2026-09-01T08:45:00Z'
  },
  {
    id: 'di-08',
    kind: 'DefInjected',
    key: 'Hediff_Cataract.label',
    source: 'cataract',
    target: '',
    status: 'untranslated',
    file: F_DI_MISC,
    line: 34
  },
  {
    id: 'di-09',
    kind: 'DefInjected',
    key: 'Trait_GreenThumb.degreeData.label',
    source: 'green thumb',
    target: 'Зелёные пальцы',
    status: 'pending_review',
    file: F_DI_MISC,
    line: 47,
    origin: 'LLM',
    editedAt: '2026-09-21T17:40:00Z'
  },
  {
    id: 'di-10',
    kind: 'DefInjected',
    key: 'RoomStat_Beauty.label',
    source: 'beauty',
    target: 'Красота',
    status: 'orphan',
    file: F_DI_MISC,
    line: 55,
    note: 'Def переименован, запись ждёт перепривязки.'
  },
  {
    id: 'di-11',
    kind: 'DefInjected',
    key: 'TerrainDef_Sand.label',
    source: 'sand',
    target: 'песок',
    status: 'translated',
    file: F_DI_MISC,
    line: 63,
    origin: 'human',
    editedAt: '2026-09-05T13:00:00Z'
  },

  // --- TKey (8): all three strategies, several with multi-contexts ---
  {
    id: 'tk-01',
    kind: 'TKey',
    key: 'Scenarios/ColdStart/scene.nodes[3]',
    strategy: 'bare',
    source: 'Your three colonists awake after the long journey.',
    target: 'Ваши три поселенца приходят в себя после долгого пути.',
    status: 'translated',
    file: F_TK_SCENARIO,
    line: 14,
    origin: 'human',
    editedAt: '2026-09-19T09:10:00Z',
    contexts: [
      {
        node: 'li #text',
        file: F_TK_SCENARIO,
        line: 14,
        sourceValue: 'Your three colonists awake after the long journey.',
        sibling: '<li>WakeUp</li>'
      },
      {
        node: 'li #text',
        file: F_TK_SCENARIO,
        line: 18,
        sourceValue: 'Your three colonists awake after the long journey.',
        sibling: '<li>ChasePets</li>'
      }
    ]
  },
  {
    id: 'tk-02',
    kind: 'TKey',
    key: 'Scenarios/RichExplorer/scene.nodes[5]',
    strategy: '.slateRef',
    source: 'You arrive with generous supplies.',
    target: '',
    status: 'untranslated',
    file: F_TK_SCENARIO,
    line: 25,
    contexts: [
      {
        node: 'li slateRef="RichExplorer.intro"',
        file: F_TK_SCENARIO,
        line: 25,
        sourceValue: 'You arrive with generous supplies.',
        sibling: '<li>GiveItem silver</li>'
      }
    ]
  },
  {
    id: 'tk-03',
    kind: 'TKey',
    key: 'Quests/DealWithDevil/quest.name',
    strategy: '.value.slateRef',
    source: 'A deal with the devil',
    target: 'Сделка с дьяволом',
    status: 'pending_review',
    file: F_TK_QUEST,
    line: 6,
    origin: 'LLM',
    editedAt: '2026-09-21T18:05:00Z',
    contexts: [
      {
        node: 'quest name="quest.name" value.slateRef="dealDevil.title"',
        file: F_TK_QUEST,
        line: 6,
        sourceValue: 'A deal with the devil',
        sibling: '<quest name="quest.description">'
      }
    ]
  },
  {
    id: 'tk-04',
    kind: 'TKey',
    key: 'Scenarios/TribalBeat/scene.nodes[1]',
    strategy: 'bare',
    source: 'Your tribe must survive the coming winter.',
    target: '',
    status: 'untranslated',
    file: F_TK_SCENARIO,
    line: 40
  },
  {
    id: 'tk-05',
    kind: 'TKey',
    key: 'Quests/GrantArt/quest.description',
    strategy: '.slateRef',
    source: 'An eccentric collector asks for three sculptures.',
    target: '',
    status: 'todo',
    file: F_TK_QUEST,
    line: 21
  },
  {
    id: 'tk-06',
    kind: 'TKey',
    key: 'Scenarios/Cultivator/scene.nodes[8]',
    strategy: 'bare',
    source: 'You are a skilled farmer exiled to a lawless rimworld.',
    target: 'Вы искусный фермер, сосланный на беззаконный рим.',
    status: 'sourceChanged',
    file: F_TK_SCENARIO,
    line: 52,
    origin: 'human',
    editedAt: '2026-08-02T16:00:00Z'
  },
  {
    id: 'tk-07',
    kind: 'TKey',
    key: 'Quests/MoonMission/quest.name',
    strategy: '.value.slateRef',
    source: 'Mission to the moon',
    target: 'Миссия на луну',
    status: 'translated',
    file: F_TK_QUEST,
    line: 33,
    origin: 'human',
    editedAt: '2026-09-17T12:30:00Z'
  },
  {
    id: 'tk-08',
    kind: 'TKey',
    key: 'Scenarios/Transhumanist/scene.nodes[2]',
    strategy: 'bare',
    source: 'Flesh is a liability. Upgrade everything.',
    target: '',
    status: 'untranslated',
    file: F_TK_SCENARIO,
    line: 61
  }
];

export const MOCK_ERROR_CODE = 'ERR_MOCK_PROVIDER_001';
export const MOCK_ERROR_RAW =
  'MockProviderFailure: simulated backend outage for dev-panel demo (no real service was contacted).';
