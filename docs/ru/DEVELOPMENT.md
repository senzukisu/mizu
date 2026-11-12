# Разработка Mizu

## Что нужно

- Windows 10/11 x64.
- Rust stable 1.85 или новее с набором `x86_64-pc-windows-msvc`.
- Visual Studio Build Tools, компонент «Разработка классических приложений на C++» (MSVC и Windows SDK; `rc.exe` нужен
  для значка и манифеста).
- Microsoft Edge WebView2 Runtime (в Windows 11 уже есть).
- По желанию — Node.js, только для `node --check` UI-скриптов.

## Сборка и запуск

```bash
cargo build
```

```bash
cargo run
```

Во время разработки используйте отдельный профиль, чтобы не трогать настоящие данные:

```bash
MIZU_USER_DATA="$TEMP/mizu-dev" MIZU_LOG=mizu=debug cargo run
```

Пока копия Mizu запущена из `target\debug`, cargo не может её заменить. Для проверки рядом собирайте в другую папку:

```bash
CARGO_TARGET_DIR=target/alt cargo run
```

Журнал предупреждает о долгой работе UI-потока (`slow UI-thread work`, дольше 50 мс) и медленных страницах
(`slow page load`, дольше 8 с).

Журналы пишутся в `<папка данных>/logs/mizu.ГГГГ-ММ-ДД.log`. Второй запуск передаёт адреса уже работающему экземпляру,
это удобно для проверки: `cargo run -- https://example.com`.

Релизная сборка (thin LTO, без отладочных символов):

```bash
cargo build --release
```

Результаты сборки лежат в `target/` и занимают много места: отладочная сборка — около 6 ГБ, релизная — около 2 ГБ.
Удаляйте папку, когда нужно место; следующая сборка просто займёт на несколько минут больше:

```bash
cargo clean
```

## Устройство проекта

Подробно — в [ARCHITECTURE.md](ARCHITECTURE.md). Коротко:

- Чистая логика живёт в крейтах, не зависящих от платформы, и покрыта модульными тестами.
- С COM работает только `mizu-engine`, и у каждого блока `unsafe` там есть комментарий `// SAFETY:`.
- Интерфейс лежит в `assets/ui` и вшивается в программу через `mizu-ui::assets` (новые файлы нужно добавить в этот список).

## Разработка интерфейса

- `shared/tokens.css` содержит токены дизайна. Используйте переменные (`--surface`, `--ink-2`, `--accent`, `--r-md` …)
  и никогда не задавайте цвета напрямую, иначе сломаются палитры и тёмная тема.
- `shared/bridge.js` даёт `mizu.send`, `mizu.request`, `mizu.on` и помощник `h()` для DOM. Никогда не используйте
  `innerHTML` с данными.
- Новой команде нужны вариант в `mizu-ui::protocol::Command` и обработчик в `mizu-browser::commands` или
  `mizu-browser::internal`. Решите, только ли она для chrome (`Command::is_chrome_only`).
- В отладочных сборках инструменты разработчика работают и для chrome, и для flyout (правый щелчок → «Проверить»).
- **Перевод:**
  - текст интерфейса пишется по-английски;
  - русский вариант добавляется в словарь `shared/i18n.js`, а для текста с числами или именами — шаблон;
  - данные сайтов помечаются `translate="no"`, чтобы их никогда не переводили.
- **Установщик.** Страница установщика — `crates/mizu-setup/src/ui.html`, логика — `setup.rs`. Чтобы проверить его с
  отладочным браузером внутри, соберите так:

  ```bash
  MIZU_SETUP_PAYLOAD="$PWD/target/debug/mizu.exe" cargo build -p mizu-setup
  ```

## Правила кода

- `cargo fmt` и `cargo clippy --workspace` без предупреждений.
- Предпочитайте `Result` и `?`. `unwrap()`/`expect()` — только в тестах или для настоящих инвариантов, с комментарием.
- Никаких новых зависимостей без объяснения в описании PR: что заменяет, лицензия, поддерживается ли.
- Файлы держите сфокусированными; модули длиннее 600–700 строк делите.
- Время от времени убирайте мёртвый код (`cargo clippy -- -W dead_code`) и лишние зависимости (`cargo machete` или
  `cargo udeps`).

## Выпуск версии

1. Обновите версию в файлах:
   - корневой `Cargo.toml`;
   - `crates/mizu-browser/resources/mizu.rc` и `mizu.manifest`;
   - `crates/mizu-setup/resources/setup.rc` и `setup.manifest`;
   - `CHANGELOG.md` и `docs/ru/CHANGELOG.md`.
2. Запустите `cargo test --workspace` и пройдите ручной чек-лист из [TESTING.md](TESTING.md).
3. Соберите всё:

   ```bash
   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "Коротко о выпуске"
   ```

   В `dist/` появятся `MizuSetup-X.Y.Z.exe`, `Mizu-X.Y.Z-portable-x64.zip`, `mizu-update.json` и `mizu-update.json.sig`.
4. Поставьте тег `vX.Y.Z` и создайте выпуск на GitHub со всеми четырьмя файлами. Установленные копии находят обновление
   по адресу `releases/latest/download/mizu-update.json`.

### Ключ подписи

Манифесты обновлений подписываются ключом ed25519 из `%USERPROFILE%\.onibi\mizu-release.key` (другой путь — через `-Key`).
- Ключ **никогда** не попадает в репозиторий; `*.key` игнорируется git.
- `mizu-sign keygen <путь>` создаёт ключ и печатает открытый ключ. Его нужно вписать в `RELEASE_PUBLIC_KEY` в
  `crates/mizu-update/src/lib.rs`.
- Если ключ сменить, старые версии больше не смогут проверить обновления. Храните резервную копию в надёжном месте.
