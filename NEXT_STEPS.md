# Mizu 0.1.0 — состояние работы и что осталось

_Обновлено: 2026-11-10. Рабочий журнал разработки (не пользовательская документация)._

## Принятые технические решения

| Вопрос | Решение | Почему |
|---|---|---|
| Движок | **Microsoft Edge WebView2** (Chromium, evergreen) через `webview2-com` | Лицензированные H.264/AAC (в стандартных сборках CEF их нет), обновления движка через Windows Update, sandbox + site isolation Chromium, InPrivate-профили, Tracking Prevention, API расширений. CEF/Servo — через тот же слой `mizu-engine` в будущем. |
| UI | HTML/CSS/JS в **отдельных привилегированных WebView** (`mizu://chrome/`, `mizu://flyout/`), окно Win32 без системного заголовка, Mica | Коммерческий вид и анимации; меню/подсказки — отдельное всплывающее окно (flyout), т.к. HTML не может перекрыть дочерний HWND контента. |
| uBlock | Нативный движок **adblock-rust** (Brave, MPL-2.0): сетевые фильтры, `$redirect`, `$removeparam`, косметика, скриптлеты `##+js()`; списки uBlock/EasyList/EasyPrivacy/Peter Lowe скачиваются по HTTPS; ресурсы скриптлетов — brave/adblock-resources (MPL-2.0) через jsdelivr | Не зависит от MV2/MV3; uBlock Origin Lite можно дополнительно поставить как MV3-расширение. |
| Tampermonkey | Встроенный **Mizu Scripts** (совместим с форматом Greasemonkey/Tampermonkey, GM_* API) | Сам Tampermonkey проприетарный, встраивать нельзя; его MV3-версия требует `chrome.userScripts`. |
| Расширения | WebView2 `AddBrowserExtension` (распакованные папки, .zip/.crx) | Официальный API движка. |
| Хранение | SQLite (rusqlite, bundled) + TOML настроек | Миграции, WAL, бэкап, восстановление после повреждения. |
| Async | Без Tokio: всё на UI-потоке (требование WebView2), тяжёлое — в worker-потоках | Меньше зависимостей. |
| Лицензия | **MPL-2.0** (рекомендована пользователю) | Совместима с adblock-rust, файловый copyleft. |

## Готово (компилируется, тесты проходят)

- `mizu-core` — id, omnibox-классификация, пути профиля, internal pages (9 тестов)
- `mizu-settings` — схема всех категорий настроек, атомарное сохранение, санация (5)
- `mizu-storage` — история/закладки/загрузки/разрешения/сессия, миграции, бэкап, восстановление (11)
- `mizu-tabs` — модель вкладок: pin, перетаскивание, закрытые вкладки, снапшоты (10)
- `mizu-privacy` — ContentBlocker (adblock-rust), FilterStore (скачивание списков), ресурсы, permissions, stats (15)
- `mizu-userscripts` — парсер метаданных, @match/@include, хранилище, GM_* bootstrap с токеном (9)
- `mizu-engine` — обёртка WebView2: окружение, View, все события, загрузки с deferral, профиль, расширения (собирается)
- `mizu-media` — диагностика кодеков (Media Foundation) и GPU (DXGI) (1)
- `mizu-ui` — протокол (`protocol.rs`), карта ассетов (`assets.rs`) — **ассеты ещё не написаны, крейт не соберётся**
- `assets/filters/mizu-shield.txt` — встроенный базовый список

## mizu-browser — написано, но ещё НИ РАЗУ не собиралось

Файлы: `main.rs, queue.rs, hot.rs, handler.rs, shortcuts.rs, scripts.rs, shield.rs, win.rs, theme.rs,
single_instance.rs, dialogs.rs, window.rs, flyout.rs, omnibox.rs, downloads.rs, app.rs, tabs.rs,
session.rs, commands.rs, internal.rs`.

## Что осталось сделать (по порядку)

### 1. Довести mizu-browser до компиляции
- [ ] Написать `crates/mizu-browser/src/extensions.rs`: `list/install/toggle/remove/open` (через `View::list_extensions/add_extension/...`
      любого обычного (не private) view; установка — `dialogs::pick_folder` или `pick_file(*.zip;*.crx)`,
      распаковка CRX3 (заголовок `Cr24`, version, header_size) и zip в `<profile>/extensions/<имя>-<hash>`, затем `add_extension`;
      `open` — прочитать `manifest.json` (options_page / options_ui.page / action.default_popup) и открыть `chrome-extension://<id>/<page>` во вкладке).
- [ ] В `App` добавить поле `script_install_tab: Option<ViewKey>` (используется в `session.rs`).
- [ ] В `commands::on_message` вызывать `internal::set_current_rid(envelope.rid)` перед `handle` и сбрасывать после.
- [ ] `app.rs`: в обработке `Queued::DownloadUpdate` убрать unsafe-указатель — использовать поля напрямую
      (`let storage = if private { None } else { Some(&self.storage) }; self.downloads.update(...)`).
- [ ] `tabs.rs::track_download`: убрать открытие панели с фиктивным anchor; пусть chrome сам открывает панель загрузок
      при появлении новой загрузки (если `downloads.open_panel_on_start`).
- [ ] Удалить заглушки-«затычки»: `window.rs::hot_key_for`, `tabs.rs::_assert_window`, `commands.rs::anchor_default`,
      `internal.rs::_unused`; поправить `WM_PAINT` в `window.rs` (замыкание с `?`).
- [ ] `build.rs` + `assets/icons/mizu.ico` + `mizu.rc` (иконка, манифест: PerMonitorV2, Common Controls 6, `supportedOS` Win10/11) через `embed-resource`.
- [ ] Вернуть `default-members = ["crates/mizu-browser"]` в корневой `Cargo.toml`.
- [ ] `cargo build`, исправить ошибки, `cargo clippy --workspace`, `cargo test --workspace`.

### 2. Интерфейс (assets/ui) — ещё не создан
Пути заданы в `crates/mizu-ui/src/assets.rs`. Протокол — `crates/mizu-ui/src/protocol.rs`.
- [ ] `shared/tokens.css` — дизайн-система: палитры Mizu/Washi/Matcha/Sumi/Sakura × light/dark, private-тема,
      акцент (CSS var), плотность (compact/comfortable/spacious), стиль soft/crisp, `prefers-reduced-motion` + флаг анимаций,
      шрифты Segoe UI Variable / Yu Gothic UI (без встраивания шрифтов).
- [ ] `shared/bridge.js` — `mizu.send(cmd, args)`, `mizu.request(cmd, args) → Promise` (rid/reply), `mizu.on(type, fn)`.
- [ ] `shared/theme.js` — применение настроек (`settings` event / ответ `ready`), `_dark` для System.
- [ ] `shared/icons.js` — набор SVG-иконок (линейные, 1.5px), `shared/logo.svg` (капля 水).
- [ ] `shared/pages.css` — общий стиль внутренних страниц.
- [ ] `chrome/` — вкладки (favicon/спиннер/звук, pin, drag-reorder с анимацией, средняя кнопка — закрыть, двойной клик — новая,
      контекстное меню → flyout `tab_menu`), кнопки окна (min/max/close), `app-region: drag`, тулбар (назад/вперёд/обновить/домой),
      omnibox (индикатор защищённости, ввод → `omnibox_input`, стрелки → `omnibox_highlight`, Enter → `omnibox_commit`/`navigate`,
      Esc, звезда закладки, щит с числом блокировок), загрузки (кольцо прогресса), меню; панель закладок; private-индикатор;
      отправка `chrome_metrics` (высота + rect omnibox) при каждом изменении раскладки; тосты.
- [ ] `flyout/` — виды: menu, tab_menu, downloads, shield, bookmark, omnibox, site_info, permission; сообщать `flyout_size`.
- [ ] `pages/` — newtab (часы, поиск, топ-сайты, статистика Shield, восстановление после сбоя), settings (10 категорий,
      списки фильтров, свои правила, allowlist, очистка данных, разрешения сайтов, поисковые системы),
      history (поиск, удаление, очистка за период), bookmarks (дерево, папки, редактирование, перетаскивание, поиск),
      downloads, about (версия движка), media (MF-отчёт + JS-проба `MediaSource.isTypeSupported`/`MediaCapabilities`),
      shield (статистика, топ трекеров, списки), scripts (Mizu Scripts: список, вкл/выкл, удалить, просмотр, установка
      `#install`, создание; Расширения: список, установка, вкл/выкл, удалить, открыть настройки), blocked (страница блокировки + «всё равно открыть»).
- [ ] Только `textContent` (никакого innerHTML с данными), CSP уже задаётся сервером.

### 3. Проверка в реальном приложении
- [ ] Запуск/закрытие, YouTube (видео, H.264/VP9/AV1), GitHub, Discord, Reddit, Google, Wikipedia, Twitch, авторизация (OAuth-попапы
      с `window.opener`), загрузки, полноэкранный режим, приватное окно, восстановление сессии после `taskkill /F`,
      блокировка рекламы (проверить на тестовых страницах), установка userscript с Greasy Fork, установка uBO Lite.
- [ ] Проверить: работает ли прозрачность Mica через WebView2 (если нет — непрозрачный фон по умолчанию),
      top-edge resize, позиционирование flyout на разных DPI, отсутствие кражи фокуса подсказками omnibox.
- [ ] Память/CPU: спящие вкладки (TrySuspend), MemoryUsageTargetLevel.

### 4. Документация (только про реально реализованное)
- [ ] `README.md` (описание, возможности, требования: Windows 10/11 x64 + WebView2 Runtime, сборка, скриншоты),
      `CHANGELOG.md`, `ROADMAP.md` (0.2.0+ с критериями), `LICENSE` (полный текст MPL-2.0),
      `docs/ARCHITECTURE.md`, `SECURITY.md` (модель доверия IPC, guard internal-страниц, sandbox), `CODECS.md`
      (что даёт WebView2, HEVC/AC-3 через системные декодеры, лицензирование), `DEVELOPMENT.md`, `TESTING.md`.
- [ ] Честно указать ограничения 0.1.0: нет менеджера паролей и синхронизации; history навигации вкладок не восстанавливается
      (только URL); `GM_xmlhttpRequest` подчиняется CORS; `@include` regex без `{n}`; generic-косметика (class/id) uBlock не применяется;
      расширения — без кнопок на панели (popup открывается вкладкой); updater — только архитектура (подпись ed25519, манифест).

### Известные моменты для проверки при первой сборке
- `webview2-com 0.39` / `windows 0.62`: возможны расхождения сигнатур в `mizu-browser` (Param<PCWSTR>, Option<HWND>).
- `queue::with_app` из `wndproc` при `WM_SIZE` — проверить, что раскладка не «запаздывает» визуально.
- Сеть в этой среде нестабильна (`raw.githubusercontent.com` недоступен) — поэтому ресурсы скриптлетов берутся с jsdelivr.
