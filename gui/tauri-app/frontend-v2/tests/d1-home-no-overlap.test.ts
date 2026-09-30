// D-1 regression (design pass 2026-10-01): the Home side rail must never
// overlap the recents table — child cards of .col-main are grid items with
// min-width:auto and were sized by the table's min-content, overflowing the
// track UNDER the side column on every width (wide included).
// jsdom can't lay out, so this asserts the STYLE CONTRACT: the column tracks
// are minmax(0, 1fr) (not plain 1fr) — the structural fix that removes the
// content-based minimum.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

describe('D-1: home columns constrain their tracks', () => {
  const css = readFileSync(path.resolve(import.meta.dirname, '../src/lib/components/screens/Home.svelte'), 'utf8');

  it('col-main/col-side implicit tracks are minmax(0, 1fr)', () => {
    const block = css.slice(css.indexOf('.col-main,'), css.indexOf('.card {'));
    expect(block).toContain('grid-template-columns: minmax(0, 1fr)');
  });

  it('the main grid keeps the two-column shape (1fr + 360px)', () => {
    const block = css.slice(css.indexOf('  .grid {'), css.indexOf('  .grid-first'));
    expect(block).toContain('grid-template-columns: minmax(0, 1fr) 360px');
  });

  it('both columns keep min-width: 0 (belt and suspenders)', () => {
    const block = css.slice(css.indexOf('.col-main,'), css.indexOf('.card {'));
    expect(block).toContain('min-width: 0');
  });
});
