# Novice walkthrough — translate a mod to Russian (RU original below)

Every step names the MCP tool that does the work. If any tool is
missing from `rimloc.capabilities`, stop and tell the user.

1. `rimloc.projects.list` — see what exists. For a new mod, the user
   gives the mod folder; `rimloc.project.open` builds the project.
   Read `rimloc.project.snapshot`: source locale, target locale
   (set to Russian if the user asked), revision, epoch.
2. Confirm the source tree is read-only (RimLoc's contract; say it).
3. `rimloc.entries.search` with a term the user cares about, or a
   filter for untranslated entries. Show 20–50 rows max.
4. Draft translations: `rimloc.translation.propose` per entry (local
   provider = free and private; paid provider only on explicit OK).
5. Review with the user, row by row. Apply ONLY what they chose:
   `rimloc.translation.apply { identities: [...], expected_revision:
   <from the snapshot>, session_epoch: <same> }`.
6. `rimloc.validation.run` — fix or explain findings.
7. `rimloc.export.build { out_dir: <user's choice> }` — report the
   produced translation mod path and its file count.
8. Done. The source mod is byte-identical; all work lives in the
   RimLoc project and the export.

# Пошаговый разбор — перевод мода на русский

1. `rimloc.projects.list` — что уже есть. Новый мод: пользователь
   называет папку, `rimloc.project.open` создаёт проект. Прочитай
   `rimloc.project.snapshot`: исходный язык, целевой (русский),
   ревизию и эпоху сессии.
2. Подтверди: исходники мода только читаются, RimLoc в них не пишет.
3. `rimloc.entries.search` по термину пользователя или фильтру
   непереведённого. Покажи не больше 20–50 строк.
4. Черновики: `rimloc.translation.propose` (локальный провайдер —
   бесплатно и приватно; платный — только после явного «да»).
5. Просмотр построчно; применяем ТОЛЬКО выбранное:
   `rimloc.translation.apply { identities, expected_revision, session_epoch }`.
6. `rimloc.validation.run` — исправь или объясни замечания.
7. `rimloc.export.build { out_dir }` — назови путь перевода-мода.
8. Готово: исходный мод не изменён, работа — в проекте RimLoc и экспорте.
