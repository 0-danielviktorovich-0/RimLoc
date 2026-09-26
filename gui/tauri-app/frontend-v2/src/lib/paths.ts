// Absolute-path FORM check for caller-specified output roots. Pure string
// logic — no filesystem access — mirroring the backend fail-closed guard
// (`invalid_output_path`): the writer builds directories under the given
// path with the app CWD as the implicit base, so a relative path would
// silently land wherever the app was launched from. Recognized forms:
//   - POSIX absolute:  /Users/you/RimLoc-Export
//   - Windows drive:   C:/Users/you/RimLoc-Export  or  C:\Users\you\...
//   - Windows UNC:     \\server\share\folder
// The Rust guard is the last line; this is the client-side pre-check that
// keeps the run button honest.
export function looksAbsolutePath(p: string): boolean {
  if (p.startsWith('/')) return true;
  if (/^[A-Za-z]:[\\/]/.test(p)) return true;
  if (p.startsWith('\\\\')) return true;
  return false;
}
