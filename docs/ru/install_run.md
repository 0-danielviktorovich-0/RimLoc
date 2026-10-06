---
title: Запуск скачанного CLI
---

# Запуск скачанного CLI

Эта страница относится только к standalone **CLI** binary. Desktop application — отдельное Tauri-приложение.

!!! warning "Pre-beta artifacts"
    Исторические GitHub alpha/dev artifacts могут заметно отставать от текущей React/Rust линии. Проверяйте version/commit.

## Windows

Откройте PowerShell в папке с <code>rimloc-cli.exe</code>:

~~~powershell
.\rimloc-cli.exe --version
.\rimloc-cli.exe --help
~~~

Базовые безопасные проверки:

~~~powershell
.\rimloc-cli.exe scan --root .\MyMod --format text
.\rimloc-cli.exe validate --root .\MyMod
~~~

## macOS / Linux

~~~bash
chmod +x ./rimloc-cli
./rimloc-cli --version
./rimloc-cli --help
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
~~~

Если macOS блокирует unsigned development binary, используйте сборку, которую собрали сами, или project artifact с проверяемым hash/build identity. Не отключайте системную защиту глобально.

## Опциональный PO workflow

Только если нужен CAT handoff:

~~~bash
./rimloc-cli export-po --root ./MyMod --out-po ./work/MyMod.po --lang ru
~~~

PO не нужен для обычной desktop project model.

## Build без PO

Если translated Languages XML уже существует:

~~~bash
./rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

## Частые проблемы

- command not found: используйте <code>./</code> (macOS/Linux) или <code>.\</code> (Windows);
- permission denied: <code>chmod +x</code>;
- wrong architecture: нужна сборка под вашу CPU/OS;
- странное поведение: сверяйте <code>--version</code>/commit с версией документации.

Для самого свежего состояния [соберите из исходников](install.md).
