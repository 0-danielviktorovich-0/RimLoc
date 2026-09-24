// Global test setup: reset persisted state (localStorage + hash) between
// tests so each one starts from a "fresh machine".
import { afterEach } from 'vitest';
import { cleanupMounted } from './helpers';

afterEach(() => {
  cleanupMounted();
  window.localStorage.clear();
  window.location.hash = '';
  window.history.replaceState(null, '', '#');
});
