# Cross-platform acceptance matrix — 2026-10-23 (honest form)

Per the finishing mandate: «Если конкретная платформа недоступна для
runtime-проверки, честно укажи BUILD_VERIFIED / RUNTIME_NOT_TESTED».
Successful installer creation is NOT counted as user acceptance.

| Platform | Artifact | Built from | SHA256 | Install | First run | Create/open | Edit+save | Reopen | Export | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| macOS (arm64) | dmg + app | CI run 39827270446, source 8610e313 (tree identical to the merge commit) | dadd6fce… / binary 50d7ba20… | PASS (app copied from CI bundle; unsigned → right-click Open path documented in UAT_GUIDE) | PASS (isolated profile, background, no focus steal) | PASS (stage1 wizard journey) | PASS (stage1: Def label + Keyed, persisted across reopen) | PASS (stage1 reopen through Projects) | PASS (stage1: LanguageData verified on disk) | **RUNTIME_VERIFIED** |
| macOS — CI-built bundle itself | app | same | same | PASS (launch smoke: process, AX window, 0600 broker socket) | PASS (start screen) | covered by the same artifact | — | — | — | **RUNTIME_VERIFIED (smoke)** |
| Windows (x64) | NSIS `..._x64-setup.exe` | CI run 39827270446 | 970dc092… | BUILD_VERIFIED | RUNTIME_NOT_TESTED (no Windows host/VM in this environment) | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | **BUILD_VERIFIED / RUNTIME_NOT_TESTED** |
| Linux (deb) | `..._amd64.deb` | CI run 39827270446 | af60364e… | BUILD_VERIFIED | RUNTIME_NOT_TESTED (no Linux desktop host; CI startup smoke runs headless Xvfb — flaky, tracked separately) | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | **BUILD_VERIFIED / RUNTIME_NOT_TESTED** |
| Linux (AppImage) | `..._amd64.AppImage` | CI run 39827270446 | 37fa36fd… | BUILD_VERIFIED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | RUNTIME_NOT_TESTED | **BUILD_VERIFIED / RUNTIME_NOT_TESTED** |

Notes:
- The Linux CI job DOES include a virtual-display startup smoke (Xvfb +
  AppImage launch, process check). It has flaked once (GUI process not found
  within the smoke window) and passed on rerun — recorded as an infra flake,
  not user acceptance.
- macOS product smoke = the app launches and serves (process + AX window +
  0600 drafts broker). The full user journey ran on the automation artifact
  built from the same source tree.
- A cross-platform installed-runtime acceptance requires Windows/Linux hosts
  or VMs — remains a named gap for the owner's environment decision.
