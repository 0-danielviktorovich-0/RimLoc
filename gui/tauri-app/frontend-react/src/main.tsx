import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@fontsource/golos-text/400.css'
import '@fontsource/golos-text/500.css'
import '@fontsource/golos-text/600.css'
import '@fontsource/literata/400.css'
import '@fontsource/literata/500.css'
import '@fontsource/literata/600.css'
import '@fontsource/ibm-plex-mono/400.css'
import '@fontsource/ibm-plex-mono/500.css'
import './styles/tokens.css'
import './styles/r1.css'
import './styles/r1-react.css'
import { App } from './App'
import { listenBrokerChanges } from './lib/state/broker_events'

// Offline-bundled fonts (mandate §40): Golos Text (UI) / Literata (display) /
// IBM Plex Mono (code) — the R1 typography trio, no network fetches.

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

// Phase 2b (MCP_SERVER_PHASE2.md «React sync»): the SINGLE broker-changed
// listener, bootstrapped once — the stores are module-level singletons, so
// by this import they already exist. A failed subscription never takes the
// UI down: honest console.warn, the app lives without broker sync.
void listenBrokerChanges().catch((e: unknown) => {
  console.warn('[broker-events] subscription failed — UI continues without broker sync:', e)
})
