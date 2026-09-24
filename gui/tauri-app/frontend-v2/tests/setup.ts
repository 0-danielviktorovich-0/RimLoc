// Global test setup: reset persisted state (localStorage + hash) between
// tests so each one starts from a "fresh machine".
import { afterEach, vi } from 'vitest';
import { cleanupMounted } from './helpers';

// jsdom does not implement scrollIntoView; the W7 viewer calls it when
// revealing the active match/node. Stub it to kill unhandled rejections
// (lead review 029 N1) without affecting the assertions.
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = vi.fn();
}

afterEach(() => {
  cleanupMounted();
  window.localStorage.clear();
  window.location.hash = '';
  window.history.replaceState(null, '', '#');
});
