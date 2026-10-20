# Novice walkthrough — translate a mod to Russian (RU original below)

**Prerequisites.** RimLoc (GUI or CLI) is installed, and the rimloc
MCP adapter is connected to your host. The adapter is a separate
component (Issue #10) — check its status in the skill's adapter note
before starting. If anything below is missing from
`rimloc.capabilities`, stop and follow references/troubleshooting.md.

Every step names the MCP tool that does the work:

1. `rimloc.projects.list` — see what exists. For a new mod, the user
   gives the mod folder (a RimWorld mod root, i.e. it has
   `About/About.xml`); `rimloc.project.open` builds the project.
   Read `rimloc.project.snapshot`: source locale, target locale
   (set to Russian if the user asked), revision, and the session
   epoch (the session's identity stamp — quote both back on every
   later write; they are what makes a stale write refuse instead of
   clobbering).
2. Confirm the source tree is read-only (RimLoc's contract; say it).
3. `rimloc.entries.search` with a term the user cares about, or a
   filter for untranslated entries. Show 20–50 rows max. The rows'
   **identities** — their stable keys — come back in this response.
4. Draft translations: `rimloc.translation.propose` per entry (local
   provider = free and private; paid provider only on explicit OK).
   The propose response repeats each row's identity.
5. Review with the user, row by row. Apply ONLY what they chose:
   `rimloc.translation.apply { identities: <the row keys from steps
   3–4>, expected_revision: <from the snapshot>, session_epoch:
   <same> }`.
6. `rimloc.validation.run` — fix findings by proposing corrected
   drafts for the affected rows (repeat propose → apply), or explain
   them to the user.
7. `rimloc.export.build { out_dir: <user's choice> }` — report the
   produced translation mod path and its file count.
8. Done. The source mod is byte-identical; all work lives in the
   RimLoc project and the export. To see it in game, the user
   installs/enables the exported translation mod in RimWorld's mod
   list (their choice — RimLoc does not touch the game).

# Пошаговый разбор — перевод мода на русский

**Предусловия.** RimLoc (GUI или CLI) установлен, rimloc MCP-адаптер
подключён к вашему хосту. Адаптер — отдельный компонент (Issue #10):
проверь его статус в заметке скилла до начала. Если чего-то из
нижеперечисленного нет в `rimloc.capabilities` — остановись и открой
references/troubleshooting.md.

1. `rimloc.projects.list` — что уже есть. Новый мод: пользователь
   называет папку (корень мода RimWorld, т.е. с `About/About.xml`),
   `rimloc.project.open` создаёт проект. Прочитай
   `rimloc.project.snapshot`: исходный язык, целевой (русский),
   ревизию и эпоху сессии (идентификатор сессии — цитируй оба при
   каждой записи: именно они превращают устаревшую запись в отказ,
   а не в затирание).
2. Подтверди: исходники мода только читаются, RimLoc в них не пишет.
3. `rimloc.entries.search` по термину пользователя или фильтру
   непереведённого. Покажи не больше 20–50 строк. В ответе приходят
   **identities** — стабильные ключи строк.
4. Черновики: `rimloc.translation.propose` (локальный провайдер —
   бесплатно и приватно; платный — только после явного «да»). Ответ
   propose повторяет identity каждой строки.
5. Просмотр построчно; применяем ТОЛЬКО выбранное:
   `rimloc.translation.apply { identities: <ключи строк из шагов 3–4>,
   expected_revision, session_epoch }`.
6. `rimloc.validation.run` — исправь замечания, предложив правки
   черновиков для затронутых строк (снова propose → apply), или
   объясни их пользователю.
7. `rimloc.export.build { out_dir }` — назови путь перевода-мода и
   количество файлов.
8. Готово: исходный мод не изменён, работа — в проекте RimLoc и
   экспорте. Чтобы увидеть в игре, пользователь устанавливает/включает
   переведённый мод в списке модов RimWorld (его решение — RimLoc игру
   не трогает).
