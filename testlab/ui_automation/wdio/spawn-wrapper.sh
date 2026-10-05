#!/bin/bash
# wdio appBinaryPath wrapper (frontier spike 2026-09-30; window correction
# 2026-09-30 — owner directive: NO off-screen/borderless parking).
# Запускает automation-бинарь КАК ОБЫЧНОЕ ВИДИМОЕ ОКНО. С P0-политики
# (2026-10-01, владелец: НОЛЬ активаций) restore-шаги не нужны: форки
# wry/tao (см. корневой Cargo.toml [patch.crates-io]) убрали все
# активаторы стека — запуск и WDIO-сессия не трогают frontmodest.
# Плагин window-state в автоматизационных сессиях не активен: фрейм
# прогона эфемерен и не затирает пользовательский.
# Использование: spawn-wrapper.sh <путь-к-бинарю>
set -u
BIN="${1:-/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/target/debug/rimloc-gui}"
# Эфемерный фрейм прогона: правый-низ visible frame (вне зоны кликов
# владельца), вычисляется из реального дисплея — не off-screen (директива
# про окна §2/§3), не персистится (window-state off в auto-сессиях).
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
