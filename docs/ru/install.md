# Установка RimLoc CLI

Где взять готовые сборки и как их проверить.

## Быстрая загрузка

- Страница релизов (pre-release): https://github.com/0-danielviktorovich-0/RimLoc/releases
- Скачайте файл `rimloc-cli-dev-latest-<target>.<ext>` под вашу платформу:
  - Linux (x86_64 GNU): `rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz`
  - Linux (x86_64 musl): `rimloc-cli-dev-latest-x86_64-unknown-linux-musl.tar.gz`
  - Linux (aarch64 GNU): `rimloc-cli-dev-latest-aarch64-unknown-linux-gnu.tar.gz`
  - Linux (aarch64 musl): `rimloc-cli-dev-latest-aarch64-unknown-linux-musl.tar.gz`
  - macOS (x86_64): `rimloc-cli-<tag>-x86_64-apple-darwin.tar.gz` (если нет dev-latest)
  - macOS (arm64): `rimloc-cli-<tag>-aarch64-apple-darwin.tar.gz` (если нет dev-latest)
  - Windows (x86_64): `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`
  - Windows (arm64): `rimloc-cli-dev-latest-aarch64-pc-windows-msvc.zip`

Примечание: dev-latest лежат в последнем dev pre-release. Если для вашей платформы нет `dev-latest`, используйте ассет с тегом (без `dev-latest`).

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
.\rimloc\rimloc-cli.exe --help
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

