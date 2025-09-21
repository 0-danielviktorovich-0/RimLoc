# -----------------------------------------------------------------------------
# RimLoc — строки только для интеграционных тестов CLI
# Файл: crates/rimloc-cli/i18n/ru/rimloc-tests.ftl
# ПРИМЕЧАНИЕ:
# • Эти сообщения используются только тестами (не попадают к конечным пользователям).
# • Ключи должны оставаться стабильными; значения можно менять.
# • Предпочитайте описательные сгруппированные ключи: test-validate-*, test-build-*, test-import-* и т.п.
# -----------------------------------------------------------------------------

test-binary-built = бинарник rimloc-cli должен собираться через cargo
test-tempdir = tempdir
test-outpo-exist = файл out.po должен существовать
test-outpo-not-empty = файл out.po не должен быть пустым

# validate (категории и элементы)
test-validate-dup-category = в выводе ожидается категория [duplicate]
test-validate-empty-category = в выводе ожидается категория [empty]
test-validate-ph-category = в выводе ожидается категория [placeholder-check]
test-validate-dup-items = ожидается наличие элементов DuplicateKey
test-validate-empty-items = ожидается наличие элементов EmptyKey
test-validate-ph-items = ожидается наличие элементов Placeholder

# validate (счётчики)
test-validate-atleast-duplicates = ожидается минимум { $min } дубль(ей), найдено { $count }
test-validate-atleast-empty = ожидается минимум { $min } пустой(ых), найдено { $count }
test-validate-atleast-placeholder = ожидается минимум { $min } проблем(а/ы) с placeholder, найдено { $count }

# import-po (одиночный файл, dry run)
test-importpo-expected-path-not-found =
    Ожидался путь, но он не найден.
    stdout=
    ```
    { $out }
    ```
    stderr=
    ```
    { $err }
    ```

# build-mod (структура и содержимое)
test-build-path-must-exist = { $path } должен существовать в собранном моде
test-build-folder-must-exist = папка { $path } должна существовать
test-build-xml-under-path = хотя бы один XML должен быть сгенерирован в { $path }
test-build-about-readable = About/About.xml должен быть читаем
test-build-should-contain-tag = { $path } должен содержать корректный { $tag }

# ftl загрузка (хелперы тестов)
test-ftl-failed-read = не удалось прочитать FTL-файл по пути { $path }

# порядок ключей/ошибки локалей
test-locale-order-mismatch = У локали { $loc } порядок ключей отличается от en. Пожалуйста, приведите порядок к en.

# скан по репозиторию на нелокализованные строки
test-nonlocalized-found =
    Найдены нелокализованные пользовательские строки в репозитории (print/terminate макросы).
    Правило действует для всего проекта, включая тесты.
    Пожалуйста, замените на tr!(...) где это уместно.
    { $offenders }

# предупреждения (специальный маркер для тестов)
test-warn-unsupported-lang = Код языка интерфейса не поддерживается