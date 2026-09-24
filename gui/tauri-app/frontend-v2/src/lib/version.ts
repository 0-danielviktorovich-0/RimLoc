// Single source of the application version for every UI surface that shows
// it (About card, bug-report block). The version is read statically from the
// GUI package.json metadata — no handwritten literal anywhere in the GUI
// (W4.5 integration requirement #4, lead correction 004). At live binding
// this module stays the one import surface: swap its body for Tauri build
// metadata without touching consumers. Do not borrow the CLI crate version
// here — backend versioning gets its own source when live binding lands.
import pkg from '../../package.json';

export const APP_VERSION: string = pkg.version;
