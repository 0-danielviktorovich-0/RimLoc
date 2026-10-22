// Source Discovery (Stage A) — read-only full-project source search state.
// Matches are reconciled against the canonical inventory THROUGH the
// scanner; a raw text match alone is never presented as a defect. Bounded
// by the backend (offset pagination + caps); errors are honest.
import { useSyncExternalStore } from 'react'
import { clientInstance } from '../client/instance'
import type { SourceSearchMatchDto, SourceSearchReportDto } from '../client/types'

export interface SourceSearchState {
  /** UI-11 Stage C: identity currently being included (button-level busy). */
  includeBusyKey: string | null
  includeError: string | null
  status: 'idle' | 'running' | 'done' | 'error'
  query: string
  scope: 'standard' | 'all_text'
  report: SourceSearchReportDto | null
  error: string | null
  /** Stage B: active classification filter. Empty set = show all — the
   * counts stay whole-tree regardless (they come from the backend). */
  statusFilter: ReadonlySet<SourceSearchMatchDto['status']>
}

let state: SourceSearchState = {
  includeBusyKey: null,
  includeError: null,
  status: 'idle',
  query: '',
  scope: 'standard',
  report: null,
  error: null,
  statusFilter: new Set(),
}

const listeners = new Set<() => void>()

function set(patch: Partial<SourceSearchState>): void {
  state = { ...state, ...patch }
  listeners.forEach((l) => l())
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function getState(): SourceSearchState {
  return state
}

export function useSourceSearch(): SourceSearchState {
  return useSyncExternalStore(subscribe, getState, getState)
}

export const sourceSearchStore = {
  setQuery(query: string): void {
    set({ query })
  },

  setScope(scope: 'standard' | 'all_text'): void {
    set({ scope })
  },

  clear(): void {
    set({
      status: 'idle',
      report: null,
      error: null,
      statusFilter: new Set(),
      includeBusyKey: null,
      includeError: null,
    })
  },

  /** UI-11 Stage C: guarded inclusion of ONE candidate into the managed
   *  project state. On success the snapshot is re-adopted (the new row is
   * live) — the caller re-runs the search to see the classification flip
   * to AlreadyIndexed. */
  async includeCandidate(
    projectId: string,
    sessionEpoch: number,
    expectedRevision: number,
    file: string,
    line: number,
    key: string,
  ): Promise<boolean> {
    set({ includeBusyKey: `${file}:${line}`, includeError: null })
    try {
      await clientInstance.getClient().includeCandidate({
        project_id: projectId,
        session_epoch: sessionEpoch,
        expected_revision: expectedRevision,
        file,
        line,
        key,
      })
      set({ includeBusyKey: null })
      return true
    } catch (e) {
      set({
        includeBusyKey: null,
        includeError: e instanceof Error ? e.message : String(e),
      })
      return false
    }
  },

  /** Stage B: toggle one classification in the filter (a fresh Set on every
   * toggle so useSyncExternalStore sees the change). */
  toggleStatusFilter(s: SourceSearchMatchDto['status']): void {
    const next = new Set(state.statusFilter)
    if (next.has(s)) {
      next.delete(s)
    } else {
      next.add(s)
    }
    set({ statusFilter: next })
  },

  clearStatusFilter(): void {
    set({ statusFilter: new Set() })
  },

  async run(projectId: string, sessionEpoch: number): Promise<void> {
    if (state.query.trim().length < 2) {
      set({ status: 'error', error: 'short_query' })
      return
    }
    set({ status: 'running', error: null })
    try {
      const report = await clientInstance.getClient().sourceSearch({
        projectId,
        sessionEpoch,
        query: state.query,
        scope: state.scope,
        offset: 0,
        maxResults: 200,
      })
      set({ status: 'done', report })
    } catch (e) {
      set({
        status: 'error',
        error: e instanceof Error ? e.message : String(e),
      })
    }
  },

  async loadMore(projectId: string, sessionEpoch: number): Promise<void> {
    const report = state.report
    if (!report || !report.truncated) return
    const nextOffset = report.offset + report.matches.length
    set({ status: 'running' })
    try {
      const more = await clientInstance.getClient().sourceSearch({
        projectId,
        sessionEpoch,
        query: state.query,
        scope: state.scope,
        offset: nextOffset,
        maxResults: 200,
      })
      const merged: SourceSearchReportDto = {
        ...more,
        total_matches: more.total_matches,
        offset: nextOffset,
        truncated: more.truncated,
        matches: [...report.matches, ...more.matches],
      }
      set({ status: 'done', report: merged })
    } catch (e) {
      set({
        status: 'error',
        error: e instanceof Error ? e.message : String(e),
      })
    }
  },
}

/** Status → i18n key suffix for the honest chip labels. */
export function statusKey(status: SourceSearchMatchDto['status']): string {
  return `ss.status.${status}`
}

/** Stage B: the visible page of matches under the active classification
 * filter (empty filter = everything the backend returned for this page). */
export function visibleMatches(s: SourceSearchState): SourceSearchMatchDto[] {
  const report = s.report
  if (!report) return []
  if (s.statusFilter.size === 0) return report.matches
  return report.matches.filter((m) => s.statusFilter.has(m.status))
}
