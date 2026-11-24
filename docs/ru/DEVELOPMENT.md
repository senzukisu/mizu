# Разработка Mizu

Как собрать Mizu, безопасно его менять и выпустить версию. Как связаны части — в [Архитектуре](ARCHITECTURE.md);
тесты — в [Тестировании](TESTING.md).

**Содержание:** [Шпаргалка](#шпаргалка) · [Подготовка](#подготовка) · [Запуск](#запуск) · [Где что лежит](#где-что-лежит) ·
[Рецепты](#рецепты) · [Правила кода](#правила-кода) · [Перед коммитом](#перед-коммитом) · [Выпуск](#выпуск) ·
[Ключ подписи](#ключ-подписи)

---

## Шпаргалка

| Задача | Команда |
|---|---|
| Собрать / запустить (debug) | `cargo build` / `cargo run` |
| Запустить с временным профилем | `MIZU_USER_DATA="$TEMP/mizu-dev" MIZU_LOG=mizu=debug cargo run` |
| Открыть адрес в запущенной копии | `cargo run -- https://example.com` |
| Собрать рядом с запущенной копией | `CARGO_TARGET_DIR=target/alt cargo run` |
| Проверить скрипты интерфейса | `node scripts/check-ui.mjs` |
| Линтер | `cargo clippy --workspace --all-targets -- -D warnings` |
| Тесты | `cargo test --workspace` |
| Сборка выпуска | `powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "…"` |
| Освободить место | `cargo clean` (debug-сборка ~6 ГБ, release ~2 ГБ) |

---

## Подготовка

- Windows 10/11 x64.
- Rust stable 1.85 или новее, набор `x86_64-pc-windows-msvc`.
- Visual Studio Build Tools → «Разработка классических приложений на C++» (MSVC и Windows SDK; `rc.exe` собирает
  значок и манифест).
- Microsoft Edge WebView2 Runtime (в Windows 11 уже есть).
- Node.js — для `scripts/check-ui.mjs`.

---

## Запуск

- **Не используйте настоящий профиль при разработке.** `MIZU_USER_DATA` направляет Mizu в другую папку; удалите её,
  чтобы начать с чистого листа.
- **Журналы:** `<данные>/logs/mizu.ГГГГ-ММ-ДД.log`. `MIZU_LOG=mizu=debug` добавляет подробностей. В журнале есть
  предупреждения `slow UI-thread work` (дольше 50 мс) и `slow page load` (дольше 8 с).
- **Второй запуск** передаёт свои адреса уже работающей копии с тем же профилем.
- **Cargo не может заменить запущенный `mizu.exe`.** Закройте его или соберите в другую папку через
  `CARGO_TARGET_DIR`.
- **DevTools для интерфейса Mizu:** в debug-сборке правый щелчок по панели или всплывающей панели → «Проверить».
- **Удобные адреса**, чтобы открыть нужное состояние без щелчков: `mizu://settings/#q=cookies`,
  `mizu://newtab/#customize`, `mizu://newtab/#folder=<id>`, `mizu://welcome/#2`.

---

## Где что лежит

| Хотите изменить… | Смотрите |
|---|---|
| порядок вкладок, закрепление, группы, пространства, дерево | `crates/mizu-tabs` (чистая логика с тестами) |
| окна, раскладку, разделённый экран | `crates/mizu-browser/src/window.rs` |
| открытие, закрытие, активацию вкладок, их события | `crates/mizu-browser/src/tabs.rs` |
| команды панели и всплывающих панелей | `crates/mizu-browser/src/commands.rs` |
| данные для страниц Mizu (`mizu://…`) | `crates/mizu-browser/src/internal.rs` |
| настройки и их значения по умолчанию | `crates/mizu-settings/src/schema.rs` |
| историю, закладки, плитки новой вкладки, импорт | `crates/mizu-storage` |
| блокировку контента | `crates/mizu-privacy` + `crates/mizu-browser/src/scripts.rs` (части в странице) |
| всё, что связано с WebView2 / COM | `crates/mizu-engine` (только этот крейт его касается) |
| панель, полосу вкладок, боковую панель | `assets/ui/chrome/` |
| меню и всплывающие панели | `assets/ui/flyout/` |
| страницы Mizu | `assets/ui/pages/` |
| цвета, шрифты, перевод, значки | `assets/ui/shared/` (`tokens.css`, `i18n.js`, `icons.js`) |

---

## Рецепты

### Добавить команду (интерфейс → браузер)

1. Добавьте вариант в `Command` в `crates/mizu-ui/src/protocol.rs` (serde: `{"cmd":"имя_в_snake_case", …}`).
   Для необязательных полей используйте `#[serde(default)]`, чтобы старые вызовы продолжали работать.
2. Если отправлять её могут только панель и всплывающие панели, добавьте её в `Command::is_chrome_only`.
3. Обработайте её в `commands.rs` (`handle`) или, для данных страниц, в `internal.rs` (`handle_data`).
4. Вызовите из JS: `mizu.send('имя', {…})` или `await mizu.request('имя', {…})`, если нужен ответ.

### Добавить страницу (`mizu://something`)

1. Добавьте вариант в `InternalPage` в `crates/mizu-core/src/internal.rs` (хост и заголовок).
2. Создайте `assets/ui/pages/something.html` и `something.js` и зарегистрируйте оба в `surface()` в
   `crates/mizu-ui/src/assets.rs`.
3. Используйте общие файлы (`/_/tokens.css`, `/_/pages.css`, `/_/bridge.js`, `/_/i18n.js`, `/_/icons.js`).

### Добавить настройку

1. Добавьте поле и значение по умолчанию в нужную структуру в `crates/mizu-settings/src/schema.rs`; плохие значения
   исправляйте в `sanitize`.
2. Покажите её в `assets/ui/pages/settings.js` (помощники `toggle`, `setting`, `seg`, `select`); `save()` сохраняет.
3. Если она нужна коду в UI-потоке на горячем пути, скопируйте её в `hot::Hot` там, где применяются настройки
   (`app.rs`).

### Добавить текст интерфейса

- Пишите его по-английски в коде.
- Добавьте русскую строку в словарь в `assets/ui/shared/i18n.js`. Текст с числами или именами — в `PATTERNS` как
  регулярное выражение.
- Данные сайтов (заголовки, адреса) помечайте `translate="no"`.
- `scripts/check-ui.mjs` не пропустит повторяющиеся ключи словаря.

### Стили

Используйте переменные из `shared/tokens.css` (`--surface`, `--ink-2`, `--accent`, `--r-md` …), а не постоянные
цвета — тогда работают все палитры и тёмная тема. Никогда не используйте `innerHTML` с данными; собирайте элементы
через `h()` из `bridge.js`.

---

## Правила кода

- `cargo fmt`; clippy без предупреждений.
- `Result` и `?`. `unwrap()`/`expect()` — только в тестах или на настоящих инвариантах, с комментарием.
- У каждого блока `unsafe` есть комментарий `// SAFETY:`.
- Новая зависимость — только с объяснением в PR: что она заменяет, лицензия, поддержка.
- Файлы держите сфокусированными; модули длиннее ~600–700 строк делите.
- Время от времени удаляйте мёртвый код (`cargo clippy -- -W dead_code`) и лишние зависимости (`cargo machete`).

---

## Перед коммитом

```bash
node scripts/check-ui.mjs
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

`check-ui.mjs` проверяет синтаксис всех скриптов интерфейса, повторяющиеся ключи перевода и глобальные имена, которые
конфликтуют между общими скриптами и скриптами страниц (из-за конфликта страница остаётся пустой). CI выполняет все
три проверки при каждом push (`.github/workflows/ci.yml`). Коммитьте, только когда проходят все три.

---

## Выпуск

1. **Версия** — укажите `X.Y.Z` в:
   - `Cargo.toml` (workspace);
   - `crates/mizu-browser/resources/mizu.rc` и `mizu.manifest`;
   - `crates/mizu-setup/resources/setup.rc` и `setup.manifest`;
   - имени установщика в `README.md` и `docs/ru/README.md`.
2. **Документация** — раздел в `CHANGELOG.md` и `docs/ru/CHANGELOG.md`; руководство и план развития (оба языка), если
   изменились возможности.
3. **Проверка** — три команды выше и [ручной список](TESTING.md#проверка-перед-выпуском).
4. **Коммит и push.**
5. **Сборка:**

   ```bash
   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "Короткое описание выпуска"
   ```

   В `dist/` появятся `MizuSetup-X.Y.Z.exe`, `Mizu-X.Y.Z-portable-x64.zip`, `mizu-update.json` и
   `mizu-update.json.sig`.
6. **Проверьте установщик:** тихая установка (`MizuSetup-X.Y.Z.exe /S`), запуск с тестовым профилем, затем
   `uninstall.exe --uninstall /S`. Учтите существующий ярлык в меню «Пуск»: установщик его заменит.
7. **Опубликуйте** выпуск `vX.Y.Z` на GitHub со всеми четырьмя файлами. Установленные копии найдут его через
   `releases/latest/download/mizu-update.json` — откройте этот адрес и убедитесь, что там новая версия.
8. **После:** убедитесь, что CI зелёный; `cargo clean`, если нужно место.

---

## Ключ подписи

Манифесты обновлений подписываются ключом ed25519 в `%USERPROFILE%\.onibi\mizu-release.key` (`-Key` задаёт другой).

- **Никогда не коммитьте его** (`*.key` в git игнорируется). Храните копию в надёжном месте: если ключ потерян,
  установленные копии не смогут проверять обновления.
- `mizu-sign keygen <путь>` создаёт новый ключ и печатает открытый ключ, который вписывается в `RELEASE_PUBLIC_KEY` в
  `crates/mizu-update/src/lib.rs`.
