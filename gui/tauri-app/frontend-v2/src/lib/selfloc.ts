// Self-localization entry (mandate D, wave 5): the app-bundled RimLoc UI
// catalog opened as an ORDINARY translation project through the EXISTING
// contract flow (selfloc_catalog_dir → project_open|project_create →
// workspace). ONE shared flow for every UI entry point — the Home card and
// the Help screen card both call openSelflocProject(); no duplicated logic.
//
// The catalog project name is the staging dir basename stamped by the
// backend (selfloc_catalog.rs) — the stable identity of "the RimLoc UI
// project". A second click REOPENS that project instead of minting a
// duplicate (rel3 acceptance found 7 copies after 7 clicks).
//
// In mock mode the typed refusal (unsupported_capability) IS the honest
// outcome: no fabricated catalog dir, no fake success — the caller surfaces
// `error` verbatim in its own section alert.
import { clientInstance } from './client/instance.svelte';
import { project } from './stores/project.svelte';
import { router } from './router.svelte';

export const SELFLOC_PROJECT_NAME = 'RimLoc UI (en)';

/** ok = the catalog project is open and the workspace navigated. */
export type SelflocResult = { ok: true } | { ok: false; error: string | null };

/**
 * Open (or reopen) the RimLoc UI catalog project. Every failure — a
 * resolver refusal, a contract failure — comes back as `error` verbatim;
 * the caller owns busy/error UI state, this function owns the flow.
 */
export async function openSelflocProject(): Promise<SelflocResult> {
  try {
    const dir = await clientInstance.getClient().selflocCatalogDir();
    const existing = (await project.listContractProjects()).find(
      (p) => p.name === SELFLOC_PROJECT_NAME
    );
    const ok = existing
      ? await project.openContractProject(existing.project_id)
      : await project.createContractProject(dir);
    if (!ok) return { ok: false, error: project.contractError };
    router.navigate('workspace');
    return { ok: true };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}
