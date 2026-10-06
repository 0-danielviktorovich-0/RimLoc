---
title: Советы
---

# Советы и лайфхаки

- Для обычного перевода используйте **desktop project workflow**, CLI — для automation/format-specific задач.
- Original game/Workshop/mod source держите read-only.
- Для записи используйте отдельный project/output и <code>--dry-run</code>.
- PO опционален: нужен для Poedit/CAT handoff, а не для каждого перевода.
- После import/bulk edit/TM/AI снова запускайте validation.
- AI/provider output считайте draft до review.
- При обновлении source используйте existing/update workflow, не начинайте заново.
- Target locale должны быть изолированы друг от друга.
- Финальный результат обязательно проверяйте в RimWorld.
- В багрепорте указывайте exact artifact/commit identity и sanitized diagnostics.

Для разработчиков: game-specific semantics должны жить за <code>LocalizationAdapter</code>, а не в generic core.
