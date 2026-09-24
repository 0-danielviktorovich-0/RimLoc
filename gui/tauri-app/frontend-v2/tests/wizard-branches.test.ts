// W1 regression: wizard is a finite state machine — step 2 branches by the
// content type picked on step 1, Back/Next work in every branch, and going
// back never resets a made selection (QA mandate §2).
import { describe, expect, it } from 'vitest';
import Wizard from '../src/lib/components/screens/Wizard.svelte';
import { check, click, exists, mountCmp, q } from './helpers';

/** Advance to the selection step with the given content type. */
function openBranch(testid: string) {
  mountCmp(Wizard);
  click(testid); // step 1: pick the content type
  click('wizard.next'); // step 2
}

describe('wizard: step 2 branches by content type', () => {
  it('MOD branch shows discovered mods + folder/drop, not installations', () => {
    openBranch('wizard.content.mod');
    expect(exists('wizard.step2.mod')).toBe(true);
    expect(exists('wizard.mod.vfe-furniture')).toBe(true);
    expect(exists('wizard.choose-folder')).toBe(true);
    // No foreign-branch content:
    expect(exists('wizard.install.steam-win')).toBe(false);
    expect(exists('wizard.pack.core-ru')).toBe(false);
  });

  it('BASE GAME branch shows detected installations (Core/version), not mods', () => {
    openBranch('wizard.content.base');
    expect(exists('wizard.step2.base')).toBe(true);
    expect(exists('wizard.install.steam-win')).toBe(true);
    expect(exists('wizard.install.gog-mac')).toBe(true);
    expect(exists('wizard.mod.vfe-furniture')).toBe(false);
  });

  it('DLC branch shows installation + installed-DLC checkboxes', () => {
    openBranch('wizard.content.dlc');
    expect(exists('wizard.step2.dlc')).toBe(true);
    // Installation selector present:
    expect(exists('wizard.install.steam-win')).toBe(true);
    // Installed DLC tickable; not-installed one disabled:
    const royalty = q('wizard.dlc.royalty') as HTMLInputElement;
    const odyssey = q('wizard.dlc.odyssey') as HTMLInputElement;
    expect(royalty.disabled).toBe(false);
    expect(odyssey.disabled).toBe(true);
    // No mod list in the DLC branch:
    expect(exists('wizard.mod.vfe-furniture')).toBe(false);
  });

  it('LANGUAGE PACK branch shows packs with language mapping', () => {
    openBranch('wizard.content.pack');
    expect(exists('wizard.step2.pack')).toBe(true);
    expect(exists('wizard.pack.core-ru')).toBe(true);
    // Mapping block for the selected pack:
    expect(exists('wizard.pack-mapping')).toBe(true);
    expect(exists('wizard.pack-from')).toBe(true);
    expect(exists('wizard.pack-to')).toBe(true);
    expect(exists('wizard.mod.vfe-furniture')).toBe(false);
  });
});

describe('wizard: Next gate per branch', () => {
  it('DLC: Next is disabled until at least one installed DLC is ticked', () => {
    openBranch('wizard.content.dlc');
    const next = q('wizard.next') as HTMLButtonElement;
    expect(next.disabled).toBe(true); // nothing ticked yet
    check('wizard.dlc.biotech');
    expect(next.disabled).toBe(false);
  });

  it('BASE GAME: Next works once an installation is picked', () => {
    openBranch('wizard.content.base');
    click('wizard.install.steam-win');
    click('wizard.next');
    // Step 3 (languages) reached:
    expect(exists('wizard.source-lang')).toBe(true);
  });

  it('LANGUAGE PACK: Next works once a pack is picked', () => {
    openBranch('wizard.content.pack');
    click('wizard.pack.core-ru');
    click('wizard.next');
    expect(exists('wizard.source-lang')).toBe(true);
  });
});

describe('wizard: back-navigation preserves branch and selection', () => {
  // One scenario per content type: pick → next → back → the branch AND the
  // selection survive; next again re-enters the same branch (QA mandate §2).
  const scenarios = [
    {
      content: 'wizard.content.mod',
      select: 'wizard.mod.rimatomics',
      selected: 'wizard.mod.rimatomics',
      unselected: 'wizard.mod.vfe-furniture'
    },
    {
      content: 'wizard.content.base',
      select: 'wizard.install.gog-mac',
      selected: 'wizard.install.gog-mac',
      unselected: 'wizard.install.steam-win'
    },
    {
      content: 'wizard.content.dlc',
      select: 'wizard.dlc.anomaly', // checkbox
      selected: 'wizard.dlc.anomaly',
      unselected: null
    },
    {
      content: 'wizard.content.pack',
      select: 'wizard.pack.core-ja',
      selected: 'wizard.pack.core-ja',
      unselected: 'wizard.pack.core-ru'
    }
  ] as const;

  for (const s of scenarios) {
    it(`${s.content}: back to step 1 and forward keeps the choice`, () => {
      openBranch(s.content);

      if (s.content === 'wizard.content.dlc') {
        check(s.select);
      } else {
        click(s.select);
      }
      const isRadio = s.content !== 'wizard.content.dlc';
      const assertSelection = () => {
        const selected = q(s.selected);
        if (isRadio) {
          expect(selected.getAttribute('aria-checked')).toBe('true');
        } else {
          expect((selected as HTMLInputElement).checked).toBe(true);
        }
        if (s.unselected) {
          const other = q(s.unselected);
          if (isRadio) {
            expect(other.getAttribute('aria-checked')).toBe('false');
          } else {
            expect((other as HTMLInputElement).checked).toBe(false);
          }
        }
      };
      assertSelection();

      click('wizard.next'); // → step 3
      click('wizard.back'); // → step 2, same branch
      expect(exists(`wizard.step2.${s.content.split('.').pop()}`)).toBe(true);
      assertSelection();

      click('wizard.back'); // → step 1, content type still highlighted
      const contentBtn = q(s.content);
      expect(contentBtn.getAttribute('aria-pressed')).toBe('true');
      click('wizard.next'); // → step 2 again, same branch + selection
      assertSelection();
    });
  }

  it('switching content types on step 1 keeps per-branch selections', () => {
    openBranch('wizard.content.mod');
    click('wizard.mod.rimatomics');

    click('wizard.back'); // step 1
    click('wizard.content.pack');
    click('wizard.next');
    click('wizard.pack.core-ja');
    click('wizard.back');
    click('wizard.back'); // step 1 again

    // Back to MOD: the earlier mod choice is still there (not reset).
    click('wizard.content.mod');
    click('wizard.next');
    expect(
      q('wizard.mod.rimatomics').getAttribute('aria-checked')
    ).toBe('true');
  });
});

describe('wizard: preflight shows the branch selection summary', () => {
  it('reflects the chosen pack, not a mod', () => {
    openBranch('wizard.content.pack');
    click('wizard.pack.core-ru');
    click('wizard.next'); // step 3
    click('wizard.next'); // step 4
    click('wizard.next'); // step 5 = preflight
    const summary = q('wizard.selection-summary').textContent ?? '';
    expect(summary).toContain('RimWorld Core');
  });
});
