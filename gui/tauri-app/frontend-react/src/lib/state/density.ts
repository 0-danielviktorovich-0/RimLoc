// UI-04: the density primitive (comfortable vs compact). Density is
// ORTHOGONAL to zoom — zoom scales everything, density re-tunes row
// padding and list gaps only, so a user can have big text AND tight
// rows. Persisted per app; consumers re-render via the custom event.
export type UiDensity = 'comfortable' | 'compact'

export const DENSITY_KEY = 'rimloc.ui.density'
export const DENSITY_EVENT = 'rimloc.ui.density'

export function readUiDensity(): UiDensity {
  return window.localStorage.getItem(DENSITY_KEY) === 'compact' ? 'compact' : 'comfortable'
}

export function writeUiDensity(d: UiDensity): void {
  window.localStorage.setItem(DENSITY_KEY, d)
  window.dispatchEvent(new CustomEvent(DENSITY_EVENT))
}
