# PLATFORM VERIFICATION MATRIX — rel24-candidate (fcb82d7, 2026-10-08)

Честная матрица установочной готовности по платформам. Не раздувать:
каждая ячейка — что реально проверено и чем.

## macOS (arm64) — PRIMARY, глубоко верифицировано

| Аспект | Статус | Доказательство |
|---|---|---|
| Сборка .app | ✅ | release 33457117/2c50dcb-дерево, `Finished 1 bundle`, DMG-стадия не запускается (контракт C6) |
| .app.zip поставка | ✅ | `e99b750a…` + sibling .sha256 (build-make-appzip.sh, без AppleDouble/`__MACOSX`) |
| Запуск GUI | ✅ | живые прогоны: WDIO 59/59 (embedded WebDriver в том же процессе), dogfood 16 шагов, скриншоты 18 |
| IPC (contract commands) | ✅ | весь WDIO живёт через Tauri IPC: validate/build/export/chat-batch/глоссарий/TM/probe — сотни команд за сессию |
| Файловые операции | ✅ | create/import/apply/build на реальных модах (HugsLib 80 ключей, VEF 672, фикстуры) с артефактами на диске, идемпотентная сборка, write-guard adversarial-тесты |
| Ключевое хранилище | ✅ | keychain feature (apple-native), cross-process restart proof (волна security) |
| gates | ✅ | preflight --gate, release-guard static+runtime, frontend gate (React R1 embedded, svelte fallback refused) |

## Windows — компиляция и тесты, GUI не запускался нативно

| Аспект | Статус | Доказательство / ограничение |
|---|---|---|
| cargo test (windows-latest) | ✅ CI | полный тест-сьют зелёный на каждом пуш (матрица CI) |
| Tauri GUI build | ✅ CI | GUI-крейт компилируется (джоба Tauri GUI build) |
| Запуск GUI + IPC + файловые операции | ❌ не проверялось | нет Windows-раннера; native launch/WebView2/IPC/инсталлятор — не верифицированы |
| Инсталлятор (MSI/NSIS) | ❌ не собирался | бандл-стадия Windows не настроена в поставке |

## Linux — компиляция и тесты, GUI не запускался нативно

| Аспект | Статус | Доказательство / ограничение |
|---|---|---|
| cargo test (ubuntu-latest) | ✅ CI | полный сьют зелёный |
| GUI compile (gtk/webkit deps) | ✅ CI | `libwebkit2gtk-4.1-dev` и т.п. ставятся в CI; крейт компилируется (--all-features) |
| Запуск GUI (X11/Wayland) + IPC | ❌ не проверялось | нет Linux-раннера с дисплеем; AppImage/deb — не собирался |

## Честный вывод

- Production-ready платформа: **macOS arm64** (глубоко доказана, включая
  файловые операции и IPC).
- Windows/Linux: **компилируются и проходят полный тест-сьют в CI**;
  нативный GUI-launch/IPC/инсталляторы — верифицированные пробелы,
  заявлять их готовыми нельзя. Для beta: отдавать macOS-пакет +
  CI-исходники; Windows/Linux — «компилируется, GUI не верифицирован».
- Закрытие пробелов требует раннеров с дисплеем (Windows VM / Linux
  Xvfb-раннер) — отдельная инфраструктурная задача, не часть этого RC.
