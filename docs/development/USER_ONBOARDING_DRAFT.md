# USER_ONBOARDING_DRAFT — GUI onboarding snippets & packaging notes

> **Draft / dev-only.** Not part of the MkDocs nav; do not link from `docs/en/` or
> `docs/ru/` yet. Sources of flows: `GUI_DESIGN_SPEC.md`, `MOCK_LIVE_ONBOARDING_MANDATE.md`
> (no-mods empty state: `[Try Demo Project][Choose folder][Configure installation]`),
> `frontend-v2` i18n keys (`home.nomods.*`, `build.*`, `home.demo.*`).
> Status: DRAFT — copy is user-facing but unapproved; final wording goes through
> brand-voice review and lands in `docs/en/getting-started.md` + `docs/ru/...` twin.

## 1. First run — no mods detected (empty state)

**EN snippet (README / welcome screen help text):**

> **Welcome to RimLoc!**
> We couldn't find your RimWorld mods folder yet — that's normal on a first run.
>
> - **Try Demo Project** — open a bundled demo mod and take a 2-minute guided tour
>   (edit a string, review, validate, build). Nothing touches your game files.
> - **Choose folder** — point RimLoc at a mod folder you want to translate.
> - **Configure installation** — tell RimLoc where your RimWorld installation
>   and/or Workshop mods live (`~/Library/Application Support/Steam/steamapps/workshop/content/294100`
>   on macOS; auto-detected on most setups). You can skip this and pick folders manually later.

**RU:**

> **Добро пожаловать в RimLoc!**
> Папку с модами RimWorld мы пока не нашли — это нормально при первом запуске.
>
> - **Открыть демо-проект** — встроенный демо-мод и 2-минутный тур: правка строки,
>   ревью, валидация, сборка. Файлы игры не затрагиваются.
> - **Выбрать папку** — укажите папку мода, который хотите перевести.
> - **Настроить установку** — показать RimLoc, где игра и/или Workshop-моды
>   (обычно `steamapps/workshop/content/294100`; на большинстве систем находится автоматически).

**Rule encoded:** demo is always safe and isolated (deterministic reset, mock badge
where the backend is stubbed); real projects require an explicit folder pick.
The `Configure installation` path never edits game files — it only *reads* mod folders.

## 2. First run — Try Demo (guided tour)

One sentence above the tour: *"You'll edit a demo translation end-to-end — the same
4 moves you'll repeat on your own mod."*

Tour steps (mirrors `MOCK_LIVE_ONBOARDING_MANDATE.md` §7):
1. **Open Demo** → demo project loads with the "Demo data" badge visible.
2. **Pick a string** → editor shows source text, context panel and target field.
3. **Edit it** → make one deliberate mistake; the draft marks the row unsaved.
4. **Review → Validate** → the deliberate mistake is caught (placeholder mismatch /
   empty value) and explained in plain language.
5. **Build demo** → a throwaway translation mod is produced; final screen:
   *"That's the whole loop. Now open your own mod."*
6. **Reset** returns the demo to its pristine state at any time.

## 3. Live project — path → create

**EN snippet:**

> **Translate a new mod**
> 1. **Choose a mod** — select the mod's root folder (the one containing `About/About.xml`).
>    If you picked the wrong folder, RimLoc tells you what's missing instead of guessing.
> 2. **Target language** — pick the language you're translating into (e.g. Russian).
> 3. **Scope** — RimLoc scans `Keyed` and `DefInjected` sources; advanced settings
>     (extra paths, exclusions) are collapsed by default.
> 4. **Create project** → the editor opens with every translation unit listed and a
>    progress overview (translated / empty / warnings).
>
> Existing translation work in the mod is picked up automatically — you resume where
> the mod's author left off, and RimLoc never writes into the source mod folder
> unless you ask it to build/install a translation mod.

CLI equivalent (for power users / docs cross-link):

```bash
rimloc-cli scan --root ./MyMod --format json
rimloc-cli export-po --root ./MyMod --out-po ./MyMod.po --lang ru
rimloc-cli validate --root ./MyMod
rimloc-cli build-mod --po ./MyMod.po --out-mod ./MyMod-ru --lang ru
```

## 4. Editing → save

**EN snippet:**

> Every change is a draft until you save. Unsaved rows are highlighted; leaving the
> editor with unsaved changes prompts once. `Cmd/Ctrl+S` saves the visible string;
> "Save all" flushes the draft set. Saving does **not** touch the game or the source
> mod — your edits live in the project until you run **Build**.

Packaging-relevant behavior note: autosave-on-close is *not* silent — a confirmation
dialog lists how many rows will be saved.

## 5. Validation & build

**EN snippet:**

> **Validate** runs the same QA checks the CLI uses: duplicate keys, empty values,
> placeholder mismatches (`{0}`, `{color}`), format problems. Results are grouped by
> severity; a row click jumps to the string.
>
> **Build** produces a standalone translation-only mod from your project:
> - previews a summary first (coverage %, warnings count, destination folder);
> - **dry-run** is the default for the first build of a project;
> - *Install into game* copies the built mod into your Workshop/Mods folder — only
>   after an explicit confirmation, and only if a game installation is configured.

Success toast: *"Translation mod built — N strings, M warnings. Open the folder?"*

## 6. Packaging notes (what ships, what doesn't)

**In the package (target set):**
- `rimloc-cli` binaries for Linux (x86_64, aarch64), macOS (aarch64, x86_64),
  Windows (x86_64) — also installable from crates.io (`cargo install rimloc-cli`).
- The new GUI app (`gui/tauri-app/frontend-v2` shell): macOS `.app`/`.dmg`,
  Windows installer, Linux AppImage/deb — artifacts from CI builds.
- SBOM is generated by CI (existing release pipeline); docs site bundled separately
  at GitHub Pages.

**Not in the package (as of this draft):**
- **No code signatures / notarization.** macOS builds are unsigned: first launch
  needs right-click → *Open* (or `xattr -cr RimLoc.app`) because Gatekeeper warns
  about unidentified developers. Windows builds are unsigned: SmartScreen shows
  "More info → Run anyway". Document this in the download page, not just in an FAQ.
- **No published checksums.** Release assets ship without a `SHA256SUMS` manifest —
  a user cannot verify a download end-to-end. Cheap fix candidate: emit
  `SHA256SUMS.txt` in the release workflow and link it from the download page.
- No auto-updater; updates are manual downloads (or `cargo install --force` for CLI).

**Future candidates (out of scope for this draft):** Apple notarization + Windows
signing cert, cosign/SSH artifact signatures, checksum verification in GUI settings,
auto-update channel.

**User-facing README blurb (EN, drop-in):**

> RimLoc ships as two artifacts: the `rimloc-cli` command-line tool (Linux / macOS /
> Windows, also on crates.io) and the desktop GUI. Builds are not code-signed yet —
> on first launch your OS may ask for an extra confirmation (macOS: right-click the
> app → *Open*; Windows: *More info* → *Run anyway*). No release checksums are
> published yet; if you need to verify a build, compile from source with
> `cargo install rimloc-cli`.

---
*Draft authored 2026-09-26 during the RimLoc corpus-expansion night shift. Companion
artifact: test corpus at `~/Developing/RimLoc-test-corpus/` (4 Workshop mods covering
PatchOperations, C# assemblies, multi-version layouts, DLC dependency) — see
`CORPUS_METADATA.md` there.*
