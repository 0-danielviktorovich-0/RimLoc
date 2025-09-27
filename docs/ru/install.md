# Установка RimLoc CLI

Где и как установить RimLoc: из crates.io, из релизов GitHub (стабильные и dev), а также из исходников.

## Вариант 1: Cargo (crates.io)

Самый простой способ, если уже установлен Rust.

```bash
cargo install rimloc-cli
```

Заметки:
- Установите Rust через https://rustup.rs (Windows: запустите rustup‑init.exe).
- Убедитесь, что `~/.cargo/bin` в PATH (после установки откройте новое окно терминала). В Windows добавьте `%USERPROFILE%\.cargo\bin` в PATH или перезапустите PowerShell.

## Вариант 2: GitHub Releases (бинарники)

Страница релизов: https://github.com/0-danielviktorovich-0/RimLoc/releases

### Стабильные релизы

Скачайте ассет вида `rimloc-cli-<tag>-<target>.<ext>` под вашу платформу (выберите последний релиз БЕЗ пометки «Pre-release»):
- Linux (x86_64 GNU): `rimloc-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- Linux (x86_64 musl): `rimloc-cli-<tag>-x86_64-unknown-linux-musl.tar.gz`
- Linux (aarch64 GNU): `rimloc-cli-<tag>-aarch64-unknown-linux-gnu.tar.gz`
- Linux (aarch64 musl): `rimloc-cli-<tag>-aarch64-unknown-linux-musl.tar.gz`
- macOS (x86_64): `rimloc-cli-<tag>-x86_64-apple-darwin.tar.gz`
- macOS (arm64): `rimloc-cli-<tag>-aarch64-apple-darwin.tar.gz`
- Windows (x86_64): `rimloc-cli-<tag>-x86_64-pc-windows-msvc.zip`
- Windows (arm64): `rimloc-cli-<tag>-aarch64-pc-windows-msvc.zip`

### Dev‑пререлизы (ночные сборки)

Скачайте ассет `rimloc-cli-dev-latest-<target>.<ext>` (лежит в последнем dev pre‑release):
- Linux (x86_64 GNU): `rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz`
- Linux (x86_64 musl): `rimloc-cli-dev-latest-x86_64-unknown-linux-musl.tar.gz`
- Linux (aarch64 GNU): `rimloc-cli-dev-latest-aarch64-unknown-linux-gnu.tar.gz`
- Linux (aarch64 musl): `rimloc-cli-dev-latest-aarch64-unknown-linux-musl.tar.gz`
- macOS (x86_64): используйте ассет с тегом `rimloc-cli-<tag>-x86_64-apple-darwin.tar.gz`, если нет dev‑latest
- macOS (arm64): используйте ассет с тегом `rimloc-cli-<tag>-aarch64-apple-darwin.tar.gz`, если нет dev‑latest
- Windows (x86_64): `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`
- Windows (arm64): `rimloc-cli-dev-latest-aarch64-pc-windows-msvc.zip`

Подсказка: если под вашу платформу нет `dev-latest`, используйте соответствующий ассет с тегом (то же имя без префикса `dev-latest`).

## Проверка хэша

Для каждого файла есть `.sha256`.

Linux/macOS:

```bash
sha256sum -c rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Windows (PowerShell):

```powershell
Get-FileHash .\rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Get-Content .\rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip.sha256
```

## Проверка подписи (опционально)

Продвинутые пользователи могут проверить `.sig`/`.pem` через `cosign` (Linux/macOS; на Windows доступность подписи зависит от CI):

```bash
cosign verify-blob \
  --cert rimloc-cli-<tag>-<target>.<ext>.pem \
  --signature rimloc-cli-<tag>-<target>.<ext>.sig \
  rimloc-cli-<tag>-<target>.<ext>
```

## Распаковка и запуск

- Linux/macOS:

```bash
tar -xzf rimloc-cli-*.tar.gz -C /tmp
/tmp/rimloc-cli --help
```

- Windows:

```powershell
Expand-Archive -Path .\rimloc-cli-*.zip -DestinationPath .\rimloc
.\rimloc\rimloc-cli --help
```

Подробные пошаговые инструкции для Windows/macOS/Linux (как открыть терминал, что делать при мгновенном закрытии окна и пр.): см. [Запуск скачанной сборки](install_run.md).

## Вариант 3: Сборка из исходников (все ОС)

Полезно, если хотите собрать из Git или получить самые свежие изменения.

1) Установите Rust (через rustup): https://rustup.rs
2) Клонируйте репозиторий и соберите CLI:

```bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
cargo build -p rimloc-cli --release
```

3) Запустите собранный бинарник:

- Linux/macOS:

```bash
./target/release/rimloc-cli --version
# По желанию: установить в ~/.local/bin
install -Dm755 ./target/release/rimloc-cli ~/.local/bin/rimloc-cli
```

- Windows (PowerShell):

```powershell
.\u005ctarget\release\rimloc-cli.exe --version
# По желанию: скопировать в папку из PATH, например %USERPROFILE%\bin
Copy-Item .\target\release\rimloc-cli.exe "$env:USERPROFILE\bin\rimloc-cli.exe"
```

## Через gh (продвинуто)

Если установлен GitHub CLI:

```bash
REPO=0-danielviktorovich-0/RimLoc
TARGET=x86_64-unknown-linux-gnu
TAG=$(gh release list -R "$REPO" --limit 20 --json tagName,isPrerelease,createdAt \
  --jq '[.[] | select(.isPrerelease==true and (.tagName|test("-dev\\.")))] | sort_by(.createdAt) | last.tagName')
gh release download -R "$REPO" --tag "$TAG" --pattern "rimloc-cli-dev-latest-$TARGET.*" -D .
```
