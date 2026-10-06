// localeId → strict language-folder form for the contract calls that take a
// target locale (`project_export`, `project_build_mod`,
// `project_import_existing`, `project_apply_existing`): the backend joins the
// locale into `Languages/<locale>` output paths and refuses any non-folder
// form BEFORE any path is built (P1-2, session.rs `ensure_locale_form`:
// `^[A-Za-z0-9_-]+$`). Port of the frozen oracle's FOLDER_FORM
// (frontend-v2 contractops.svelte.ts); builtin names derive from the
// registry's rimworldFolder minus the "(native)" pack suffix —
// 'Russian (Русский)' is the on-disk official-pack name, the contract wants
// the plain folder segment. Unknown/user locales fall back to the localeId
// itself (user languages already store a plain rimworldFolder).
import { BUILTIN_LANGUAGES } from './registry'

const FOLDER_FORM: Record<string, string> = Object.fromEntries(
  BUILTIN_LANGUAGES.map((l) => [l.localeId, l.rimworldFolder.replace(/\s*\(.*\)\s*$/, '')]),
)

export function folderForm(localeId: string): string {
  return FOLDER_FORM[localeId] ?? localeId
}
