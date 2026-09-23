// Static mock dataset: ~60 fictional RimWorld-like entries covering all three
// record kinds (Keyed / DefInjected / TKey) and all six statuses. The corpus is
// deliberately representative (mandate §27): long texts, {0}/{PAWN_nameDef}
// placeholders, Cyrillic + CJK + German sources, sourceChanged with previous
// source, validation problems (placeholder mismatch, glossary), TODO marks,
// imported/TM/AI origins, two TKey multi-context entries and one ambiguous
// source. Zero backend calls — this is the only data source of the mock phase.

import type { Entry } from './types';

const F_KEYED = 'Languages/English/Keyed/Misc_Gameplay.xml';
const F_KEYED_UI = 'Languages/English/Keyed/Dialogs.xml';
const F_DI_WEAPONS = 'Languages/English/DefInjected/ThingDef/Things_Weapons.xml';
const F_DI_APPAREL = 'Languages/English/DefInjected/ThingDef/Things_Apparel.xml';
const F_DI_BUILDINGS = 'Languages/English/DefInjected/ThingDef/Things_Buildings.xml';
const F_DI_MISC = 'Languages/English/DefInjected/Misc/Defs_Misc.xml';
const F_TK_SCENARIO = 'TKey/Scenarios/Scenarios.xml';
const F_TK_QUEST = 'TKey/Quests/Quests.xml';

export const mockEntries: Entry[] = [
  // ---------------------------------------------------------------- Keyed (22)
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
    editedAt: '2026-09-20T10:14:00Z',
    usages: ['Список писем', 'История событий'],
    history: [
      { at: '2026-09-20T10:14:00Z', action: 'edited', origin: 'human' },
      { at: '2026-09-01T08:00:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 100%' }
    ]
  },
  {
    id: 'keyed-02',
    kind: 'Keyed',
    key: 'AncientComplexWarning',
    source:
      'Warning: the ancient complex is unstable. Structural collapses may occur while your colonists explore its deepest levels. Bring construction materials and medical supplies.',
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
    note: 'Проверить падеж имени после подстановки: «Изгнать Иван?» — нужен творительный.'
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
    editedAt: '2026-09-19T15:30:00Z',
    history: [{ at: '2026-09-19T15:30:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 100%' }]
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
    editedAt: '2026-08-11T12:00:00Z',
    sourcePrev: 'Raid incoming!',
    issues: [
      {
        kind: 'placeholder_mismatch',
        severity: 'error',
        message: 'В цели нет плейсхолдеров {ENEMYPAWN_nameFull} и {ENEMYFACTION_name} из источника.'
      }
    ],
    history: [
      { at: '2026-08-11T12:00:00Z', action: 'source_changed', detail: '«Raid incoming!» → плейсхолдеры фракции и имени' },
      { at: '2026-07-02T09:00:00Z', action: 'edited', origin: 'human' }
    ]
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
    editedAt: '2026-09-15T09:41:00Z',
    usages: ['Меню торговли']
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
    editedAt: '2026-09-21T17:22:00Z',
    issues: [
      {
        kind: 'ai_concern',
        severity: 'warning',
        message: '«Психический» может читаться как медицинский термин; в контексте ритуала проверь трактовку.'
      }
    ],
    suggestions: [{ source: 'LLM', text: 'Начать пси-ритуал' }],
    history: [{ at: '2026-09-21T17:22:00Z', action: 'ai_draft', origin: 'LLM', model: 'glm-4.7' }]
  },
  {
    id: 'keyed-11',
    kind: 'Keyed',
    key: 'AlertHypothermia',
    source: 'Hypothermia',
    target: '',
    status: 'todo',
    file: F_KEYED,
    line: 95,
    note: 'Сверить с Medical glossary: «переохлаждение».'
  },
  {
    id: 'keyed-12',
    kind: 'Keyed',
    key: 'PrisonerEscape',
    source: '{0} is attempting to escape from {1}.',
    target: '',
    status: 'untranslated',
    file: F_KEYED_UI,
    line: 103,
    note: 'Два плейсхолдера: имя и помещение.'
  },
  {
    id: 'keyed-13',
    kind: 'Keyed',
    key: 'RoomStatsInsulated',
    source:
      'A well-insulated room retains heat far longer than an ordinary one. In cold snaps this difference decides whether your crops and colonists survive the night.',
    target:
      'Хорошо утеплённая комната удерживает тепло значительно дольше обычной. В холода именно это решает, переживут ли урожай и поселенцы ночь.',
    status: 'translated',
    file: F_KEYED,
    line: 112,
    origin: 'human',
    editedAt: '2026-09-16T14:05:00Z',
    issues: [
      {
        kind: 'glossary',
        severity: 'warning',
        message: 'Глоссарий: термин «colonist» переведён как «поселенцы», в проекте принят «колонисты».'
      }
    ]
  },
  {
    id: 'keyed-14',
    kind: 'Keyed',
    key: 'SleepAcceleratorDesc',
    source:
      'The sleep accelerator compresses restful cycles, letting a sleeper recover a full night of rest in a fraction of the time. Prolonged use causes vivid dreams.',
    target: '',
    status: 'untranslated',
    file: F_KEYED,
    line: 121
  },
  {
    id: 'keyed-15',
    kind: 'Keyed',
    key: 'TradeCurrencySilver',
    source: 'Silver is the standard trade currency of the rim.',
    target: 'Серебро — стандартная торговая валюта обочины.',
    status: 'translated',
    file: F_KEYED,
    line: 133,
    origin: 'imported',
    editedAt: '2026-09-01T08:45:00Z',
    history: [{ at: '2026-09-01T08:45:00Z', action: 'imported', detail: 'Vanilla-RU 2.1' }]
  },
  {
    id: 'keyed-16',
    kind: 'Keyed',
    key: 'BillRepeatCount',
    source: 'Repeat count: {0}',
    target: 'Количество повторений: {0}',
    status: 'translated',
    file: F_KEYED,
    line: 140,
    origin: 'TM',
    editedAt: '2026-09-12T11:00:00Z'
  },
  {
    id: 'keyed-17',
    kind: 'Keyed',
    key: 'MoodDebuffTitle',
    source: '{PAWN_nameDef} is in a bad mood',
    target: '{PAWN_nameDef} в плохой морали',
    status: 'pending_review',
    file: F_KEYED_UI,
    line: 150,
    origin: 'LLM',
    editedAt: '2026-09-21T17:35:00Z',
    issues: [
      {
        kind: 'glossary',
        severity: 'warning',
        message: 'Глоссарий: «mood» → «настроение»; «мораль» — другой термин проекта.'
      }
    ],
    history: [{ at: '2026-09-21T17:35:00Z', action: 'ai_draft', origin: 'LLM', model: 'glm-4.7' }]
  },
  {
    id: 'keyed-18',
    kind: 'Keyed',
    key: 'ConfirmDelete',
    source: 'Delete {0}?',
    target: 'Удалить {0}?',
    status: 'translated',
    file: F_KEYED_UI,
    line: 162,
    origin: 'human',
    editedAt: '2026-09-14T10:30:00Z'
  },
  {
    id: 'keyed-19',
    kind: 'Keyed',
    key: 'BuildPlacementLeft',
    source: 'Left',
    target: 'Слева',
    status: 'translated',
    file: F_KEYED_UI,
    line: 171,
    origin: 'human',
    editedAt: '2026-09-08T16:20:00Z',
    usages: ['Панель строительства', 'Контекстное меню приказа'],
    note: 'Источник неоднозначен: в другой подсказке «Left» значит «осталось».',
    issues: [
      {
        kind: 'ambiguity',
        severity: 'warning',
        message: 'Неоднозначный источник: «слева» или «осталось»? Контекст — размещение постройки.'
      }
    ]
  },
  {
    id: 'keyed-20',
    kind: 'Keyed',
    key: 'ResearchFinishedMessage',
    source: 'Research finished: {0}',
    target: 'Исследование завершено: {0}',
    status: 'translated',
    file: F_KEYED,
    line: 178,
    origin: 'TM',
    editedAt: '2026-09-10T09:00:00Z',
    history: [{ at: '2026-09-10T09:00:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 98%' }]
  },
  {
    id: 'keyed-21',
    kind: 'Keyed',
    key: 'KaizenMod.Welcome',
    source: '改善モジュールへようこそ。快適な植民地生活をお過ごしください。',
    target: 'Добро пожаловать в мод улучшений. Пусть колониальная жизнь будет в радость.',
    status: 'translated',
    file: 'Languages/Japanese/Keyed/Kaizen.xml',
    line: 3,
    origin: 'imported',
    editedAt: '2026-09-05T12:00:00Z',
    note: 'Мод изначально на японском; английской строки в исходнике нет.',
    usages: ['Экран приветствия мода'],
    history: [{ at: '2026-09-05T12:00:00Z', action: 'imported', detail: 'Kaizen-RU 0.9' }]
  },
  {
    id: 'keyed-22',
    kind: 'Keyed',
    key: 'FahrzeugMod.Start',
    source: 'Willkommen beim Fahrzeug-Mod. Gute Fahrt auf dem Rim!',
    target: 'Добро пожаловать в мод транспорта. Удачной езды по обочине!',
    status: 'translated',
    file: 'Languages/German/Keyed/Fahrzeug.xml',
    line: 5,
    origin: 'imported',
    editedAt: '2026-09-06T10:00:00Z',
    history: [{ at: '2026-09-06T10:00:00Z', action: 'imported', detail: 'Fahrzeug-RU 1.3' }]
  },

  // ----------------------------------------------------------- DefInjected (27)
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
    source:
      'A gas-operated military rifle. Good middle ground between range, damage and accuracy. Standard issue for outlander soldiers across the rim.',
    target:
      'Армейская винтовка с газоотводной автоматикой. Хороший баланс дальности, урона и точности. Штатное оружие чужеземных солдат по всей обочине.',
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
    line: 19,
    note: 'Решить: «длинный меч» или «лонгсворд» — см. глоссарий оружия.'
  },
  {
    id: 'di-03b',
    kind: 'DefInjected',
    key: 'MeleeWeapon_LongSword.description',
    source:
      'A long, double-edged blade designed for reach and leverage. In trained hands it keeps enemies at a distance where their knives are useless.',
    target: '',
    status: 'untranslated',
    file: F_DI_WEAPONS,
    line: 24
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
    editedAt: '2026-09-08T14:12:00Z',
    history: [{ at: '2026-09-08T14:12:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 100%' }]
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
    editedAt: '2026-07-30T10:00:00Z',
    sourcePrev: 'sarcophagus',
    issues: [
      {
        kind: 'wordinfo',
        severity: 'warning',
        message: 'WordInfo: у «саркофаг» не указан род — прилагательные в подсказках соберутся неверно.'
      }
    ],
    history: [{ at: '2026-07-30T10:00:00Z', action: 'source_changed', detail: 'sarcophagus → ancient sarcophagus' }]
  },
  {
    id: 'di-06b',
    kind: 'DefInjected',
    key: 'Building_Sarcophagus.description',
    source:
      'An ornate stone coffin, carved with figures of the dead. Ancient builders sealed their honored ones inside with careful masonry — and sometimes other things.',
    target: '',
    status: 'untranslated',
    file: F_DI_MISC,
    line: 19
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
    editedAt: '2026-09-01T08:45:00Z',
    history: [{ at: '2026-09-01T08:45:00Z', action: 'imported', detail: 'Vanilla-RU 2.1' }]
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
    editedAt: '2026-09-21T17:40:00Z',
    issues: [
      {
        kind: 'ai_concern',
        severity: 'warning',
        message: 'Идиома «зелёные пальцы» в русском употребима; проверь, звучит ли естественно как черта.'
      }
    ],
    history: [{ at: '2026-09-21T17:40:00Z', action: 'ai_draft', origin: 'LLM', model: 'glm-4.7' }]
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
  {
    id: 'di-12',
    kind: 'DefInjected',
    key: 'Building_SolarGenerator.label',
    source: 'solar generator',
    target: 'солнечный генератор',
    status: 'translated',
    file: F_DI_BUILDINGS,
    line: 4,
    origin: 'human',
    editedAt: '2026-09-13T09:55:00Z',
    usages: ['Строительство → Электроэнергия']
  },
  {
    id: 'di-12b',
    kind: 'DefInjected',
    key: 'Building_SolarGenerator.description',
    source:
      'A platform of photovoltaic panels that converts sunlight into electricity. Output drops to zero during eclipses and solar flares, so keep backup power.',
    target:
      'Платформа из фотоэлектрических панелей, превращающая солнечный свет в электричество. Во время затмений и вспышек выдача падает до нуля — держите резерв.',
    status: 'translated',
    file: F_DI_BUILDINGS,
    line: 9,
    origin: 'human',
    editedAt: '2026-09-13T10:05:00Z'
  },
  {
    id: 'di-13',
    kind: 'DefInjected',
    key: 'Gun_ChargeRifle.label',
    source: 'charge rifle',
    target: 'энергетическая винтовка',
    status: 'pending_review',
    file: F_DI_WEAPONS,
    line: 33,
    origin: 'LLM',
    editedAt: '2026-09-21T18:00:00Z',
    issues: [
      {
        kind: 'glossary',
        severity: 'warning',
        message: 'Глоссарий: «charge» → «зарядовое»; «энергетическое» — термин другого мода.'
      }
    ],
    history: [{ at: '2026-09-21T18:00:00Z', action: 'ai_draft', origin: 'LLM', model: 'glm-4.7' }]
  },
  {
    id: 'di-13b',
    kind: 'DefInjected',
    key: 'Gun_ChargeRifle.description',
    source:
      'A pulse-charged energy weapon. Its bolts are less damaging than bullets, but they pierce armor that would turn aside ordinary rounds.',
    target:
      'Импульсное зарядовое оружие. Выстрелы наносят меньше урона, чем пули, но пробивают броню, от которой обычные патроны отскакивают.',
    status: 'translated',
    file: F_DI_WEAPONS,
    line: 38,
    origin: 'human',
    editedAt: '2026-09-11T15:40:00Z'
  },
  {
    id: 'di-14',
    kind: 'DefInjected',
    key: 'Animal_Wolf.label',
    source: 'wolf',
    target: 'волк',
    status: 'translated',
    file: F_DI_MISC,
    line: 71,
    origin: 'TM',
    editedAt: '2026-09-09T12:10:00Z',
    suggestions: [{ source: 'TM', similarity: 97, text: 'волк' }],
    history: [{ at: '2026-09-09T12:10:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 97%' }]
  },
  {
    id: 'di-15',
    kind: 'DefInjected',
    key: 'Animal_Wolf.description',
    source:
      'A large predatory canine that hunts in packs. Wolves can be tamed, but a hungry pack near your livestock is a problem you will have to solve.',
    target: '',
    status: 'untranslated',
    file: F_DI_MISC,
    line: 76
  },
  {
    id: 'di-16',
    kind: 'DefInjected',
    key: 'Plant_Haygrass.label',
    source: 'haygrass',
    target: 'сено-трава',
    status: 'translated',
    file: F_DI_MISC,
    line: 88,
    origin: 'human',
    editedAt: '2026-09-03T08:00:00Z',
    issues: [
      {
        kind: 'glossary',
        severity: 'warning',
        message: 'Глоссарий: «hay» → «сено»; составное «сено-трава» через дефис не принято в проекте.'
      }
    ],
    suggestions: [{ source: 'glossary', term: 'hay → сено', text: 'сенозлак' }]
  },
  {
    id: 'di-17',
    kind: 'DefInjected',
    key: 'Trait_NightOwl.degreeData.label',
    source: 'night owl',
    target: '',
    status: 'todo',
    file: F_DI_MISC,
    line: 100,
    note: 'Проверить: «сова» уже занята другим трейтом?'
  },
  {
    id: 'di-18',
    kind: 'DefInjected',
    key: 'ResearchTab_MultiAnalyzer.label',
    source: 'multi-analyzer',
    target: '',
    status: 'untranslated',
    file: F_DI_MISC,
    line: 111
  },
  {
    id: 'di-19',
    kind: 'DefInjected',
    key: 'Hediff_BionicEye.label',
    source: 'bionic eye',
    target: 'бионический глаз',
    status: 'translated',
    file: F_DI_MISC,
    line: 118,
    origin: 'TM',
    editedAt: '2026-09-07T14:30:00Z',
    suggestions: [{ source: 'TM', similarity: 96, text: 'бионический глаз' }],
    history: [{ at: '2026-09-07T14:30:00Z', action: 'tm_match', origin: 'TM', detail: 'Core 1.5, 96%' }]
  },
  {
    id: 'di-20',
    kind: 'DefInjected',
    key: 'Building_MarbleColumn.label',
    source: 'marble column',
    target: 'мраморная колонна',
    status: 'translated',
    file: F_DI_BUILDINGS,
    line: 21,
    origin: 'human',
    editedAt: '2026-09-04T11:15:00Z'
  },
  {
    id: 'di-21',
    kind: 'DefInjected',
    key: 'Faction_Outlander.label',
    source: 'outlander union',
    target: 'союз чужеземцев',
    status: 'sourceChanged',
    file: F_DI_MISC,
    line: 129,
    origin: 'human',
    editedAt: '2026-08-19T13:25:00Z',
    sourcePrev: 'outlander union',
    history: [{ at: '2026-08-19T13:25:00Z', action: 'source_changed', detail: '1.5 → 1.6: сменился ключ Def' }]
  },
  {
    id: 'di-22',
    kind: 'DefInjected',
    key: 'Apparel_Parka.label',
    source: 'parka',
    target: 'парка',
    status: 'translated',
    file: F_DI_APPAREL,
    line: 16,
    origin: 'imported',
    editedAt: '2026-09-02T09:20:00Z',
    history: [{ at: '2026-09-02T09:20:00Z', action: 'imported', detail: 'Vanilla-RU 2.1' }]
  },
  {
    id: 'di-22b',
    kind: 'DefInjected',
    key: 'Apparel_Parka.description',
    source:
      'A heavy insulated parka with a deep hood. It turns a lethal blizzard into an unpleasant walk, at the cost of looking like a walking tent.',
    target:
      'Тяжёлая утеплённая парка с глубоким капюшоном. Превращает смертельную метель в неприятную прогулку ценой вида ходячей палатки.',
    status: 'translated',
    file: F_DI_APPAREL,
    line: 21,
    origin: 'human',
    editedAt: '2026-09-12T16:45:00Z'
  },
  {
    id: 'di-23',
    kind: 'DefInjected',
    key: 'Building_HospitalBed.label',
    source: 'hospital bed',
    target: 'больничная койка',
    status: 'translated',
    file: F_DI_BUILDINGS,
    line: 34,
    origin: 'human',
    editedAt: '2026-09-15T08:05:00Z',
    usages: ['Строительство → Здравоохранение', 'Меню медицинского режима']
  },
  {
    id: 'di-23b',
    kind: 'DefInjected',
    key: 'Building_HospitalBed.description',
    source:
      'A bed fitted with monitoring equipment. Patients resting here recover faster and suffer fewer complications during surgery.',
    target: '',
    status: 'untranslated',
    file: F_DI_BUILDINGS,
    line: 39
  },
  {
    id: 'di-24',
    kind: 'DefInjected',
    key: 'Hediff_Bruise.label',
    source: 'bruise',
    target: 'синяк',
    status: 'translated',
    file: F_DI_MISC,
    line: 140,
    origin: 'human',
    editedAt: '2026-09-09T17:00:00Z'
  },

  // ---------------------------------------------------------------- TKey (12)
  {
    id: 'tk-01',
    kind: 'TKey',
    key: 'Scenarios/ColdStart/scene.nodes[3]',
    strategy: 'bare',
    source: 'Your three colonists awake after the long journey.',
    target: 'Ваши три колониста приходят в себя после долгого пути.',
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
    ],
    history: [{ at: '2026-09-19T09:10:00Z', action: 'edited', origin: 'human' }]
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
    ],
    issues: [
      {
        kind: 'ai_concern',
        severity: 'warning',
        message: 'Каламбур «deal/devil» потерян; допустимо, но проверь тон квеста.'
      }
    ],
    history: [{ at: '2026-09-21T18:05:00Z', action: 'ai_draft', origin: 'LLM', model: 'glm-4.7' }]
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
    source: 'An eccentric collector asks for three sculptures. He insists on marble.',
    target: '',
    status: 'todo',
    file: F_TK_QUEST,
    line: 21,
    note: 'Уточнить у автора: коллекционер требует мрамор или любой камень?'
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
    editedAt: '2026-08-02T16:00:00Z',
    sourcePrev: 'You are a skilled farmer exiled to a rimworld.',
    history: [{ at: '2026-08-02T16:00:00Z', action: 'source_changed', detail: 'rimworld → lawless rimworld' }]
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
    editedAt: '2026-09-17T12:30:00Z',
    suggestions: [{ source: 'TM', similarity: 91, text: 'Экспедиция на луну' }]
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
  },
  {
    id: 'tk-09',
    kind: 'TKey',
    key: 'Quests/CrashLanding/scene.nodes[2]',
    strategy: 'bare',
    source: 'Your shuttle breaks apart in the upper atmosphere.',
    target: '',
    status: 'untranslated',
    file: F_TK_QUEST,
    line: 48,
    contexts: [
      {
        node: 'li #text',
        file: F_TK_QUEST,
        line: 48,
        sourceValue: 'Your shuttle breaks apart in the upper atmosphere.',
        sibling: '<li>CameraShake</li>'
      },
      {
        node: 'li #text',
        file: F_TK_QUEST,
        line: 55,
        sourceValue: 'Your shuttle breaks apart in the upper atmosphere.',
        sibling: '<li>SpawnWreckage</li>'
      }
    ],
    note: 'Два узла — текст и реплика выжившего: падеж может отличаться.'
  },
  {
    id: 'tk-10',
    kind: 'TKey',
    key: 'Quests/AncientArmory/quest.description',
    strategy: '.slateRef',
    source:
      'Rumors speak of an armory sealed since the cataclysm. Guardians still patrol its corridors, and the doors answer only to those who carry the old sigils.',
    target: '',
    status: 'untranslated',
    file: F_TK_QUEST,
    line: 70
  },
  {
    id: 'tk-11',
    kind: 'TKey',
    key: 'Scenarios/NakedBrutality/scene.nodes[1]',
    strategy: 'bare',
    source: 'You arrive with nothing but the fire in your chest.',
    target: 'Вы прибываете с одним лишь огнём в груди.',
    status: 'sourceChanged',
    file: F_TK_SCENARIO,
    line: 74,
    origin: 'human',
    editedAt: '2026-08-25T10:40:00Z',
    sourcePrev: 'You arrive with nothing but the clothes on your back.',
    history: [{ at: '2026-08-25T10:40:00Z', action: 'source_changed', detail: 'clothes on your back → fire in your chest' }]
  },
  {
    id: 'tk-12',
    kind: 'TKey',
    key: 'Quests/MechhiveAttack/quest.name',
    strategy: '.value.slateRef',
    source: 'Mechhive assault',
    target: 'Штурм меканоидного улья',
    status: 'translated',
    file: F_TK_QUEST,
    line: 92,
    origin: 'TM',
    editedAt: '2026-09-18T13:00:00Z',
    history: [{ at: '2026-09-18T13:00:00Z', action: 'tm_match', origin: 'TM', detail: 'Biotech-RU, 94%' }]
  }
];

export const MOCK_ERROR_CODE = 'ERR_MOCK_PROVIDER_001';
export const MOCK_ERROR_RAW =
  'MockProviderFailure: simulated backend outage for dev-panel demo (no real service was contacted).';
