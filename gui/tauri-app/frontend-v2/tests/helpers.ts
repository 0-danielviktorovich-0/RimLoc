// Minimal Svelte 5 mount/unmount helpers for component regression tests:
// no testing-library, plain DOM queries against data-testid attributes.
import { flushSync, mount, unmount } from 'svelte';

type Component = Parameters<typeof mount>[0];

const instances: ReturnType<typeof mount>[] = [];

/** Mount a component into a fresh div on document.body and flush effects. */
export function mountCmp(C: Component, props: Record<string, unknown> = {}): void {
  const target = document.createElement('div');
  document.body.appendChild(target);
  instances.push(mount(C, { target, props }));
  flushSync();
}

/** Unmount everything mounted by mountCmp and clear the DOM. */
export function cleanupMounted(): void {
  for (const inst of instances) {
    unmount(inst);
  }
  instances.length = 0;
  document.body.innerHTML = '';
}

export function q(testid: string): HTMLElement {
  const el = document.querySelector<HTMLElement>(`[data-testid="${testid}"]`);
  if (!el) throw new Error(`Element not found: ${testid}`);
  return el;
}

export function qAll(testid: string): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(`[data-testid="${testid}"]`)];
}

export function exists(testid: string): boolean {
  return document.querySelector(`[data-testid="${testid}"]`) !== null;
}

export function click(testid: string): void {
  q(testid).click();
  flushSync();
  // A click may navigate via the hash (jsdom fires `hashchange` async, a
  // real browser does too) — settle the router synchronously. Re-firing
  // with an unchanged hash is a no-op for the store.
  window.dispatchEvent(new HashChangeEvent('hashchange'));
  flushSync();
}

/** Set an input value the way real typing does (value + input event), then
 *  flush — the canonical contract-ops idiom, factored for reuse. */
export function typeInInput(testid: string, value: string): void {
  const input = q(testid) as HTMLInputElement;
  input.value = value;
  input.dispatchEvent(new Event('input'));
  flushSync();
}

/** Click a checkbox testid by dispatching change (real input element). */
export function check(testid: string): void {
  const input = q(testid) as HTMLInputElement;
  input.checked = true;
  input.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

/**
 * Navigate via the hash router. jsdom fires `hashchange` asynchronously (a
 * real browser does too), so dispatch it synchronously to keep tests
 * deterministic.
 */
export function goto(hash: string): void {
  window.location.hash = hash;
  window.dispatchEvent(new HashChangeEvent('hashchange'));
  flushSync();
}
