// Minimal hash router (mandate phase 1): #/home, #/wizard, #/workspace,
// #/review, #/build, #/existing, #/chat, #/settings, #/providers, #/help. No
// dependencies; the hash is the single source of truth, so browser
// back/forward works and deep links (#/review from the wizard result) open
// the right screen. #/chat is the chat-batch manager (spec §10, W3), reached
// from the wizard result or by direct link.
//
// Project-scoped routes (workspace/review/build) render the Workspace shell;
// app-level routes are Home, Wizard, Existing, Chat and the phase-2 stubs
// (settings, providers, help).

export type RouteId =
  | 'home'
  | 'wizard'
  | 'workspace'
  | 'review'
  | 'build'
  | 'existing'
  | 'chat'
  | 'settings'
  | 'providers'
  | 'help';

export const ROUTES: RouteId[] = [
  'home',
  'wizard',
  'workspace',
  'review',
  'build',
  'existing',
  'chat',
  'settings',
  'providers',
  'help'
];

export const PROJECT_ROUTES: RouteId[] = ['workspace', 'review', 'build'];

const ROUTE_SET = new Set<string>(ROUTES);

function parseHash(): RouteId {
  if (typeof window === 'undefined') return 'home';
  const h = window.location.hash.replace(/^#\/?/, '').split('?')[0];
  return ROUTE_SET.has(h) ? (h as RouteId) : 'home';
}

class RouterStore {
  route = $state<RouteId>(parseHash());

  constructor() {
    if (typeof window !== 'undefined') {
      if (!window.location.hash) {
        // Canonicalize the very first load so reload keeps the screen.
        history.replaceState(null, '', '#/home');
      }
      window.addEventListener('hashchange', () => {
        this.route = parseHash();
      });
    }
  }

  isProjectRoute(): boolean {
    return (PROJECT_ROUTES as string[]).includes(this.route);
  }

  navigate(r: RouteId) {
    if (this.route === r && parseHash() === r) return;
    window.location.hash = `#/${r}`;
  }
}

export const router = new RouterStore();
