<!-- mandate_id: g5-source-inspector | wave: G5-W7 | scope: read-only source viewer, one-click actions, external editor -->
# МАНДАТ: Source Inspector / File Navigation / External Editor (G5-W7, 2026-09-24)

Сжатая персистенция. Инвариант: **исходники RimWorld/Workshop/модов — read-only по
умолчанию**; RimLoc не превращается в код-редактор.

1. **Source location — first-class**: canonical SourceEntry несёт (где доступно) effective
   file, logical path, line/column (если гарантировано), XML/node path, def type/name,
   identity, версию RW, effective provenance. Не выдумывать line numbers.
2. **Действия над записью**: Open source · Reveal in Finder/Explorer · Open in external
   editor (file+line+column, graceful fallback) · Copy path/location.
3. **Detail panel**: таб SOURCE (файл, локация, node-контекст, provenance, действия);
   advanced свёрнут.
4. **Inline excerpt**: структурированный контекст (ThingDef→defName→label→related), не
   «N строк».
5. **Read-only Source Viewer**: подсветка, номера строк, jump-to, поиск, folding, copy,
   breadcrumb, highlight текущей записи. Полное редактирование НЕ приоритет.
6. **Source Browser**: advanced (Advanced panel/Command Palette): mod → version/load
   folder → категория → файлы; через effective-source resolver — ACTIVE vs shadowed
   кандидаты (Common/1.5/1.6).
7. **«Why this source?»**: переиспользовать provenance канон-резолвера (версия/LoadFolders/
   precedence), никакой GUI-собственной логики precedence.
8. **Settings → External editor**: detect/select/test (system default, Zed, VS Code,
   Custom); запуск структурный (executable+args, БЕЗ shell-строк из путей) — защита от
   injection/malformed/untrusted.
9. **Контекстное меню записи**: open original/effective, reveal, external editor, copy
   location, open generated output, show candidates — только осмысленные для записи.
10. **Compare**: original source ↔ canonical translation ↔ generated output (read-only;
    output — не второй источник правды).
11. **Политика мутаций**: не редактировать молча установки/Workshop/чужие сорцы; правки
    сорцов — через внешний редактор; вывод RimLoc — в контролируемых output-локациях.
12. **External change detection** (future-ready): «Source changed externally. [Rescan][Show
    changes][Ignore]» — без агрессивного watcher сейчас.
13. **Двусторонняя навигация**: entry→source location и source-node→entry (в пределах
    резолвимых идентичностей).
14. **TKey/multi-context**: Primary location + Other usages — не притворяться одним
    источником.
15. **Command palette**: open source / open in editor / reveal / browser / copy location /
    compare — в remappable-систему, без новых фиксированных шорткотов по умолчанию.
16. **Скоуп реализации**: сейчас — read-only viewer/действия/архитектура внешнего
    редактора; deferred — editing/LSP/completion/refactoring. Технология — лёгкий
    read-only подход, не копирование IDE-движков.
17. **UI contract**: Source Browser — capability через UI-контракт, не Svelte-связка.
18. **Мок-сценарии**: source/simple | tkey-multi-context | version-override |
    generated-output | external-change | missing-file.
19. **Live acceptance после фриза**: реальный мод → Open Source → точный файл+нода;
    Reveal; внешний редактор; whole-game/DLC записи. Мок-успех недостаточен.
20. **Security-тесты**: пробелы/Unicode/сломанные/чужие/удалённые пути, traversal-подобный
    metadata, custom editor args — не реинтродутировать удалённые open/write/shell
    поверхности.
21. **UX-принцип**: один клик от записи к источнику; новичок не обязан знать ФС.

Принять в W-волны: W7 = source tab + actions + viewer mock (после W3/W4), live-bind после
фриза. Доки: не подразумевать редактируемость исходников.
