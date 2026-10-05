// M-10 regression (UI audit 2026-09-29): an export attempt with an invalid
// output path must never be silent —
//   (a) non-absolute path → the run button disables AND an inline reason
//       is visible next to the field;
//   (b) a late typed refusal (guard) → the error renders INSIDE the export
//       card (role=alert), not only above the fold.
// Component-level on the real ContractOps markup (jsdom + svelte mount),
// against the mock contract transport — the same wire semantics as Tauri.
import { flushSync } from 'svelte';
import { mount } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import ContractOps from '../src/lib/components/screens/ContractOps.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { contractops } from '../src/lib/stores/contractops.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import { review } from '../src/lib/stores/review.svelte';

function setInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

async function settle() {
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
}

describe('M-10: export refusal is visible, never silence', () => {
  let host: HTMLElement;

  beforeEach(async () => {
    project.reset();
    contractops.reset();
    buildState.reset();
    review.resetSession();
    // A REAL contract project through the mock transport (same DTOs/wire
    // errors as the Tauri bridge) so the live build panel renders.
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    host = document.createElement('div');
    document.body.appendChild(host);
    mount(ContractOps, { target: host, props: { kind: 'build' } });
    await settle();
  });

  it('(a) relative path: button disabled + inline reason visible', async () => {
    const field = host.querySelector<HTMLInputElement>('[data-testid="contractops.export.outdir"]');
    expect(field).toBeTruthy();

    setInputValue(field!, 'RimLoc-Export/Мод-Russian'); // относительный
    await settle();

    const run = host.querySelector<HTMLButtonElement>('[data-testid="contractops.export.run"]');
    expect(run?.disabled).toBe(true); // отказ ДО запроса — кнопка не жмётся

    const reason = host.querySelector('[data-testid="contractops.export.outdir.invalid"]');
    expect(reason).toBeTruthy(); // ...и причина видна рядом, не тишиной
    expect(reason?.textContent ?? '').toMatch(/абсолютн/i);
  });

  it('(a-empty) пустой путь: кнопка неактивна, ложного результата нет', async () => {
    const run = host.querySelector<HTMLButtonElement>('[data-testid="contractops.export.run"]');
    expect(run?.disabled).toBe(true);
    expect(host.querySelector('[data-testid="contractops.export.result"]')).toBeNull();
    expect(host.querySelector('[data-testid="contractops.export.error"]')).toBeNull(); // без нажатия — без ошибки
  });

  it('(b) поздний типизированный отказ: ошибка внутри карточки экспорта', async () => {
    const field = host.querySelector<HTMLInputElement>('[data-testid="contractops.export.outdir"]');
    // Абсолютный, но внутри исходного дерева — бэкенд-гард откажет ПОСЛЕ
    // запроса (guard_output_denied); UI обязан показать это у кнопки.
    setInputValue(field!, '/mods/Demo/Languages/Russian');
    await settle();

    const run = host.querySelector<HTMLButtonElement>('[data-testid="contractops.export.run"]');
    expect(run?.disabled).toBe(false);
    run!.click();
    // mock-транспорт отвечает сразу; дорисовка ошибки идёт после промиса
    await new Promise((r) => setTimeout(r, 20));
    flushSync();

    const err = host.querySelector('[data-testid="contractops.export.error"]');
    expect(err).toBeTruthy();
    expect(err?.getAttribute('role')).toBe('alert');
    expect(contractops.error ?? '').toBeTruthy();
    expect(host.querySelector('[data-testid="contractops.export.result"]')).toBeNull();
  });
});
