// Interface translations.
//
// Mizu's UI is written in English; this module translates the rendered interface in place.
// A MutationObserver translates text nodes and title/placeholder/aria-label attributes as the
// UI renders them, so every surface (tab strip, menus, internal pages) is covered without
// threading a t() call through each component. Content that comes from websites (page titles,
// history, bookmarks, file names) is marked translate="no" and never touched.
//
// The language is chosen by the browser and written into <html data-lang> before the page
// is served, so the first paint is already translated.
'use strict';

const i18n = (() => {
  const RU = {
    // ── Tab strip, toolbar ──
    'Add to Mizu': 'Добавить в Mizu',
    'Installing extension…': 'Устанавливаю расширение…',
    'Close tab (Ctrl+W)': 'Закрыть вкладку (Ctrl+W)',
    'New Tab': 'Новая вкладка',
    'New tab (Ctrl+T)': 'Новая вкладка (Ctrl+T)',
    'Unmute tab': 'Включить звук',
    'Mute tab': 'Выключить звук',
    'Stop (Esc)': 'Остановить (Esc)',
    'Reload (Ctrl+R)': 'Обновить (Ctrl+R)',
    'Back (Alt+Left)': 'Назад (Alt+←)',
    'Forward (Alt+Right)': 'Вперёд (Alt+→)',
    'Home (Alt+Home)': 'Домой (Alt+Home)',
    'Connection is secure': 'Защищённое соединение',
    'Connection is not secure': 'Незащищённое соединение',
    'Mizu page': 'Страница Mizu',
    'Local file': 'Локальный файл',
    'Site information': 'Сведения о сайте',
    'Mizu Shield is off for this site': 'Mizu Shield выключен для этого сайта',
    'Downloads (Ctrl+J)': 'Загрузки (Ctrl+J)',
    'Bookmark this page (Ctrl+D)': 'Добавить в закладки (Ctrl+D)',
    'Install this extension in Mizu': 'Установить это расширение в Mizu',
    'Bookmark pages with Ctrl+D or the star to see them here.': 'Добавляйте страницы в закладки (Ctrl+D или звёздочка) — они появятся здесь.',
    'Private': 'Приватно',
    'Restore': 'Восстановить',
    'Maximize': 'Развернуть',
    'Minimize': 'Свернуть',
    'Close': 'Закрыть',
    'Menu': 'Меню',
    'Address and search bar': 'Адресная строка и поиск',
    'Pop-up blocked': 'Всплывающее окно заблокировано',
    'Downloading userscript…': 'Загружаю скрипт…',
    'Filter lists updated': 'Списки фильтров обновлены',

    // ── Main menu, tab menu ──
    'Full screen (F11)': 'Полный экран (F11)',
    'New tab': 'Новая вкладка',
    'New window': 'Новое окно',
    'New private window': 'Новое приватное окно',
    'Zoom': 'Масштаб',
    'Zoom out (Ctrl+−)': 'Уменьшить (Ctrl+−)',
    'Zoom in (Ctrl++)': 'Увеличить (Ctrl++)',
    'History': 'История',
    'Downloads': 'Загрузки',
    'Bookmarks': 'Закладки',
    'Scripts & extensions': 'Скрипты и расширения',
    'Mizu Shield': 'Mizu Shield',
    'Print…': 'Печать…',
    'Developer tools': 'Инструменты разработчика',
    'Task manager': 'Диспетчер задач',
    'Settings': 'Настройки',
    'About Mizu': 'О Mizu',
    'Reload': 'Обновить',
    'Duplicate': 'Дублировать',
    'Unpin': 'Открепить',
    'Pin': 'Закрепить',
    'Unmute site': 'Включить звук',
    'Close tab': 'Закрыть вкладку',
    'Close other tabs': 'Закрыть другие вкладки',
    'Close tabs to the right': 'Закрыть вкладки справа',
    'Reopen closed tab': 'Открыть закрытую вкладку',
    'Close Mizu': 'Закрыть Mizu',
    "Don't ask again": 'Больше не спрашивать',
    'They will be reopened the next time you start Mizu.': 'Они откроются снова при следующем запуске Mizu.',
    'Private tabs and their data will be gone for good.': 'Приватные вкладки и их данные будут удалены безвозвратно.',
    'You can reopen them from History.': 'Их можно открыть снова из истории.',

    // ── Downloads ──
    'File moved or deleted': 'Файл перемещён или удалён',
    'Cancelled': 'Отменено',
    'Interrupted when Mizu closed': 'Прервано при закрытии Mizu',
    'Failed': 'Ошибка',
    'Pause': 'Пауза',
    'Cancel': 'Отмена',
    'Resume': 'Продолжить',
    'Show in folder': 'Показать в папке',
    'Retry': 'Повторить',
    'Nothing downloaded yet.': 'Загрузок пока нет.',
    'All downloads': 'Все загрузки',
    'Paused': 'Приостановлено',
    'Deleted or moved': 'Удалён или перемещён',
    'Interrupted — Mizu was closed during the download': 'Прервано — Mizu был закрыт во время загрузки',
    'No downloads match.': 'Ничего не найдено.',
    'Files you download will appear here.': 'Здесь появятся скачанные файлы.',
    'Open': 'Открыть',
    'Remove from list': 'Убрать из списка',
    'Clear list': 'Очистить список',
    'Search downloads': 'Поиск по загрузкам',

    // ── Shield / site info / prompts ──
    'Turn off for this site': 'Выключить для этого сайта',
    'Turn on for this site': 'Включить для этого сайта',
    'Off': 'Выкл.',
    'On': 'Вкл.',
    'Basic': 'Базовая',
    'Balanced': 'Сбалансированная',
    'Strict': 'Строгая',
    'Shield is off for this site': 'Shield выключен для этого сайта',
    'ads and trackers blocked on this page': 'рекламы и трекеров заблокировано на странице',
    'Content blocking is turned off in Settings.': 'Блокировка контента выключена в настройках.',
    'Tracking prevention (engine)': 'Защита от отслеживания (движок)',
    'Shield dashboard': 'Панель Shield',
    'Private window: nothing from this site is kept after you close it.': 'Приватное окно: после закрытия от этого сайта ничего не останется.',
    'Connection is secure. Information you send to this site is encrypted.': 'Соединение защищено. Данные, которые вы отправляете сайту, зашифрованы.',
    'Connection is not secure. Do not enter passwords or payment details here.': 'Соединение не защищено. Не вводите здесь пароли и платёжные данные.',
    'Reset': 'Сбросить',
    'Site permissions': 'Разрешения сайтов',
    'This page cannot be bookmarked.': 'Эту страницу нельзя добавить в закладки.',
    'Remove': 'Удалить',
    'Done': 'Готово',
    'Bookmark added': 'Закладка добавлена',
    'Name': 'Название',
    'Folder': 'Папка',
    'Block': 'Блокировать',
    'Allow': 'Разрешить',
    'Remember until private windows are closed': 'Запомнить до закрытия приватных окон',
    'Remember this decision': 'Запомнить решение',
    'use your camera': 'использовать камеру',
    'use your microphone': 'использовать микрофон',
    'know your location': 'знать ваше местоположение',
    'show notifications': 'показывать уведомления',
    'see text and images copied to the clipboard': 'видеть текст и изображения в буфере обмена',
    'download multiple files': 'скачивать несколько файлов',
    'control MIDI devices': 'управлять MIDI-устройствами',
    'manage windows on all your displays': 'управлять окнами на всех экранах',
    'use fonts installed on this device': 'использовать шрифты этого устройства',
    'edit files on this device': 'изменять файлы на этом устройстве',
    'play media automatically': 'автоматически воспроизводить медиа',
    'use motion and light sensors': 'использовать датчики движения и освещённости',
    'use a device feature': 'использовать функцию устройства',

    // ── Internal page titles ──
    'Page blocked': 'Страница заблокирована',
    'Media diagnostics': 'Диагностика медиа',

    // ── About ──
    'Mizu': 'Mizu',
    'Engine': 'Движок',
    'Operating system': 'Операционная система',
    'Profile folder': 'Папка профиля',
    'Updates': 'Обновления',
    'The web engine (Microsoft Edge WebView2) is kept up to date automatically by Windows, including security fixes. Mizu 0.1.0 itself does not yet update automatically: new versions are published on the project page.':
      'Веб-движок (Microsoft Edge WebView2) обновляется автоматически через Windows, включая исправления безопасности. Сам Mizu 0.1.0 пока не обновляется автоматически: новые версии публикуются на странице проекта.',
    'Privacy': 'Конфиденциальность',
    'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.':
      'Mizu не отправляет в Onibi Lab никакой аналитики и телеметрии. История, закладки и настройки хранятся только на этом устройстве. Списки фильтров загружаются напрямую у их авторов.',
    'A new way to browse.': 'Новый взгляд на браузер.',

    // ── Blocked page ──
    'Go back': 'Назад',
    'Open anyway': 'Всё равно открыть',
    'Mizu Shield blocked this page': 'Mizu Shield заблокировал эту страницу',
    'A filter list you use marks this address as malicious, deceptive or unwanted.': 'Один из ваших списков фильтров помечает этот адрес как вредоносный, обманный или нежелательный.',
    'If you open it anyway, the site stays allowed until you close Mizu.': 'Если открыть его, сайт останется разрешённым до закрытия Mizu.',

    // ── Bookmarks ──
    'Rename folder': 'Переименовать папку',
    'Edit bookmark': 'Изменить закладку',
    'New folder': 'Новая папка',
    'Add bookmark': 'Добавить закладку',
    'Save': 'Сохранить',
    'Enter a full address, e.g. https://example.com': 'Введите полный адрес, например https://example.com',
    'Edit': 'Изменить',
    'Delete': 'Удалить',
    'No bookmarks match.': 'Закладки не найдены.',
    'This folder is empty. Press Ctrl+D on any page to bookmark it.': 'Папка пуста. Нажмите Ctrl+D на любой странице, чтобы добавить её.',
    'Search bookmarks': 'Поиск по закладкам',
    'Bookmarks bar': 'Панель закладок',
    'Other bookmarks': 'Другие закладки',
    'URL': 'Адрес',

    // ── History ──
    'Today': 'Сегодня',
    'Yesterday': 'Вчера',
    'Remove from history': 'Удалить из истории',
    'No pages match your search.': 'Ничего не найдено.',
    'Pages you visit will appear here. Private windows are never recorded.': 'Здесь появятся посещённые страницы. Приватные окна никогда не записываются.',
    'Last hour': 'Последний час',
    'Last 24 hours': 'Последние 24 часа',
    'Last 7 days': 'Последние 7 дней',
    'Last 4 weeks': 'Последние 4 недели',
    'All time': 'Всё время',
    'Clear history': 'Очистить историю',
    'Time range': 'Период',
    'Cookies, cache and other site data can be cleared in Settings → Privacy.': 'Файлы cookie, кеш и другие данные сайтов можно очистить в Настройки → Конфиденциальность.',
    'Clear': 'Очистить',
    'Clear history…': 'Очистить историю…',
    'Search history': 'Поиск по истории',

    // ── Media ──
    'Yes': 'Да',
    'No': 'Нет',
    'Codec': 'Кодек',
    'Files': 'Файлы',
    'Streaming (MSE)': 'Потоковое (MSE)',
    'Smooth 1080p / hardware': 'Плавно 1080p / аппаратно',
    'Smooth': 'Плавно',
    'System decoders': 'Системные декодеры',
    'Hardware': 'Аппаратно',
    'Software': 'Программно',
    'Slow': 'Медленно',
    'Likely': 'Вероятно',
    'Probing system decoders…': 'Проверяю системные декодеры…',
    'Playback is handled by the Chromium media stack in the engine. "Files" and "Streaming" are what websites can actually use; "System decoders" are Windows Media Foundation decoders the engine can use for hardware acceleration (and for HEVC, which needs a GPU decoder plus the HEVC Video Extensions from the Microsoft Store). Mizu does not bundle any codecs.':
      'Воспроизведением занимается медиастек Chromium в движке. «Файлы» и «Потоковое» — то, что реально доступно сайтам; «Системные декодеры» — декодеры Windows Media Foundation, которые движок использует для аппаратного ускорения (и для HEVC, которому нужен декодер видеокарты и «Расширения для видео HEVC» из Microsoft Store). Mizu не содержит собственных кодеков.',
    'Graphics': 'Графика',
    'Integrated': 'Встроенная',
    'Video codecs': 'Видеокодеки',
    'Audio codecs': 'Аудиокодеки',
    'Container': 'Контейнер',
    'Playable as file': 'Воспроизводится как файл',
    'Containers': 'Контейнеры',
    '4K playback': 'Воспроизведение 4K',
    'none': 'нет',

    // ── New tab ──
    'Good night': 'Доброй ночи',
    'Good morning': 'Доброе утро',
    'Good afternoon': 'Добрый день',
    'Good evening': 'Добрый вечер',
    'Search or address': 'Поиск или адрес',
    'Private window': 'Приватное окно',
    "Mizu won't save your history, cookies, site data or form entries from this window. They are deleted when you close the last private window.":
      'Mizu не сохранит историю, файлы cookie, данные сайтов и введённые в формы данные из этого окна. Всё удаляется, когда закрывается последнее приватное окно.',
    'Private browsing does not hide you from the sites you visit, your employer, school or internet provider. Downloads and bookmarks you create are kept.':
      'Приватный режим не скрывает вас от посещаемых сайтов, работодателя, учебного заведения или провайдера. Скачанные файлы и созданные закладки сохраняются.',
    "Mizu didn't shut down correctly.": 'Mizu был закрыт некорректно.',

    // ── Scripts & extensions ──
    'New userscript': 'Новый скрипт',
    'Script saved': 'Скрипт сохранён',
    'Update userscript': 'Обновить скрипт',
    'Install userscript': 'Установить скрипт',
    'Install userscripts': 'Установить скрипты',
    'No description': 'Без описания',
    '(no pages — the script will never run)': '(нет страниц — скрипт никогда не запустится)',
    'Show source code': 'Показать исходный код',
    'Install selected': 'Установить выбранные',
    'Install': 'Установить',
    'Nothing selected': 'Ничего не выбрано',
    'Userscripts can read and change everything on the pages they run on. Install only scripts you trust.':
      'Скрипты могут читать и менять всё на страницах, где работают. Устанавливайте только скрипты, которым доверяете.',
    'One link per line: GitHub, GitLab, Gist, Codeberg, Greasy Fork, OpenUserJS or any .user.js URL':
      'По одной ссылке в строке: GitHub, GitLab, Gist, Codeberg, Greasy Fork, OpenUserJS или любой адрес .user.js',
    'Get scripts': 'Получить скрипты',
    'Paste at least one link': 'Вставьте хотя бы одну ссылку',
    'Downloading…': 'Загрузка…',
    'From files…': 'Из файлов…',
    'Add scripts': 'Добавить скрипты',
    'Clicking a .user.js link anywhere installs it too. Several links or files can be added at once.':
      'Скрипт также устанавливается по клику на любую ссылку .user.js. Можно добавить сразу несколько ссылок или файлов.',
    'No userscripts yet.': 'Скриптов пока нет.',
    'Open any .user.js link (for example on greasyfork.org or openuserjs.org) and Mizu will offer to install it.':
      'Откройте любую ссылку .user.js (например, на greasyfork.org или openuserjs.org) — Mizu предложит установить скрипт.',
    'View source': 'Исходный код',
    'Enabled — reload pages to apply': 'Включено — обновите страницы',
    'Disabled': 'Выключено',
    'Local script · no automatic updates': 'Локальный скрипт · без автообновления',
    'New script': 'Новый скрипт',
    'Check for updates': 'Проверить обновления',
    'Checking…': 'Проверяю…',
    'All scripts are up to date': 'Все скрипты актуальны',
    'Mizu Scripts understands the Greasemonkey/Tampermonkey format (@match, @include, @exclude, @require, @run-at) and the common GM_* API. Scripts with an update source are checked for new versions once a day (Settings → Advanced). GM_xmlhttpRequest follows normal web CORS rules.':
      'Mizu Scripts понимает формат Greasemonkey/Tampermonkey (@match, @include, @exclude, @require, @run-at) и основные функции GM_*. Скрипты с источником обновлений проверяются раз в день (Настройки → Дополнительно). GM_xmlhttpRequest подчиняется обычным правилам CORS.',
    'Install extension…': 'Установить расширение…',
    'No extensions installed.': 'Расширения не установлены.',
    'Options': 'Параметры',
    'Chrome Web Store or Edge Add-ons link, e.g. https://chromewebstore.google.com/detail/…': 'Ссылка на Chrome Web Store или Edge Add-ons, например https://chromewebstore.google.com/detail/…',
    'Add from store': 'Добавить из магазина',
    'Install from a web store': 'Установка из магазина',
    'Open an extension page on chromewebstore.google.com or microsoftedge.microsoft.com and press "Add to Mizu" in the address bar — or paste its link here.':
      'Откройте страницу расширения на chromewebstore.google.com или microsoftedge.microsoft.com и нажмите «Добавить в Mizu» в адресной строке — или вставьте ссылку сюда.',
    'Install from file…': 'Установить из файла…',
    'Install from the Chrome Web Store or Edge Add-ons, or from a .crx / .zip file or an unpacked folder (manifest.json). Extensions have no toolbar button in Mizu 0.1 — open their options from this page. Mizu Shield already blocks ads without any extension.':
      'Устанавливайте из Chrome Web Store или Edge Add-ons, из файла .crx / .zip или распакованной папки (manifest.json). В Mizu 0.1 у расширений нет кнопки на панели — их параметры открываются на этой странице. Mizu Shield блокирует рекламу и без расширений.',
    'Userscripts': 'Скрипты',
    'Extensions': 'Расширения',
    'Open a normal (non-private) window first.': 'Сначала откройте обычное (не приватное) окно.',
    'Extensions are disabled in Settings → Advanced.': 'Расширения выключены в Настройки → Дополнительно.',

    // ── Settings ──
    'General': 'Основные',
    'Appearance': 'Внешний вид',
    'Search': 'Поиск',
    'Privacy & Shield': 'Конфиденциальность и Shield',
    'Security': 'Безопасность',
    'Tabs': 'Вкладки',
    'Performance': 'Производительность',
    'Media': 'Мультимедиа',
    'Advanced': 'Дополнительно',
    'Saved — restart Mizu to apply this change': 'Сохранено — перезапустите Mizu, чтобы применить',
    'Saved': 'Сохранено',
    'On startup': 'При запуске',
    'When Mizu starts': 'При запуске Mizu',
    'Continue where I left off': 'Продолжить с того же места',
    'Home page': 'Домашняя страница',
    'Used by the Home button and on startup.': 'Открывается кнопкой «Домой» и при запуске.',
    'Offer to restore tabs after a crash': 'Восстанавливать вкладки после сбоя',
    "If Mizu didn't close normally, your tabs come back.": 'Если Mizu закрылся некорректно, вкладки вернутся.',
    'Confirm before closing many tabs': 'Подтверждать закрытие нескольких вкладок',
    'Language': 'Язык',
    'Interface language. "Automatic" follows Windows.': 'Язык интерфейса. «Автоматически» — как в Windows.',
    'Automatic': 'Автоматически',
    'Custom colour': 'Свой цвет',
    'Theme': 'Тема',
    'Mode': 'Режим',
    'Light': 'Светлая',
    'Dark': 'Тёмная',
    'System': 'Системная',
    'Palette': 'Палитра',
    'Accent colour': 'Акцентный цвет',
    'Interface': 'Интерфейс',
    'Style': 'Стиль',
    'Soft uses rounded, floating controls; Crisp is flatter.': '«Мягкий» — скруглённые парящие элементы, «Чёткий» — более плоский.',
    'Soft': 'Мягкий',
    'Crisp': 'Чёткий',
    'Density': 'Плотность',
    'Compact': 'Компактно',
    'Comfortable': 'Удобно',
    'Spacious': 'Просторно',
    'Animations': 'Анимации',
    'Smooth transitions in the interface. Windows "reduce motion" is always respected.': 'Плавные переходы в интерфейсе. Настройка Windows «уменьшить движение» учитывается всегда.',
    'Translucent window (Mica)': 'Полупрозрачное окно (Mica)',
    'Lets the desktop colour show through the tab strip on Windows 11.': 'Цвет рабочего стола просвечивает через панель вкладок в Windows 11.',
    'Show bookmarks bar': 'Показывать панель закладок',
    'Show Home button': 'Показывать кнопку «Домой»',
    'Default page zoom': 'Масштаб страниц по умолчанию',
    'Add search engine': 'Добавить поисковую систему',
    'Keyword': 'Ключевое слово',
    'URL with {searchTerms}': 'Адрес с {searchTerms}',
    'Add': 'Добавить',
    'Enter a name and an http(s) URL containing {searchTerms}': 'Введите название и адрес http(s), содержащий {searchTerms}',
    'Address bar': 'Адресная строка',
    'Search engine': 'Поисковая система',
    "Used when you type something that isn't an address. Type a keyword like @w to search a specific engine.":
      'Используется, если введённое не является адресом. Введите ключевое слово, например @w, чтобы искать в конкретной системе.',
    'Suggest from history and bookmarks': 'Подсказки из истории и закладок',
    'Suggestions are computed on this device; nothing you type is sent anywhere until you press Enter.':
      'Подсказки вычисляются на этом устройстве; введённый текст никуда не отправляется, пока вы не нажмёте Enter.',
    'Custom search engines': 'Свои поисковые системы',
    'Update failed': 'Ошибка обновления',
    'Built in': 'Встроенный',
    'Waiting for download': 'Ожидает загрузки',
    'Not downloaded': 'Не загружен',
    'Remove list': 'Удалить список',
    'Shipped with Mizu': 'Входит в Mizu',
    'Updating…': 'Обновление…',
    'Update lists now': 'Обновить списки',
    'Add list…': 'Добавить список…',
    'My list': 'Мой список',
    'Subscribe to a filter list': 'Подписаться на список фильтров',
    'Any list in uBlock Origin / Adblock Plus syntax, or a hosts file.': 'Любой список в синтаксисе uBlock Origin / Adblock Plus или файл hosts.',
    'HTTPS address': 'Адрес HTTPS',
    'Subscribe': 'Подписаться',
    'Apply rules': 'Применить правила',
    'No exceptions. Turn Shield off for a site from the shield button in the address bar.': 'Исключений нет. Выключить Shield для сайта можно кнопкой со щитом в адресной строке.',
    'Remove exception': 'Удалить исключение',
    "Blocks ads, trackers and known malicious content with filter lists in uBlock Origin syntax, on top of the engine's own tracking prevention.":
      'Блокирует рекламу, трекеры и известный вредоносный контент по спискам фильтров в синтаксисе uBlock Origin — в дополнение к защите от отслеживания в движке.',
    'Block ads and trackers': 'Блокировать рекламу и трекеры',
    'Network filtering, element hiding and scriptlets, like uBlock Origin.': 'Сетевые фильтры, скрытие элементов и скриптлеты — как в uBlock Origin.',
    'Tracking prevention': 'Защита от отслеживания',
    'Engine-level protection against cross-site tracking storage. Strict may break some sign-ins.': 'Защита на уровне движка от межсайтового отслеживания. «Строгая» может нарушить вход на некоторых сайтах.',
    'Send Global Privacy Control': 'Отправлять Global Privacy Control',
    'Tells websites not to sell or share your data (Sec-GPC header). Legally binding in some regions.': 'Сообщает сайтам, что ваши данные нельзя продавать и передавать (заголовок Sec-GPC). В некоторых регионах имеет юридическую силу.',
    'Filter lists': 'Списки фильтров',
    'Lists are downloaded over HTTPS directly from their maintainers and refreshed every few days.': 'Списки загружаются по HTTPS напрямую у авторов и обновляются раз в несколько дней.',
    'My rules': 'Мои правила',
    'Sites where Shield is off': 'Сайты, где Shield выключен',
    'Clear when Mizu closes': 'Очищать при закрытии Mizu',
    'Cookies': 'Файлы cookie',
    'You will be signed out of websites.': 'Вы выйдете из аккаунтов на сайтах.',
    'Cached files': 'Кешированные файлы',
    'Site storage': 'Данные сайтов',
    'Local storage, IndexedDB, service workers.': 'Local storage, IndexedDB, service workers.',
    'Download history': 'История загрузок',
    'Files stay on disk.': 'Файлы остаются на диске.',
    'Forms and passwords': 'Формы и пароли',
    'Mizu does not have its own password manager yet. These let the engine remember entries inside your profile.': 'В Mizu пока нет своего менеджера паролей. Эти настройки позволяют движку запоминать данные в вашем профиле.',
    'Offer to save passwords': 'Предлагать сохранять пароли',
    'Autofill forms': 'Автозаполнение форм',
    'Browsing history': 'История просмотров',
    'Cached images and files': 'Кешированные изображения и файлы',
    'Site storage (local storage, IndexedDB, service workers)': 'Данные сайтов (local storage, IndexedDB, service workers)',
    'Autofill data': 'Данные автозаполнения',
    'Saved passwords': 'Сохранённые пароли',
    'Clear data': 'Очистить данные',
    'Browsing data cleared': 'Данные очищены',
    'Clear browsing data': 'Очистка данных',
    'Loading…': 'Загрузка…',
    'Sites ask before using your camera, microphone, location, notifications or clipboard. Remembered answers appear here.':
      'Сайты спрашивают разрешения на камеру, микрофон, местоположение, уведомления и буфер обмена. Запомненные ответы появятся здесь.',
    'Site': 'Сайт',
    'Permission': 'Разрешение',
    'Setting': 'Значение',
    'Protection': 'Защита',
    'Microsoft Defender SmartScreen': 'Microsoft Defender SmartScreen',
    "Warns about phishing and malware sites and downloads. Addresses you visit are checked with Microsoft's service.":
      'Предупреждает о фишинговых и вредоносных сайтах и загрузках. Адреса проверяются сервисом Microsoft.',
    'Block pop-ups': 'Блокировать всплывающие окна',
    'Pop-ups that open without a click are blocked; you will see a notice.': 'Окна, открывающиеся без клика, блокируются; вы увидите уведомление.',
    'Always on': 'Всегда включено',
    'These protections cannot be turned off.': 'Эти механизмы защиты нельзя отключить.',
    'Process sandbox and site isolation': 'Песочница процессов и изоляция сайтов',
    'Every site runs in a separate, sandboxed process of the Chromium engine.': 'Каждый сайт работает в отдельном изолированном процессе движка Chromium.',
    'Certificate verification': 'Проверка сертификатов',
    'Invalid HTTPS certificates are always rejected; there is no "proceed anyway".': 'Недействительные HTTPS-сертификаты всегда отклоняются; кнопки «всё равно продолжить» нет.',
    'Internal page isolation': 'Изоляция внутренних страниц',
    "Websites cannot open or script Mizu's own pages.": 'Сайты не могут открывать страницы Mizu или управлять ими.',
    'Change…': 'Изменить…',
    'Download folder changed': 'Папка загрузок изменена',
    'Save files to': 'Сохранять файлы в',
    'Ask where to save each file': 'Спрашивать, куда сохранять каждый файл',
    'Show downloads when a download starts': 'Показывать загрузки при начале скачивания',
    'Open new tabs': 'Открывать новые вкладки',
    'Next to current': 'Рядом с текущей',
    'At the end': 'В конце',
    'Switch to tabs opened from links': 'Переключаться на вкладки, открытые по ссылкам',
    'Sleeping tabs': 'Спящие вкладки',
    "Background tabs you haven't used for a while are suspended to save memory and battery. Tabs playing audio never sleep.":
      'Фоновые вкладки, которыми вы давно не пользовались, приостанавливаются для экономии памяти и батареи. Вкладки со звуком не засыпают.',
    'Put inactive tabs to sleep': 'Усыплять неактивные вкладки',
    'After': 'Через',
    'Efficiency mode': 'Режим эффективности',
    'Asks the engine to keep memory low for tabs you are not looking at.': 'Движок снижает потребление памяти вкладками, которые вы давно не открывали.',
    'Hardware acceleration': 'Аппаратное ускорение',
    'Uses the GPU for rendering and video. Requires restart.': 'Использует видеокарту для отрисовки и видео. Нужен перезапуск.',
    'Open media diagnostics': 'Открыть диагностику медиа',
    'Autoplay': 'Автовоспроизведение',
    'Media autoplay': 'Автовоспроизведение медиа',
    'Requires restart.': 'Нужен перезапуск.',
    'Muted autoplay allowed': 'Разрешено без звука',
    'Only after I interact': 'Только после взаимодействия',
    'Codecs': 'Кодеки',
    'See which video and audio formats this system can decode, and whether hardware acceleration is available.': 'Какие видео- и аудиоформаты может декодировать система и доступно ли аппаратное ускорение.',
    'Reset all settings': 'Сбросить все настройки',
    'Reset all settings to their defaults? Your history, bookmarks and passwords are not affected.': 'Сбросить все настройки? История, закладки и пароли не пострадают.',
    'Settings reset': 'Настройки сброшены',
    'Manage scripts & extensions': 'Управление скриптами и расширениями',
    'Developers': 'Для разработчиков',
    'F12 and "Inspect" in the context menu.': 'F12 и «Проверить» в контекстном меню.',
    'Allow Chromium extensions (Manifest V3), e.g. uBlock Origin Lite. Requires restart.': 'Разрешить расширения Chromium (Manifest V3), например uBlock Origin Lite. Нужен перезапуск.',
    'Update userscripts automatically': 'Обновлять скрипты автоматически',
    "Checks each script's update source once a day and installs newer versions.": 'Раз в день проверяет источник каждого скрипта и устанавливает новые версии.',
    'Userscripts and extensions': 'Скрипты и расширения',
    'Updated automatically by Windows, independently of Mizu.': 'Обновляется автоматически через Windows, независимо от Mizu.',
    'Restore default settings': 'Восстановить настройки по умолчанию',
    'Reset': 'Сбросить',
    'Startup': 'Запуск',
    '5 minutes': '5 минут',
    '15 minutes': '15 минут',
    '30 minutes': '30 минут',
    '1 hour': '1 час',
    '2 hours': '2 часа',
    '6 hours': '6 часов',
    'Ctrl+Shift+B': 'Ctrl+Shift+B',

    // ── Shield page ──
    'Most blocked this session': 'Чаще всего блокировалось за сеанс',
    'Nothing blocked yet in this session.': 'В этом сеансе ещё ничего не заблокировано.',
    'Shield settings': 'Настройки Shield',
    'Update filter lists': 'Обновить списки фильтров',
    'Ad & tracker blocking': 'Блокировка рекламы и трекеров',
    'uBlock Origin–compatible filtering: network rules, element hiding, scriptlets.': 'Фильтрация, совместимая с uBlock Origin: сетевые правила, скрытие элементов, скриптлеты.',
    'Engine tracking prevention': 'Защита от отслеживания в движке',
    'Restricts storage access for known trackers.': 'Ограничивает доступ известных трекеров к хранилищу.',
    'Statistics are kept on this device only. Mizu Shield cannot make you anonymous: websites can still see your IP address and recognise you when you sign in.':
      'Статистика хранится только на этом устройстве. Mizu Shield не делает вас анонимным: сайты по-прежнему видят ваш IP-адрес и узнают вас после входа в аккаунт.',
    'Protection': 'Защита',
    'blocked since Mizu started': 'заблокировано с запуска Mizu',
    'blocked in total': 'заблокировано всего',
    'active filter rules': 'активных правил фильтрации',

    // ── Errors ──
    'Request failed': 'Запрос не выполнен',
    'Not running inside Mizu': 'Работает не внутри Mizu',
    'Timed out': 'Время ожидания истекло',
    'unsupported': 'не поддерживается',

    // ── Passwords ──
    'Password manager': 'Менеджер паролей',
    'Open password manager': 'Открыть менеджер паролей',
    // ── Updates & shortcuts ──
    'Check for updates': 'Проверить обновления',
    'Update now': 'Обновить сейчас',
    'Later': 'Позже',
    'Download': 'Скачать',
    'Checks for new versions when Mizu starts and every few hours, then asks before installing.': 'Проверяет новые версии при запуске Mizu и каждые несколько часов, а перед установкой спрашивает.',
    'The new version is downloaded and verified. Mizu restarts and your tabs come back.': 'Новая версия загружена и проверена. Mizu перезапустится, а вкладки вернутся.',
    'This copy is not installed with MizuSetup, so it cannot update itself. Download the new version from the releases page.': 'Эта копия установлена не через MizuSetup и не может обновиться сама. Скачайте новую версию со страницы выпусков.',
    'Shortcuts': 'Сочетания клавиш',
    'Search shortcuts': 'Поиск сочетаний',
    'Shortcuts work everywhere in Mizu. Some page shortcuts (Find, Print, Developer tools) are handled by the web engine.': 'Сочетания работают во всём Mizu. Некоторые сочетания страниц (поиск, печать, инструменты разработчика) обрабатывает веб-движок.',
    'Windows': 'Окна',
    'Navigation': 'Навигация',
    'Page': 'Страница',
    'Close tab': 'Закрыть вкладку',
    'Reopen closed tab': 'Открыть закрытую вкладку',
    'Next tab': 'Следующая вкладка',
    'Previous tab': 'Предыдущая вкладка',
    'Go to tab 1–9': 'Перейти к вкладке 1–9',
    'New window': 'Новое окно',
    'New private window': 'Новое приватное окно',
    'Full screen': 'Полный экран',
    'Back': 'Назад',
    'Forward': 'Вперёд',
    'Reload page': 'Обновить страницу',
    'Reload without cache': 'Обновить без кеша',
    'Address bar': 'Адресная строка',
    'Search from the address bar': 'Поиск из адресной строки',
    'Find on page': 'Найти на странице',
    'Print': 'Печать',
    'Zoom in / out': 'Увеличить / уменьшить',
    'Reset zoom': 'Сбросить масштаб',
    'Picture in picture': 'Картинка в картинке',
    'Developer tools': 'Инструменты разработчика',
    'Bookmark this page': 'Добавить страницу в закладки',
    'Show or hide bookmarks bar': 'Показать или скрыть панель закладок',
    'Clear browsing data': 'Очистить данные браузера',
    'Open bookmarks': 'Открыть закладки',
    'Delete folder': 'Удалить папку',
    'Open in new tab': 'Открыть в новой вкладке',
    'Search suggestions': 'Поисковые подсказки',
    'Unload tab': 'Выгрузить вкладку',
    'Go with the current.': 'Плыви по течению.',
    'Unload other tabs': 'Выгрузить остальные вкладки',
    'Change': 'Изменить',
    'Reset': 'Сбросить',
    'Not set': 'Не задано',
    'Press a key combination… (Esc to cancel)': 'Нажмите сочетание клавиш… (Esc — отмена)',
    'Use Ctrl or Alt with a key, or a function key (F1–F24)': 'Используйте Ctrl или Alt вместе с клавишей либо функциональную клавишу (F1–F24)',
    'Removed from "{0}"': 'Убрано из «{0}»',
    'These are handled by the web engine and cannot be changed.': 'Их обрабатывает веб-движок, изменить их нельзя.',
    'Reset all shortcuts': 'Сбросить все сочетания',
    'Shortcuts reset': 'Сочетания сброшены',
    'Click the pencil next to an action and press the new keys. Changes apply immediately in every window.': 'Нажмите на карандаш рядом с действием и нажмите новые клавиши. Изменения сразу действуют во всех окнах.',
    'Reload page': 'Обновить страницу',
    'Shows popular queries from your search engine while you type. What you type is sent to that engine; never in private windows.': 'Показывает популярные запросы поисковой системы, пока вы печатаете. Набранный текст отправляется этой поисковой системе; в приватных окнах — никогда.',
    'Button position': 'Положение кнопки',
    'Top right': 'Справа вверху',
    'Top left': 'Слева вверху',
    'Compact button': 'Компактная кнопка',
    'Video switching in the floating window': 'Переключение видео в плавающем окне',
    "Adds previous/next video and 10-second skip buttons. On YouTube they use the player's own playlist; elsewhere they switch between the videos on the page.": 'Добавляет кнопки «предыдущее/следующее видео» и перемотку на 10 секунд. На YouTube они используют плейлист плеера, на других сайтах — переключают видео на странице.',
    'Show only the icon, without the "PiP" label.': 'Только значок, без надписи «PiP».',
    'Keyboard shortcut': 'Сочетание клавиш',
    'Pops out the playing video on any page, even with the button turned off. Reload open pages to apply changes.': 'Выносит играющее видео в отдельное окно на любой странице, даже если кнопка выключена. Чтобы применить изменения, перезагрузите открытые страницы.',
    'Hover a video to pop it out into a floating window that stays on top. Works even where a site hides its own button.': 'Наведите курсор на видео, чтобы вынести его в плавающее окно поверх других. Работает даже там, где сайт прячет свою кнопку.',
    'Open in new window': 'Открыть в новом окне',
    'Open in private window': 'Открыть в приватном окне',
    'Copy link': 'Копировать ссылку',
    'Edit in bookmarks…': 'Изменить в закладках…',
    'Where it runs': 'Где работает',
    'Run on': 'Запускать на',
    'Never run on': 'Никогда не запускать на',
    'Sites from the script': 'Сайтах из скрипта',
    'Only sites I choose': 'Только выбранных сайтах',
    'The script does not list any sites.': 'В скрипте не указаны сайты.',
    'Add': 'Добавить',
    'Add at least one site': 'Добавьте хотя бы один сайт',
    'Saved — reload pages to apply': 'Сохранено — перезагрузите страницы, чтобы применить',
    'A site name covers all its pages and subdomains: youtube.com. Limit to a section with a path: github.com/user. Patterns like https://*.example.org/* work too.': 'Имя сайта включает все его страницы и поддомены: youtube.com. Чтобы ограничить разделом, добавьте путь: github.com/user. Шаблоны вида https://*.example.org/* тоже работают.',
    'Your saved logins, encrypted with your Windows account. View, edit, import or export them.': 'Ваши сохранённые логины, зашифрованные учётной записью Windows. Просмотр, изменение, импорт и экспорт.',
    'Passwords': 'Пароли',
    'Saved password for this site': 'Сохранённый пароль для этого сайта',
    'Saved passwords for this site': 'Сохранённые пароли для этого сайта',
    'Save password?': 'Сохранить пароль?',
    'Update password?': 'Обновить пароль?',
    'Username': 'Имя пользователя',
    'Password': 'Пароль',
    'Username or email': 'Имя пользователя или почта',
    'Show password': 'Показать пароль',
    'Hide password': 'Скрыть пароль',
    'Never for this site': 'Никогда для этого сайта',
    'Not now': 'Не сейчас',
    'Update': 'Обновить',
    'Encrypted with your Windows account. Only you can read it on this PC.': 'Шифруется вашей учётной записью Windows. Прочитать его можете только вы на этом ПК.',
    'Password saved': 'Пароль сохранён',
    'Fill': 'Вставить',
    '(no username)': '(без имени)',
    'Manage passwords': 'Управление паролями',
    'Edit password': 'Изменить пароль',
    'Add password': 'Добавить пароль',
    'Site': 'Сайт',
    'Note': 'Заметка',
    'Leave empty to keep the current password': 'Оставьте пустым, чтобы не менять пароль',
    'Copy password': 'Копировать пароль',
    'Copy username': 'Копировать имя пользователя',
    'Password copied': 'Пароль скопирован',
    'Username copied': 'Имя пользователя скопировано',
    'No passwords match.': 'Пароли не найдены.',
    'No saved passwords yet.': 'Сохранённых паролей пока нет.',
    'Sign in on any site and Mizu will offer to save the password — or import them from another browser.': 'Войдите на любом сайте — Mizu предложит сохранить пароль. Или импортируйте пароли из другого браузера.',
    'Never saved': 'Не сохранять',
    'Sites where you chose "Never for this site" appear here.': 'Здесь появятся сайты, для которых выбрано «Никогда для этого сайта».',
    'Import…': 'Импорт…',
    'Export…': 'Экспорт…',
    'Search passwords': 'Поиск паролей',
    'The exported file contains your passwords in plain text. Anyone who opens it can read them. Continue?': 'Экспортируемый файл содержит пароли в открытом виде — их сможет прочитать любой, кто его откроет. Продолжить?',
    'Offering to save passwords is turned off in Settings → Privacy.': 'Предложение сохранять пароли выключено в Настройки → Конфиденциальность.',
    'Passwords are encrypted with your Windows account (DPAPI) and never leave this device. Import accepts CSV exports from Chrome, Edge, Firefox, Opera, Brave, Vivaldi and Bitwarden.': 'Пароли шифруются вашей учётной записью Windows (DPAPI) и никогда не покидают это устройство. Импорт принимает CSV-экспорт из Chrome, Edge, Firefox, Opera, Brave, Vivaldi и Bitwarden.',
    'Mizu Passwords stores logins encrypted with your Windows account (DPAPI) on this device only.': 'Mizu Passwords хранит логины зашифрованными вашей учётной записью Windows (DPAPI) только на этом устройстве.',
    'After you sign in, Mizu asks whether to remember the login. Fill it later with the key in the address bar.': 'После входа Mizu спросит, запомнить ли логин. Вставить его потом можно ключом в адресной строке.',
    'Saved passwords': 'Сохранённые пароли',
    'View, edit, import from other browsers or export.': 'Просмотр, изменение, импорт из других браузеров и экспорт.',
    'Engine form autofill': 'Автозаполнение форм движком',
    'Lets the web engine remember addresses and other form entries.': 'Позволяет движку запоминать адреса и другие данные форм.',
    'This login belongs to a different site': 'Этот логин относится к другому сайту',
    'The password cannot be empty': 'Пароль не может быть пустым',
    'Enter the site address, e.g. https://example.com': 'Введите адрес сайта, например https://example.com',

    // ── Vertical tabs ──
    'Tab layout': 'Расположение вкладок',
    'Vertical tabs live in a sidebar on the left; collapse it to icons with the arrow at its bottom.': 'Вертикальные вкладки находятся в боковой панели слева; её можно свернуть до значков стрелкой внизу.',
    'Top': 'Сверху',
    'Side': 'Сбоку',
    'Collapse sidebar': 'Свернуть панель',
    'Expand sidebar': 'Развернуть панель',
    'Drag to resize': 'Потяните, чтобы изменить ширину',
    'Open windows maximized': 'Открывать окна развёрнутыми',
    'Update Mizu automatically': 'Обновлять Mizu автоматически',
    'Downloads signed updates in the background and installs them when you restart.': 'Загружает подписанные обновления в фоне и устанавливает их при перезапуске.',
    'Update': 'Обновить',
    'Checking for updates…': 'Проверка обновлений…',
    'Mizu is up to date.': 'Установлена последняя версия Mizu.',
    'Updates are signed by Onibi Lab and installed automatically.': 'Обновления подписаны Onibi Lab и устанавливаются автоматически.',
    'Portable copy: updates are announced here; install Mizu with MizuSetup for automatic updates.': 'Портативная копия: о новых версиях сообщается здесь; для автообновления установите Mizu через MizuSetup.',
    'Restart to update': 'Перезапустить и обновить',
    'Check now': 'Проверить',
    'No update is ready': 'Обновление ещё не готово',
    'Mizu starts full-size instead of in a smaller window.': 'Mizu запускается на весь экран, а не в небольшом окне.',

    // ── Picture in picture ──
    'Picture in picture': 'Картинка в картинке',
    'Picture-in-picture button on videos': 'Кнопка «картинка в картинке» на видео',
    'Hover a video to pop it out into a floating window that stays on top. Shortcut: Alt+Shift+P. Works even where a site hides its own button.': 'Наведите на видео, чтобы вынести его в плавающее окно поверх остальных. Сочетание клавиш: Alt+Shift+P. Работает даже там, где сайт прячет свою кнопку.',

    // ── New tab ──
    'Search engine': 'Поисковая система',
    'Add shortcut': 'Добавить ярлык',
    'Edit shortcut': 'Изменить ярлык',
    'Enter an address': 'Введите адрес',

    // ── About ──
    'Passwords': 'Пароли',
    'Logins encrypted with your Windows account, filled from the address bar.': 'Логины, зашифрованные учётной записью Windows, вставляются из адресной строки.',
    'uBlock Origin–compatible ad and tracker blocking built into the browser.': 'Встроенная блокировка рекламы и трекеров, совместимая с uBlock Origin.',
    'Mizu Scripts': 'Mizu Scripts',
    'Tampermonkey-style userscripts with automatic updates.': 'Пользовательские скрипты в стиле Tampermonkey с автообновлением.',
    'Chromium extensions straight from the Chrome Web Store.': 'Расширения Chromium прямо из Chrome Web Store.',
    'Pop any video out into a floating window.': 'Выносите любое видео в плавающее окно.',
    'Private windows': 'Приватные окна',
    'A separate in-memory session that leaves nothing behind.': 'Отдельный сеанс в памяти, после которого ничего не остаётся.',
    'System': 'Система',
    'Copy': 'Копировать',
    'Copied': 'Скопировано',
    'Web engine': 'Веб-движок',
    'Microsoft Edge WebView2 is updated automatically by Windows, including security fixes.': 'Microsoft Edge WebView2 обновляется автоматически через Windows, включая исправления безопасности.',
    'Automatic': 'Автоматически',
    'New versions of Mizu are published on GitHub. Built-in updates are planned for 0.2.': 'Новые версии Mizu публикуются на GitHub. Встроенное обновление запланировано на 0.2.',
    'Releases': 'Релизы',
    'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks, passwords and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.': 'Mizu не отправляет в Onibi Lab никакой аналитики и телеметрии. История, закладки, пароли и настройки хранятся только на этом устройстве. Списки фильтров загружаются напрямую у их авторов.',
    'Source code': 'Исходный код',
    'Report a problem': 'Сообщить о проблеме',
    'Changelog': 'Список изменений',
    'Roadmap': 'План развития',
    'Content blocking: adblock-rust and adblock-resources by Brave Software (MPL-2.0). Filter lists: uBlock Origin, EasyList, Peter Lowe and their contributors. Web engine: Microsoft Edge WebView2.': 'Блокировка контента: adblock-rust и adblock-resources от Brave Software (MPL-2.0). Списки фильтров: uBlock Origin, EasyList, Peter Lowe и их участники. Веб-движок: Microsoft Edge WebView2.',
    'Install from the Chrome Web Store or Edge Add-ons, or from a .crx / .zip file or an unpacked folder (manifest.json). Extensions have no toolbar button in Mizu 0.1 — open their options from this page. Mizu Shield already blocks ads without any extension.': 'Устанавливайте из Chrome Web Store или Edge Add-ons, из файла .crx / .zip или распакованной папки (manifest.json). В Mizu 0.1 у расширений нет кнопки на панели — их параметры открываются на этой странице. Mizu Shield блокирует рекламу и без расширений.',
  };

  // Strings with variable parts.
  const PATTERNS = [
    [/^Mizu Shield: (\d+) blocked on this page$/, 'Mizu Shield: заблокировано на странице — $1'],
    [/^Search (.+) or enter address$/, 'Поиск в $1 или адрес'],
    [/^Search the web or enter address$/, 'Поиск или адрес'],
    [/^Search (.+)$/, 'Поиск: $1'],
    [/^Downloads — (\d+) in progress$/, 'Загрузки — выполняется: $1'],
    [/^(.+) added to Mizu$/, '$1 добавлено в Mizu'],
    [/^(.+) installed$/, 'Установлено: $1'],
    [/^(\d+) scripts installed$/, 'Установлено скриптов: $1'],
    [/^Install (\d+) userscripts$/, 'Установить скрипты ($1)'],
    [/^Close (\d+) tabs\?$/, 'Закрыть вкладки ($1)?'],
    [/^Restore (\d+) tabs?$/, 'Восстановить вкладки ($1)'],
    [/^Version (.+) · (.+)$/, 'Версия $1 · $2'],
    [/^Paused · (.+)$/, 'Приостановлено · $1'],
    [/^(.+) of (.+) · (.+)\/s · (.+) left$/, '$1 из $2 · $3/с · осталось $4'],
    [/^(.+) of (.+) · (.+)\/s$/, '$1 из $2 · $3/с'],
    [/^(\S+ \S+) of (\S+ \S+)$/, '$1 из $2'],
    [/^(\d+) visits? removed$/, 'Удалено посещений: $1'],
    [/^Updated (.+)$/, 'Обновлено $1'],
    [/^Updated: (.+)$/, 'Обновлены: $1'],
    [/^(\d+) updated: (.+)$/, 'Обновлено ($1): $2'],
    [/^No updates \((\d+) could not be checked\)$/, 'Обновлений нет (не удалось проверить: $1)'],
    [/^Remove "(.+)"\?$/, 'Удалить «$1»?'],
    [/^Delete "(.+)" and everything in it\?$/, 'Удалить «$1» со всем содержимым?'],
    [/^(\d+) items$/, 'Элементов: $1'],
    [/^From (.+?)( · by .+?)?( · .+ of @require libraries)?$/, (m, from, by, req) => `Источник: ${from}${by ? by.replace(' · by ', ' · автор: ') : ''}${req ? req.replace(/ · (.+) of @require libraries/, ' · библиотеки @require: $1') : ''}`],
    [/^Uses features Mizu Scripts does not support yet: (.+)\.$/, 'Использует возможности, которых пока нет в Mizu Scripts: $1.'],
    [/^update (.+) → (.+)$/, 'обновление $1 → $2'],
    [/^new$/, 'новый'],
    [/^(.+) · checked (.+)$/, '$1 · проверено $2'],
    [/^(\d+) MB video memory$/, '$1 МБ видеопамяти'],
    [/^System probe failed: (.+)$/, 'Проверка системы не удалась: $1'],
    [/^ wants to (.+)$/, (m, what) => ` хочет ${RU[what] || what}`],
    [/^(\d+) ads and trackers blocked by Mizu Shield$/, 'Mizu Shield заблокировал рекламы и трекеров: $1'],
    [/^([\d\s,.]+) ads and trackers blocked by Mizu Shield$/, 'Mizu Shield заблокировал рекламы и трекеров: $1'],
    [/^Could not install (.+): (.+)$/, 'Не удалось установить $1: $2'],
    [/^Could not download the userscript: (.+)$/, 'Не удалось загрузить скрипт: $1'],
    [/^Not a valid userscript: (.+)$/, 'Это не скрипт: $1'],
    [/^(\d+(?:\.\d)?) (KB|MB|GB|TB|B)$/, (m, n, u) => `${n.replace('.', ',')} ${{ B: 'Б', KB: 'КБ', MB: 'МБ', GB: 'ГБ', TB: 'ТБ' }[u]}`],
    [/^(\d+) tabs?$/, (m, n) => `Вкладок: ${n}`],
    [/^Where "(.+)" runs$/, 'Где работает «$1»'],
    [/^Update to Mizu (.+)\?$/, 'Обновить до Mizu $1?'],
    [/^Mizu (\S+) is available$/, 'Доступен Mizu $1'],
    [/^Only on (.+?)(?: · Never on (.+))?$/, (m, a, b) => `Только: ${a}${b ? ` · Кроме: ${b}` : ''}`],
    [/^Never on (.+)$/, 'Кроме: $1'],
    [/^Restart to update to Mizu (.+)$/, 'Перезапустить и обновить до Mizu $1'],
    [/^Mizu (.+) is ready — restart to update$/, 'Mizu $1 готов — перезапустите для обновления'],
    [/^Mizu (.+) is downloaded and verified\. Restart to install it\.$/, 'Mizu $1 загружен и проверен. Перезапустите, чтобы установить.'],
    [/^Mizu (.+) is available\. (.+)$/, 'Доступен Mizu $1. Эта копия установлена не через MizuSetup — скачайте новую версию со страницы релизов.'],
    [/^Could not check for updates: (.+)$/, 'Не удалось проверить обновления: $1'],
    [/^(\d+) saved passwords for this site$/, 'Сохранённых паролей для сайта: $1'],
    [/^(\d+) passwords imported$/, 'Импортировано паролей: $1'],
    [/^(\d+) passwords exported$/, 'Экспортировано паролей: $1'],
    [/^Delete the saved password for (.+)\?$/, 'Удалить сохранённый пароль для $1?'],
    [/^Version (\S+)$/, 'Версия $1'],
    [/^Close (\d+) tabs\?$/, 'Закрыть вкладки ($1)?'],
    [/^(\d+) s$/, '$1 с'],
    [/^(\d+) min$/, '$1 мин'],
    [/^(\d+) h (\d+) min$/, '$1 ч $2 мин'],
  ];

  let lang = document.documentElement.dataset.lang === 'ru' ? 'ru' : 'en';

  function tr(s) {
    if (lang !== 'ru' || !s) return s;
    const key = s.trim();
    if (!key) return s;
    let out = RU[key];
    if (out === undefined) {
      for (const [re, rep] of PATTERNS) {
        if (re.test(key)) {
          out = key.replace(re, rep);
          break;
        }
      }
    }
    if (out === undefined || out === key) return s;
    return s.replace(key, out);
  }

  const SKIP = '[translate="no"], script, style, pre, textarea, code';
  const ATTRS = ['title', 'placeholder', 'aria-label'];

  function translateText(node) {
    const p = node.parentElement;
    if (!p || p.closest(SKIP)) return;
    const v = node.nodeValue;
    const t = tr(v);
    if (t !== v) node.nodeValue = t;
  }

  function translateAttrs(el) {
    if (el.closest('[translate="no"]')) return;
    for (const a of ATTRS) {
      const v = el.getAttribute(a);
      if (v) {
        const t = tr(v);
        if (t !== v) el.setAttribute(a, t);
      }
    }
  }

  function walk(root) {
    if (lang !== 'ru' || !root) return;
    if (root.nodeType === Node.TEXT_NODE) return translateText(root);
    if (root.nodeType !== Node.ELEMENT_NODE) return;
    if (root.closest(SKIP)) return;
    translateAttrs(root);
    const tw = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
    for (let n = tw.nextNode(); n; n = tw.nextNode()) {
      if (n.nodeType === Node.TEXT_NODE) translateText(n);
      else translateAttrs(n);
    }
  }

  const observer = new MutationObserver((mutations) => {
    if (lang !== 'ru') return;
    for (const m of mutations) {
      if (m.type === 'childList') m.addedNodes.forEach(walk);
      else if (m.type === 'characterData') translateText(m.target);
      else if (m.type === 'attributes') translateAttrs(m.target);
    }
  });

  function start() {
    observer.observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
    walk(document.body);
    if (lang === 'ru') document.title = tr(document.title);
    document.documentElement.lang = lang;
  }
  if (document.body) start();
  else document.addEventListener('DOMContentLoaded', start, { once: true });

  // Dialogs created with confirm()/alert() are translated too.
  const nativeConfirm = window.confirm.bind(window);
  window.confirm = (msg) => nativeConfirm(tr(String(msg)));

  return {
    tr,
    get lang() {
      return lang;
    },
    /// Called with the resolved language when settings change; switching reloads the
    /// surface so every string is rendered fresh in the new language.
    setLang(next) {
      next = next === 'ru' ? 'ru' : 'en';
      if (next === lang) return;
      const was = lang;
      lang = next;
      if (was === 'ru' || document.readyState === 'complete') location.reload();
      else walk(document.body);
    },
  };
})();
