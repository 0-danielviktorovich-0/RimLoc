// RimLoc-owned SYNTHETIC source fixtures (W7, mock/pre-freeze). Every file
// body, line number, candidate and provenance fact here is demo DATA — the
// real resolver/scanner/precedence stays backend-owned (J); the GUI renders
// facts and computes nothing. Scenario ids follow SOURCE_INSPECTOR_MANDATE
// §18 and are exported for the W6 scenario browser.
import type {
  SourceBrowserData,
  SourceEntryData,
  SourceFile,
  SourceFileContent,
  SourceNodeSpan,
  SourceScenario
} from './types';

const MOD_NAME = 'Demo TestMod';
const MOD_ID = 'rimloc.demo.testmod';
const PKG_ID = 'rimloc.demo.testmod';

// ---------------------------------------------------------------- file bodies
// Line numbers in comments are the 1-based truth the fixtures guarantee.

const KEYED_GAMEPLAY_PATH = 'Languages/English/Keyed/Misc_Gameplay.xml';
const KEYED_GAMEPLAY = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<!-- RimLoc-owned synthetic demo fixture. Not a real mod file. -->', // 2
  '<LanguageData>', // 3
  '', // 4
  '  <!-- ============ Messages ============ -->', // 5
  '  <!-- Demo: shipped letter strings for the colony start. -->', // 6
  '', // 7
  '  <!-- Arrived-letter header shown in the letter stack. -->', // 8
  '  <!-- {0} is the day number; placeholders must survive translation. -->', // 9
  '', // 10
  '  <!-- node: MessageLetterArrived -->', // 11
  '  <MessageLetterArrived>{0}: A letter has arrived.</MessageLetterArrived>', // 12
  '', // 13
  '</LanguageData>' // 14
];

const DI_WEAPONS_PATH = 'Languages/English/DefInjected/ThingDef/Things_Weapons.xml';
const DI_WEAPONS = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<RimLoc-Demo-DefInjected><!-- RimLoc-owned synthetic fixture -->', // 2
  '  <Gun_AssaultRifle.label>assault rifle</Gun_AssaultRifle.label>', // 3
  '', // 4
  '  <!-- description paragraph shown on the info card -->', // 5
  '', // 6
  '  <!-- node: Gun_AssaultRifle.description -->', // 7
  '  <Gun_AssaultRifle.description>A gas-operated military rifle. Good middle ground between range, damage and accuracy. Standard issue for outlander soldiers across the rim.</Gun_AssaultRifle.description>', // 8
  '', // 9
  '</RimLoc-Demo-DefInjected>' // 10
];

const TK_SCENARIO_PATH = 'Defs/ScenarioDefs/Scenarios_Demo.xml';
const TK_SCENARIO = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<!-- RimLoc-owned synthetic demo fixture. Not a real mod file. -->', // 2
  '<Defs>', // 3
  '', // 4
  '  <ScenarioDef>', // 5
  '    <defName>ColdStart</defName>', // 6
  '    <label>Cold start</label>', // 7
  '    <description>Three colonists, one broken ship, winter is close.</description>', // 8
  '    <scenario>', // 9
  '      <parts>', // 10
  '', // 11
  '        <!-- TKey text nodes: two usages share ONE identity -->', // 12
  '', // 13
  '        <li>Your three colonists awake after the long journey.</li>', // 14
  '', // 15
  '        <!-- second occurrence: same identity, another position -->', // 16
  '', // 17
  '        <li>Your three colonists awake after the long journey.</li>', // 18
  '', // 19
  '      </parts>', // 20
  '    </scenario>', // 21
  '  </ScenarioDef>', // 22
  '', // 23
  '</Defs>' // 24
];

// version-override scenario: Common root is ACTIVE, 1.5 root is SHADOWED.
const ITEMS_DEMO_COMMON_PATH = 'Common/Defs/ThingDefs_Items/Items_Demo.xml';
const ITEMS_DEMO_COMMON = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<!-- RimLoc-owned synthetic demo fixture. Not a real mod file. -->', // 2
  '<RimLoc-Demo-Defs>', // 3
  '', // 4
  '  <!-- Melee weapons demo subset. -->', // 5
  '  <!-- Version roots: Common (ACTIVE) vs 1.5 (SHADOWED). -->', // 6
  '', // 7
  '  <ThingDef ParentName="BaseWeapon">', // 8
  '    <defName>MeleeWeapon_LongSword</defName>', // 9
  '', // 10
  '    <!-- ---------------------------------------- -->', // 11
  '    <!-- Label shown in menus, bills, stockpiles. -->', // 12
  '    <!-- ACTIVE root: Common (this file).         -->', // 13
  '    <!-- Shadowed twin: 1.5/Defs (older wording). -->', // 14
  '    <!-- ---------------------------------------- -->', // 15
  '', // 16
  '    <!-- node: MeleeWeapon_LongSword.label -->', // 17
  '    <!-- column of the text node is NOT guaranteed here -->', // 18
  '    <label>longsword</label>', // 19
  '    <description>A heavy blade with a long reach and a lethal point.</description>', // 20
  '  </ThingDef>', // 21
  '', // 22
  '</RimLoc-Demo-Defs>' // 23
];

const ITEMS_DEMO_15_PATH = '1.5/Defs/ThingDefs_Items/Items_Demo.xml';
const ITEMS_DEMO_15 = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<!-- RimLoc-owned synthetic demo fixture (1.5 root, SHADOWED). -->', // 2
  '<RimLoc-Demo-Defs>', // 3
  '', // 4
  '  <ThingDef ParentName="BaseWeapon">', // 5
  '    <defName>MeleeWeapon_LongSword</defName>', // 6
  '', // 7
  '    <!-- older 1.5 wording; kept for rollbacks -->', // 8
  '', // 9
  '    <!-- node: MeleeWeapon_LongSword.label -->', // 10
  '    <label>long sword</label>', // 11
  '    <description>A heavy blade with a long reach and a lethal point.</description>', // 12
  '  </ThingDef>', // 13
  '', // 14
  '</RimLoc-Demo-Defs>' // 15
];

// generated-output scenario: RimLoc OUTPUT (never a second source of truth).
const GENERATED_WEAPONS_PATH = 'Languages/Russian/DefInjected/ThingDef/Things_Weapons.xml';
const GENERATED_WEAPONS = [
  '<?xml version="1.0" encoding="utf-8"?>', // 1
  '<!-- Generated by RimLoc (demo). RimLoc output is a build artifact. -->', // 2
  '<RimLoc-Demo-DefInjected>', // 3
  '', // 4
  '  <Gun_AssaultRifle.label>штурмовая винтовка</Gun_AssaultRifle.label>', // 5
  '', // 6
  '  <Gun_AssaultRifle.description>Армейская винтовка с газоотводной автоматикой. Хороший баланс дальности, урона и точности. Штатное оружие чужеземных солдат по всей обочине.</Gun_AssaultRifle.description>', // 7
  '', // 8
  '</RimLoc-Demo-DefInjected>' // 9
];

function file(
  path: string,
  displayPath: string,
  kind: SourceFileContent['kind'],
  lines: string[],
  spans: SourceNodeSpan[],
  breadcrumb: string[],
  versionLabel?: string,
  loadFolder?: string
): SourceFileContent {
  return {
    path,
    displayPath,
    kind,
    lines,
    lineNumbersGuaranteed: true,
    spans,
    breadcrumb,
    versionLabel,
    loadFolder
  };
}

// ------------------------------------------------------------------- scenarios

const simpleEntries: Record<string, SourceEntryData> = {
  'keyed-01': {
    entryId: 'keyed-01',
    usages: [
      {
        role: 'primary',
        effective: true,
        location: {
          path: KEYED_GAMEPLAY_PATH,
          displayPath: KEYED_GAMEPLAY_PATH,
          line: 12,
          column: null, // column is NOT guaranteed by this fixture — honest null
          nodePath: 'LanguageData/MessageLetterArrived'
        },
        provenance: [
          { kind: 'definition', detail: 'Keyed string defined in the mod language folder.' },
          { kind: 'keyed-last-wins', detail: 'Keyed family: last file declaring the key wins.' }
        ]
      }
    ],
    excerpt: {
      defName: '— (Keyed)',
      field: 'MessageLetterArrived',
      related: [
        { labelKey: 'source.excerpt.sibling', value: 'MessageColonyChangeLand' },
        { labelKey: 'source.excerpt.folder', value: 'Languages/English/Keyed' }
      ]
    }
  },
  'di-02': {
    entryId: 'di-02',
    usages: [
      {
        role: 'primary',
        effective: true,
        location: {
          path: DI_WEAPONS_PATH,
          displayPath: DI_WEAPONS_PATH,
          line: 8,
          column: 31,
          nodePath: 'RimLoc-Demo-DefInjected/Gun_AssaultRifle/description'
        },
        provenance: [
          { kind: 'definition', detail: 'DefInjected translation slot for ThingDef field.' },
          { kind: 'first-file-wins', detail: 'DefInjected family: first file declaring the path wins.' }
        ]
      }
    ],
    excerpt: {
      defName: 'Gun_AssaultRifle',
      field: 'description',
      related: [
        { labelKey: 'source.excerpt.label', value: 'assault rifle' },
        { labelKey: 'source.excerpt.defType', value: 'ThingDef' },
        { labelKey: 'source.excerpt.folder', value: 'DefInjected/ThingDef' }
      ]
    }
  }
};

const tkeyEntries: Record<string, SourceEntryData> = {
  'tk-01': {
    entryId: 'tk-01',
    usages: [
      {
        role: 'primary',
        effective: true,
        location: {
          path: TK_SCENARIO_PATH,
          displayPath: TK_SCENARIO_PATH,
          line: 14,
          column: 9,
          nodePath: 'Defs/ScenarioDef[defName=ColdStart]/scenario/parts/li[1]'
        },
        provenance: [
          { kind: 'definition', detail: 'TKey-attributed text node (system since RimWorld 1.1).' },
          { kind: 'first-file-wins', detail: 'Primary usage: first li in document order.' }
        ]
      },
      {
        role: 'other',
        effective: true,
        location: {
          path: TK_SCENARIO_PATH,
          displayPath: TK_SCENARIO_PATH,
          line: 18,
          column: 9,
          nodePath: 'Defs/ScenarioDef[defName=ColdStart]/scenario/parts/li[2]'
        },
        provenance: [
          { kind: 'definition', detail: 'Same TKey identity used again later in the list.' }
        ]
      }
    ],
    excerpt: {
      defName: 'ColdStart',
      field: 'scene.nodes[3] (TKey)',
      related: [
        { labelKey: 'source.excerpt.defType', value: 'ScenarioDef' },
        { labelKey: 'source.excerpt.sibling', value: '<li>WakeUp</li>' },
        { labelKey: 'source.excerpt.usages', value: '2' }
      ]
    }
  }
};

const versionOverrideEntries: Record<string, SourceEntryData> = {
  'di-03': {
    entryId: 'di-03',
    usages: [
      {
        role: 'primary',
        effective: true,
        location: {
          path: ITEMS_DEMO_COMMON_PATH,
          displayPath: ITEMS_DEMO_COMMON_PATH,
          line: 19,
          column: null, // nullable honesty: fixture does not guarantee the column
          nodePath: 'RimLoc-Demo-Defs/ThingDef[defName=MeleeWeapon_LongSword]/label'
        },
        provenance: [
          { kind: 'version-selected', detail: 'Target version 1.6 selects the Common root.' },
          { kind: 'loadfolders', detail: 'LoadFolders branch: Common ships for every version.' }
        ]
      },
      {
        role: 'other',
        effective: false, // shadowed twin — diagnostic evidence only
        location: {
          path: ITEMS_DEMO_15_PATH,
          displayPath: ITEMS_DEMO_15_PATH,
          line: 11,
          column: null,
          nodePath: 'RimLoc-Demo-Defs/ThingDef[defName=MeleeWeapon_LongSword]/label'
        },
        provenance: [
          { kind: 'version-selected', detail: '1.5 root loses to Common for the selected version.' },
          { kind: 'loadfolders', detail: 'Version-specific branch: loaded only for 1.5.' }
        ]
      }
    ],
    excerpt: {
      defName: 'MeleeWeapon_LongSword',
      field: 'label',
      related: [
        { labelKey: 'source.excerpt.defType', value: 'ThingDef' },
        { labelKey: 'source.excerpt.shadowed', value: ITEMS_DEMO_15_PATH }
      ]
    }
  }
};

const browser: SourceBrowserData = {
  modId: MOD_ID,
  modName: MOD_NAME,
  packageId: PKG_ID,
  versions: [
    { label: '1.4', selected: false },
    { label: '1.5', selected: false },
    { label: '1.6', selected: true }
  ],
  loadFolders: [
    { label: 'Common', selected: true },
    { label: '1.5', selected: false },
    { label: '1.6', selected: true }
  ],
  categories: [
    {
      name: 'Defs/ScenarioDefs',
      files: [{ path: TK_SCENARIO_PATH, displayPath: TK_SCENARIO_PATH }]
    },
    {
      name: 'Defs/ThingDefs_Items',
      files: [
        { path: ITEMS_DEMO_COMMON_PATH, displayPath: ITEMS_DEMO_COMMON_PATH },
        { path: ITEMS_DEMO_15_PATH, displayPath: ITEMS_DEMO_15_PATH }
      ]
    },
    {
      name: 'Languages/English/Keyed',
      files: [{ path: KEYED_GAMEPLAY_PATH, displayPath: KEYED_GAMEPLAY_PATH }]
    },
    {
      name: 'Languages/English/DefInjected/ThingDef',
      files: [{ path: DI_WEAPONS_PATH, displayPath: DI_WEAPONS_PATH }]
    }
  ],
  candidates: [
    {
      path: ITEMS_DEMO_COMMON_PATH,
      displayPath: ITEMS_DEMO_COMMON_PATH,
      status: 'active',
      reasonKey: 'source.candidate.reason.activeCommon',
      versionLabel: '1.6',
      loadFolder: 'Common'
    },
    {
      path: ITEMS_DEMO_15_PATH,
      displayPath: ITEMS_DEMO_15_PATH,
      status: 'shadowed',
      reasonKey: 'source.candidate.reason.shadowedVersion',
      versionLabel: '1.5',
      loadFolder: '1.5'
    }
  ]
};

function scenarioBase(): Pick<SourceScenario, 'browser' | 'compare' | 'externalChange'> {
  return { browser, compare: {}, externalChange: undefined };
}

const filesFor = (extra: Record<string, SourceFile> = {}): Record<string, SourceFile> => ({
  [KEYED_GAMEPLAY_PATH]: file(
    KEYED_GAMEPLAY_PATH,
    KEYED_GAMEPLAY_PATH,
    'effective',
    KEYED_GAMEPLAY,
    [{ nodeId: 'keyed-01', entryId: 'keyed-01', start: 12, end: 12 }],
    [MOD_NAME, '1.6 · Common', 'Languages/English/Keyed', 'Misc_Gameplay.xml'],
    '1.6',
    'Common'
  ),
  [DI_WEAPONS_PATH]: file(
    DI_WEAPONS_PATH,
    DI_WEAPONS_PATH,
    'effective',
    DI_WEAPONS,
    [
      { nodeId: 'di-01', entryId: 'di-01', start: 3, end: 3 },
      { nodeId: 'di-02', entryId: 'di-02', start: 8, end: 8 }
    ],
    [MOD_NAME, '1.6 · Common', 'DefInjected/ThingDef', 'Things_Weapons.xml'],
    '1.6',
    'Common'
  ),
  ...extra
});

/** Scenario registry — ids follow SOURCE_INSPECTOR_MANDATE §18. */
export const SOURCE_SCENARIOS: SourceScenario[] = [
  {
    id: 'source/simple',
    entries: simpleEntries,
    files: filesFor(),
    ...scenarioBase()
  },
  {
    id: 'source/tkey-multi-context',
    entries: tkeyEntries,
    files: filesFor({
      [TK_SCENARIO_PATH]: file(
        TK_SCENARIO_PATH,
        TK_SCENARIO_PATH,
        'effective',
        TK_SCENARIO,
        [
          { nodeId: 'tk-01.primary', entryId: 'tk-01', start: 14, end: 14 },
          { nodeId: 'tk-01.other', entryId: 'tk-01', start: 18, end: 18 }
        ],
        [MOD_NAME, '1.6 · Common', 'Defs/ScenarioDefs', 'Scenarios_Demo.xml'],
        '1.6',
        'Common'
      )
    }),
    ...scenarioBase()
  },
  {
    id: 'source/version-override',
    entries: versionOverrideEntries,
    files: filesFor({
      [ITEMS_DEMO_COMMON_PATH]: file(
        ITEMS_DEMO_COMMON_PATH,
        ITEMS_DEMO_COMMON_PATH,
        'effective',
        ITEMS_DEMO_COMMON,
        [{ nodeId: 'di-03', entryId: 'di-03', start: 19, end: 19 }],
        [MOD_NAME, '1.6 · Common', 'Defs/ThingDefs_Items', 'Items_Demo.xml'],
        '1.6',
        'Common'
      ),
      [ITEMS_DEMO_15_PATH]: file(
        ITEMS_DEMO_15_PATH,
        ITEMS_DEMO_15_PATH,
        'original',
        ITEMS_DEMO_15,
        [{ nodeId: 'di-03.shadowed', entryId: 'di-03', start: 11, end: 11 }],
        [MOD_NAME, '1.5', 'Defs/ThingDefs_Items', 'Items_Demo.xml'],
        '1.5',
        '1.5'
      )
    }),
    ...scenarioBase()
  },
  {
    id: 'source/generated-output',
    entries: simpleEntries,
    files: filesFor({
      [GENERATED_WEAPONS_PATH]: file(
        GENERATED_WEAPONS_PATH,
        GENERATED_WEAPONS_PATH,
        'generated',
        GENERATED_WEAPONS,
        [
          { nodeId: 'gen.di-01', entryId: 'di-01', start: 5, end: 5 },
          { nodeId: 'gen.di-02', entryId: 'di-02', start: 7, end: 7 }
        ],
        [MOD_NAME, 'output · ru', 'DefInjected/ThingDef', 'Things_Weapons.xml']
      )
    }),
    compare: {
      'di-02': {
        entryId: 'di-02',
        sourceText: simpleEntries['di-02'].usages[0]
          ? 'A gas-operated military rifle. Good middle ground between range, damage and accuracy. Standard issue for outlander soldiers across the rim.'
          : '',
        targetText:
          'Армейская винтовка с газоотводной автоматикой. Хороший баланс дальности, урона и точности. Штатное оружие чужеземных солдат по всей обочине.',
        generated: {
          path: GENERATED_WEAPONS_PATH,
          displayPath: GENERATED_WEAPONS_PATH,
          kind: 'generated',
          lines: GENERATED_WEAPONS,
          lineNumbersGuaranteed: true,
          spans: [{ nodeId: 'gen.di-02', entryId: 'di-02', start: 7, end: 7 }],
          breadcrumb: [MOD_NAME, 'output · ru', 'Things_Weapons.xml']
        },
        generatedSpan: { nodeId: 'gen.di-02', entryId: 'di-02', start: 7, end: 7 }
      }
    },
    browser
  },
  {
    id: 'source/external-change',
    entries: simpleEntries,
    files: filesFor(),
    ...scenarioBase(),
    externalChange: {
      file: KEYED_GAMEPLAY_PATH,
      displayPath: KEYED_GAMEPLAY_PATH,
      summaryKey: 'source.changed.summary',
      oldExcerpt: ['<MessageLetterArrived>{0}: A letter has arrived.</MessageLetterArrived>'],
      newExcerpt: ['<MessageLetterArrived>{0}: A letter has just arrived at the colony.</MessageLetterArrived>']
    }
  },
  {
    id: 'source/missing-file',
    entries: {
      'keyed-02': {
        entryId: 'keyed-02',
        usages: [
          {
            role: 'primary',
            effective: true,
            location: {
              path: 'Languages/English/Keyed/Ancient_Complex.xml',
              displayPath: 'Languages/English/Keyed/Ancient_Complex.xml',
              line: 9,
              column: null,
              nodePath: 'LanguageData/AncientComplexWarning'
            },
            provenance: [
              { kind: 'definition', detail: 'Recorded location from the last scan.' }
            ]
          }
        ],
        excerpt: {
          defName: '— (Keyed)',
          field: 'AncientComplexWarning',
          related: [{ labelKey: 'source.excerpt.folder', value: 'Languages/English/Keyed' }]
        }
      }
    },
    files: filesFor({
      'Languages/English/Keyed/Ancient_Complex.xml': {
        path: 'Languages/English/Keyed/Ancient_Complex.xml',
        displayPath: 'Languages/English/Keyed/Ancient_Complex.xml',
        kind: 'effective',
        missing: true,
        reasonKey: 'source.missing.reason'
      }
    }),
    ...scenarioBase()
  }
];

export const DEFAULT_SCENARIO_ID = SOURCE_SCENARIOS[0].id;
