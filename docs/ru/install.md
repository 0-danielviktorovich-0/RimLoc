---
title: Установка
---

# Установка RimLoc

RimLoc сейчас находится в **pre-beta**. Исторические CLI/release artifacts существуют, но свежая React/desktop разработка идёт отдельно и не должна восприниматься как стабильный релиз.

## Что выбрать?

| Цель | Рекомендуемый путь |
| --- | --- |
| Посмотреть самый свежий desktop UI | собрать React candidate из актуального source или использовать явно выданный owner/test artifact |
| Автоматизировать через CLI | собрать текущий source; crates.io подходит, если достаточно старой alpha |
| Установить стабильный desktop release | пока недоступно |

## Собрать актуальный CLI

Установите Rust с <https://rustup.rs>, затем:

~~~bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
cargo build -p rimloc-cli --release
./target/release/rimloc-cli --version
~~~

Windows:

~~~powershell
.\target\release\rimloc-cli.exe --version
~~~

## CLI из crates.io

Если нужен именно опубликованный пакет:

~~~bash
cargo install rimloc-cli
~~~

!!! note
    crates.io может заметно отставать от active development branch. Сначала проверьте версию и не ожидайте от старой alpha текущих pre-beta функций.

## Собрать React desktop candidate

Нужны Rust, Node.js 20+ и системные prerequisites Tauri 2.

~~~bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc/gui/tauri-app/frontend-react
npm ci
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

Во время миграции default Tauri config всё ещё указывает на замороженный Svelte fallback. Для React R1 используйте <code>tauri.react.conf.json</code>.

## GitHub Releases

В репозитории есть исторические alpha/dev pre-release. Это история проекта, а не гарантия свежей React-сборки.

До новой beta:

- для разработки/тестов предпочитайте актуальный source;
- проверяйте commit/build identity любого готового artifact;
- не считайте файл с именем RimLoc GUI.app автоматически свежим React UI.

## macOS Gatekeeper

Development builds могут быть unsigned/not notarized. Открывайте такие artifacts только если собрали их сами либо получили от владельца проекта и проверили hash.

Signing/notarization относится к release-candidate этапу.

## Проверка test artifact

Если owner/test packet содержит SHA-256:

~~~bash
shasum -a 256 "RimLoc GUI.app/Contents/MacOS/RimLoc GUI"
~~~

Сверяйте hash именно с identity-файлом этого artifact.

## Дальше

- [Начало работы](getting-started.md)
- [Desktop GUI](guide/gui.md)
- [CLI](cli/index.md)
- [Security](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md)
