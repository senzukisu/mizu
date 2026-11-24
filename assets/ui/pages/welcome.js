// Welcome to Mizu: shown once on first run and after updating to a new major version.
// A short tour, how Mizu should look, default browser, import. Every step can be skipped
// and everything here is also in Settings.
'use strict';

const main = document.getElementById('main');
let step = 0;
let s = null; // settings working copy
let info = { default: false, registered: false };
let sources = null;

const STEPS = ['Welcome', 'Your style', 'Default browser', 'Your data', 'Ready'];

const BENEFITS = [
  ['shield-check', 'Ads and trackers blocked', 'Mizu Shield works out of the box, with uBlock Origin–style lists.'],
  ['cookie', 'Privacy by default', 'Third-party cookies blocked, no telemetry, everything stays on this PC.'],
  ['music', 'Music and video hub', 'Control what plays in any tab, with a volume per site.'],
  ['tabs', 'Tabs your way', 'On top or in a sidebar, with pinned sites and a find bar.'],
  ['sun', 'Make it yours', 'Nine palettes, accent colours, fonts, backgrounds and widgets.'],
  ['key', 'Passwords and scripts', 'An encrypted password manager and userscripts per site.'],
];

function save() {
  return mizu.request('set_settings', { settings: s }).catch(() => {});
}

function dots() {
  return h('div', { class: 'wl-dots' }, ...STEPS.map((_, i) => h('span', { class: i === step ? 'on' : i < step ? 'done' : '' })));
}

function nav(primaryLabel, onPrimary, opts = {}) {
  const row = h('div', { class: 'wl-nav' });
  if (step > 0) {
    const back = h('button', { class: 'btn ghost', text: 'Back' });
    back.addEventListener('click', () => go(step - 1));
    row.append(back);
  } else {
    row.append(h('span'));
  }
  const right = h('div', { class: 'wl-nav-r' });
  if (opts.skip) {
    const skip = h('button', { class: 'btn ghost', text: opts.skip });
    skip.addEventListener('click', () => go(step + 1));
    right.append(skip);
  }
  const p = h('button', { class: 'btn primary', text: primaryLabel });
  p.addEventListener('click', onPrimary);
  right.append(p);
  row.append(right);
  return row;
}

function choice(options, value, onPick) {
  const el = h('div', { class: 'wl-choice' });
  for (const [v, label, ic] of options) {
    const b = h('button', { class: v === value ? 'on' : '' }, icon(ic, { size: 22 }), h('span', { text: label }));
    b.addEventListener('click', () => {
      for (const x of el.children) x.classList.toggle('on', x === b);
      onPick(v);
    });
    el.append(b);
  }
  return el;
}

function screen() {
  switch (step) {
    case 0:
      return [
        langSwitch(),
        h('div', { class: 'wl-hero' },
          h('div', { class: 'wl-name' }, h('img', { src: '/_/logo.svg', alt: '' }), h('span', { class: 'wl-word', text: 'Mizu' }), h('span', { class: 'wl-kanji', text: '水' })),
          h('h1', { text: 'Welcome to Mizu' }),
          h('div', { class: 'wl-motto', text: 'Go with the current.' }),
          h('p', { text: 'A calm, fast browser that keeps your data to yourself. Here is what it does, then a few quick choices.' })),
        h('div', { class: 'wl-grid' }, ...BENEFITS.map(([ic, t, d]) => h('div', { class: 'wl-card' }, h('div', { class: 'wl-ic' }, icon(ic, { size: 20 })), h('div', { class: 't', text: t }), h('div', { class: 'd', text: d })))),
        nav("Let's go", () => go(1)),
      ];
    case 1: {
      const a = s.appearance;
      return [
        h('h2', { text: 'How should Mizu look?' }),
        h('p', { class: 'wl-sub', text: 'You can change all of this later in Settings → Appearance.' }),
        h('div', { class: 'wl-label', text: 'Theme' }),
        choice([['light', 'Light', 'sun'], ['dark', 'Dark', 'moon'], ['system', 'Like Windows', 'window']], a.theme, (v) => {
          a.theme = v;
          save();
        }),
        h('div', { class: 'wl-label', text: 'Tabs' }),
        choice([['top', 'On top', 'tabs'], ['side', 'In a sidebar', 'sidebar']], a.tab_layout || 'top', (v) => {
          a.tab_layout = v;
          save();
        }),
        h('div', { class: 'wl-label', text: 'Accent colour' }),
        accentRow(a),
        nav('Next', () => go(2)),
      ];
    }
    case 2: {
      const status = info.default ? h('div', { class: 'wl-ok' }, icon('check', { size: 18 }), h('span', { text: 'Mizu is your default browser.' })) : null;
      const make = h('button', { class: 'btn primary wl-big' }, icon('globe', { size: 18 }), h('span', { text: 'Make Mizu the default browser' }));
      make.addEventListener('click', () => {
        mizu.request('make_default_browser').then(() => setTimeout(refreshDefault, 4000));
      });
      return [
        h('h2', { text: 'Use Mizu for every link?' }),
        h('p', { class: 'wl-sub', text: 'Links from other apps (mail, messengers, documents) will open in Mizu. Windows asks you to confirm in its own settings.' }),
        status || h('div', { class: 'wl-center' }, make),
        h('p', { class: 'wl-later' }, icon('info', { size: 14 }), h('span', { text: 'You can do this any time in Settings → General → Default browser.' })),
        nav('Next', () => go(3), { skip: info.default ? null : 'Not now' }),
      ];
    }
    case 3: {
      const list = h('div', { class: 'wl-sources' });
      const fill = () => {
        list.replaceChildren();
        if (sources === null) return list.append(h('div', { class: 'wl-muted', text: 'Looking for browsers…' }));
        const useful = sources.filter((x) => x.counts.bookmarks + x.counts.history > 0);
        if (!useful.length) return list.append(h('div', { class: 'wl-muted', text: 'No other browsers with data were found on this PC.' }));
        for (const src of useful) {
          const btn = h('button', { class: 'btn', text: 'Import' });
          btn.addEventListener('click', async () => {
            btn.disabled = true;
            btn.textContent = i18n.tr('Importing…');
            try {
              const r = await mizu.request('import_run', { id: src.id, bookmarks: true, history: true, engines: true, to_bar: false, bar_name: i18n.tr('Bookmarks bar') });
              btn.replaceWith(h('span', { class: 'wl-done-chip' }, icon('check', { size: 14 }), h('span', { text: `${r.bookmarks + r.history} items` })));
            } catch (e) {
              btn.disabled = false;
              btn.textContent = e.message;
            }
          });
          list.append(h('div', { class: 'wl-src' }, h('div', { class: 'wl-src-m' }, h('div', { class: 't', translate: 'no', text: `${src.browser} · ${src.profile}` }), h('div', { class: 'd', text: `${src.counts.bookmarks} bookmarks, ${src.counts.history} history` })), btn));
        }
      };
      fill();
      if (sources === null) {
        mizu.request('import_sources').then((l) => {
          sources = l || [];
          fill();
        });
      }
      return [
        h('h2', { text: 'Bring your bookmarks and history?' }),
        h('p', { class: 'wl-sub', text: 'From another browser on this PC. It stays as it is and can stay open.' }),
        list,
        h('p', { class: 'wl-later' }, icon('info', { size: 14 }), h('span', { text: 'You can import later whenever you like: Settings → General → Import, or the import button in Bookmarks.' })),
        nav('Next', () => go(4), { skip: 'Skip' }),
      ];
    }
    default:
      return [
        h('div', { class: 'wl-hero' }, h('div', { class: 'wl-check' }, icon('check', { size: 34 })), h('h1', { text: 'All set' }), h('p', { text: 'Enjoy Mizu. Press Ctrl+T for a new tab, Ctrl+F to find on a page, and look for the music button while something plays.' })),
        h('label', { class: 'wl-opt' }, switchInput(s.general.whats_new !== false, (v) => {
          s.general.whats_new = v;
          save();
        }, { label: 'Show what’s new after each update' }), h('span', null, h('b', { text: 'Show what’s new after each update' }), h('small', { text: 'A short page once per update. You can change this in Settings → General.' }))),
        nav('Start browsing', finish),
      ];
  }
}

// Interface language right on the first screen: like Windows, English or Russian.
function langSwitch() {
  const g = s.general;
  const row = h('div', { class: 'wl-lang' }, icon('globe', { size: 14 }));
  for (const [v, label] of [['auto', 'Like Windows'], ['en', 'English'], ['ru', 'Русский']]) {
    const b = h('button', { class: (g.language || 'auto') === v ? 'on' : '', text: label, translate: v === 'auto' ? null : 'no' });
    b.addEventListener('click', () => {
      if ((g.language || 'auto') === v) return;
      g.language = v;
      // The page reloads itself in the new language once the browser confirms.
      save();
    });
    row.append(b);
  }
  return row;
}

function accentRow(a) {
  const colors = ['#2f7de1', '#14919b', '#2e9d6a', '#d9822b', '#e5484d', '#db2777', '#7a5af5', '#4b5563'];
  const row = h('div', { class: 'wl-accents' });
  for (const c of colors) {
    const b = h('button', { class: a.accent.toLowerCase() === c ? 'on' : '', style: { background: c }, title: c }, icon('check', { size: 14 }));
    b.addEventListener('click', () => {
      a.accent = c;
      for (const x of row.children) x.classList.toggle('on', x === b);
      document.documentElement.style.setProperty('--accent', c);
      save();
    });
    row.append(b);
  }
  return row;
}

function refreshDefault() {
  return mizu.request('default_browser_status').then((r) => {
    info = r || info;
    if (step === 2) render();
  });
}

// The tour has its own window: finishing closes it (the browser window is already open).
function finish() {
  mizu.request('welcome_done');
}

function go(n) {
  step = Math.max(0, Math.min(STEPS.length - 1, n));
  if (step === 2) refreshDefault();
  render();
}

// The living background, created once and kept across steps.
let bg = null;
function render() {
  if (!bg) {
    bg = currentsCanvas({ fps: 60 });
    bg.classList.add('wl-bg');
    document.body.prepend(bg, h('div', { class: 'wl-veil', 'aria-hidden': 'true' }));
  }
  main.replaceChildren(h('div', { class: 'wl' }, h('div', { class: 'wl-panel', key: step }, dots(), ...screen().filter(Boolean))));
}

bootPage(() => {
  mizu.request('get_settings').then((p) => {
    s = structuredClone(p.settings);
    // mizu://welcome/#3 opens a given step (links from Settings, checks).
    const n = Number(location.hash.slice(1));
    if (n > 0) step = Math.min(STEPS.length - 1, n);
    refreshDefault();
    render();
  });
});
