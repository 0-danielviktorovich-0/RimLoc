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
import { App } from './App'

// Offline-bundled fonts (mandate §40): Golos Text (UI) / Literata (display) /
// IBM Plex Mono (code) — the R1 typography trio, no network fetches.

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
