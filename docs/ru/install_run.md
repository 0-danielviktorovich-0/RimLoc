---
title: Запуск скачанной сборки
---

# Запуск скачанной сборки

RimLoc CLI — это консольная программа. Если запустить файл двойным кликом, окно может на миг открыться и сразу закрыться. Нужно запускать из терминала и передавать команды/пути.

## Что такое терминал?

- Windows: PowerShell или Windows Terminal
- macOS: приложение «Terminal»
- Linux: «Terminal» (например, GNOME Terminal, Konsole)

Откройте терминал в папке, куда вы распаковали `rimloc-cli`, и выполните команды ниже.

## Windows (PowerShell)

1) Распакуйте ZIP (например, `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`). Внутри будет `rimloc-cli.exe`.
2) Откройте PowerShell в этой папке:
   - Проводник → перейдите в папку → в адресной строке введите `powershell` → Enter; или
   - Правый клик по фону папки → «Open in Terminal».
3) Запустите CLI:

```powershell
.\u005crimloc-cli.exe --help
.rimloc-cli.exe --version
```

4) Базовые примеры (удобно положить папку мода рядом с EXE, либо укажите полный путь):

```powershell
# Список единиц перевода (текстом)
.rimloc-cli.exe scan --root .\MyMod --format text

# Проверка XML перевода
.rimloc-cli.exe validate --root .\MyMod

# Экспорт единого PO
.rimloc-cli.exe export-po --root .\MyMod --out-po .\MyMod.ru.po --lang ru

# Предпросмотр сборки отдельного мода‑перевода из PO
.rimloc-cli.exe build-mod --po .\MyMod.ru.po --out-mod .\MyMod_RU --lang ru --dry-run
```

Подсказка: запускайте из PowerShell, а не двойным кликом — так вы увидите вывод и ошибки.

Добавить в PATH (необязательно): создайте `%USERPROFILE%\bin`, скопируйте туда `rimloc-cli.exe` и добавьте эту папку в «Переменные среды» → `Path`.

## macOS (Terminal)

1) Распакуйте `tar.gz` (через Finder или терминал):

```bash
tar -xzf rimloc-cli-*.tar.gz -C "$HOME/Downloads/rimloc"
cd "$HOME/Downloads/rimloc"
```

2) Проверьте, что есть права на запуск, и снимите карантин при необходимости:

```bash
chmod +x ./rimloc-cli
# Если macOS ругается на безопасность или «невозможно открыть»:
xattr -d com.apple.quarantine ./rimloc-cli 2>/dev/null || true
```

3) Запустите CLI:

```bash
./rimloc-cli --help
./rimloc-cli --version
```

4) Базовые команды:

```bash
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
./rimloc-cli export-po --root ./MyMod --out-po ./MyMod.ru.po --lang ru
./rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
```

Добавить в PATH (необязательно): `install -Dm755 ./rimloc-cli ~/.local/bin/rimloc-cli` и убедитесь, что `~/.local/bin` в PATH, либо скопируйте в `/usr/local/bin` (может потребоваться `sudo`).

## Linux

1) Распакуйте архив и перейдите в папку:

```bash
tar -xzf rimloc-cli-*.tar.gz -C "$HOME/Downloads/rimloc"
cd "$HOME/Downloads/rimloc"
chmod +x ./rimloc-cli
```

2) Запустите CLI:

```bash
./rimloc-cli --help
./rimloc-cli --version
```

3) Базовые команды:

```bash
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
./rimloc-cli export-po --root ./MyMod --out-po ./MyMod.ru.po --lang ru
./rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
```

Если видите «No such file or directory» на старых дистрибутивах — скачайте сборку с `-musl` вместо `-gnu`.

Добавить в PATH (необязательно): `install -Dm755 ./rimloc-cli ~/.local/bin/rimloc-cli`.

## Частые проблемы

- Окно открывается и сразу закрывается: запускайте из терминала, а не двойным кликом.
- «command not found» / «не является внутренней или внешней командой»: запускайте `./rimloc-cli` (Linux/macOS) или `.\u005crimloc-cli.exe` (Windows) из текущей папки либо добавьте в PATH.
- Permission denied (Linux/macOS): `chmod +x ./rimloc-cli`.
- Предупреждение безопасности macOS: разрешите запуск в Настройках → Privacy & Security → «Open Anyway» или снимите карантин `xattr -d com.apple.quarantine ./rimloc-cli`.
- Неверная архитектура: скачайте ассет под ваш процессор/ОС (например, для Apple Silicon — `aarch64-apple-darwin`, для Intel Mac — `x86_64-apple-darwin`).
- Старый glibc на Linux: используйте сборку `-musl`.

## Что дальше

- Посмотрите обзор CLI с командами и флагами: CLI → Overview.
- Отдельные задачи: Scan, Validate, Export/Import, Build Mod.
- Хотите проще обновлять? Установите через crates.io: `cargo install rimloc-cli`.

