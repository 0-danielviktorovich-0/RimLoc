// Reactive binding for the plain project store (useSyncExternalStore).
import { useSyncExternalStore } from 'react'
import { getState, subscribe, type ProjectState } from './project'

export function useProjectState(): ProjectState {
  return useSyncExternalStore(subscribe, getState, getState)
}
