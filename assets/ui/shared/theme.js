// Applies appearance settings to the current document and keeps them in sync.
'use strict';

const theme = (() => {
  let current = null;
  const listeners = [];

  function apply(settings, opts = {}) {
    if (!settings) return;
    current = settings;
    const a = settings.appearance || {};
    const root = document.documentElement;
    const dark = a.theme === 'dark' || (a.theme === 'system' && !!settings._dark) || (a.theme === undefined && !!settings._dark);
    root.dataset.theme = dark ? 'dark' : 'light';
    root.dataset.palette = a.palette || 'mizu';
    root.dataset.density = a.density || 'comfortable';
    root.dataset.style = a.style || 'soft';
    root.dataset.motion = a.animations === false ? 'off' : 'on';
    root.toggleAttribute('data-tint', !!a.accent_tint);
    applyFont(root, a.ui_font || '');
    if (/^#[0-9a-f]{6}$/i.test(a.accent || '')) {
      root.style.setProperty('--accent', a.accent);
      root.style.setProperty('--accent-ink', readableInk(a.accent));
    }
    if (opts.private) root.dataset.private = '';
    if (opts.translucent !== undefined) {
      if (opts.translucent && a.translucency && !opts.private) root.dataset.translucent = '';
      else delete root.dataset.translucent;
    }
    if (typeof i18n !== 'undefined' && settings._lang) i18n.setLang(settings._lang);
    for (const fn of listeners) fn(settings);
  }

  // Interface font: Mizu's standard, the Windows UI font, or an installed family.
  function applyFont(root, f) {
    if (!f) {
      root.style.removeProperty('--font');
      root.style.removeProperty('--font-display');
      return;
    }
    const name = f === 'system' ? 'system-ui' : `'${f.replace(/[^\p{L}\p{N} \-_.&]/gu, '')}'`;
    const stack = `${name}, "Segoe UI", sans-serif`;
    root.style.setProperty('--font', stack);
    root.style.setProperty('--font-display', stack);
  }

  function readableInk(hex) {
    const n = parseInt(hex.slice(1), 16);
    const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => {
      c /= 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    const l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    return l > 0.45 ? '#111111' : '#ffffff';
  }

  return {
    apply,
    get settings() {
      return current;
    },
    onChange(fn) {
      listeners.push(fn);
    },
  };
})();

// Internal pages: fetch settings once and follow later changes.
function bootPage(onReady) {
  mizu.on('settings', (m) => theme.apply(m.settings));
  mizu
    .request('ready')
    .then((info) => {
      theme.apply(info.settings, { private: info.private });
      onReady && onReady(info);
    })
    .catch((e) => {
      console.error(e);
      onReady && onReady({});
    });
}
