// About Mizu.
'use strict';

const main = document.getElementById('main');
const REPO = 'https://github.com/senzukisu/mizu';

const FEATURES = [
  ['shield-check', 'Mizu Shield', 'uBlock Origin–compatible ad and tracker blocking built into the browser.'],
  ['key', 'Passwords', 'Logins encrypted with your Windows account, filled from the address bar.'],
  ['code', 'Mizu Scripts', 'Tampermonkey-style userscripts with automatic updates.'],
  ['puzzle', 'Extensions', 'Chromium extensions straight from the Chrome Web Store.'],
  ['media', 'Picture in picture', 'Pop any video out into a floating window.'],
  ['private', 'Private windows', 'A separate in-memory session that leaves nothing behind.'],
];

function updateText(u, installed) {
  switch (u && u.state) {
    case 'checking':
      return 'Checking for updates…';
    case 'up_to_date':
      return 'Mizu is up to date.';
    case 'ready':
      return `Mizu ${u.version} is downloaded and verified. Restart to install it.`;
    case 'available':
      return `Mizu ${u.version} is available. This copy is not installed with MizuSetup, so download it from the releases page.`;
    case 'failed':
      return `Could not check for updates: ${u.error}`;
    default:
      return installed ? 'Updates are signed by Onibi Lab and installed automatically.' : 'Portable copy: updates are announced here; install Mizu with MizuSetup for automatic updates.';
  }
}

function updateRow(a) {
  const text = h('div', { class: 'd', text: updateText(a.update, a.installed) });
  const btn = h('button', { class: 'btn' });
  const set = (u) => {
    text.textContent = i18n.tr(updateText(u, a.installed));
    btn.disabled = u && u.state === 'checking';
    if (u && u.state === 'ready') {
      btn.className = 'btn primary';
      btn.textContent = i18n.tr('Restart to update');
      btn.onclick = () => mizu.request('update_install').catch((e) => showToast(e.message));
    } else if (u && u.state === 'available') {
      btn.className = 'btn primary';
      btn.textContent = i18n.tr('Releases');
      btn.onclick = () => openUrl(`${REPO}/releases/latest`);
    } else {
      btn.className = 'btn';
      btn.textContent = i18n.tr('Check now');
      btn.onclick = () => {
        set({ state: 'checking' });
        mizu.request('update_check').then(set).catch((e) => set({ state: 'failed', error: e.message }));
      };
    }
  };
  set(a.update);
  return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Mizu' }), text), btn);
}

// ── Hero: a slow current of lines bending around a mirrored drop ──
function currentsCanvas() {
  const canvas = h('canvas', { class: 'currents', 'aria-hidden': 'true' });
  const ctx = canvas.getContext('2d');
  const still = () => document.documentElement.dataset.motion === 'off' || matchMedia('(prefers-reduced-motion: reduce)').matches;
  let w = 0;
  let hgt = 0;
  let t = 0;
  let last = 0;
  let raf = 0;

  const resize = () => {
    const r = canvas.getBoundingClientRect();
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    w = r.width;
    hgt = r.height;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(hgt * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  };

  // Streamlines of a flow around a round body: lines wrap smoothly around the drop and never
  // cross. In flow coordinates (u along the current, v across) the stream function is
  // psi = v * (1 - R^2 / (u^2 + v^2)); each line is psi = c, solved for v by bisection.
  function solveV(u, c, R) {
    const target = Math.abs(c);
    const r2 = R * R;
    let lo = Math.abs(u) < R ? Math.sqrt(r2 - u * u) : 0;
    let hi = target + R + 2;
    for (let k = 0; k < 22; k++) {
      const m = (lo + hi) / 2;
      const f = m * (1 - r2 / (u * u + m * m)) - target;
      if (f < 0) lo = m;
      else hi = m;
    }
    return Math.sign(c) * (lo + hi) / 2;
  }

  function draw() {
    if (!w || !hgt) return;
    const R = Math.min(w, hgt) * 0.15;
    const cx = w * 0.7;
    const cy = hgt * 0.5 + Math.sin(t * 0.6) * 2;
    const a = 0.36; // the current runs diagonally, top-left to bottom-right
    const ca = Math.cos(a);
    const sa = Math.sin(a);
    const L = Math.hypot(w, hgt);

    const bg = ctx.createLinearGradient(0, 0, w, hgt);
    bg.addColorStop(0, '#12061f');
    bg.addColorStop(1, '#1d0a33');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, hgt);

    const ink = ctx.createLinearGradient(0, 0, w, hgt);
    ink.addColorStop(0, '#6d3fb8');
    ink.addColorStop(0.55, '#a77bff');
    ink.addColorStop(1, '#d6bcff');
    ctx.strokeStyle = ink;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    const step = 6;
    ctx.lineWidth = step * 0.48;
    const span = Math.ceil(L / step);
    for (let n = -span; n <= span; n++) {
      const c = (n + 0.5) * step;
      ctx.beginPath();
      let first = true;
      for (let u = -L; u <= L; u += 4) {
        let v = solveV(u, c, R);
        // Downstream the current breaks into slow ripples.
        const behind = Math.max(0, Math.min(1, (u - R * 0.5) / (R * 4)));
        const near = Math.exp(-Math.abs(c) / (R * 3.5));
        v += Math.sin(u * 0.045 - t * 1.6 + c * 0.07) * (0.5 + behind * near * 9);
        const x = cx + u * ca - v * sa;
        const y = cy + u * sa + v * ca;
        if (x < -20 || x > w + 20 || y < -20 || y > hgt + 20) {
          first = true;
          continue;
        }
        if (first) {
          ctx.moveTo(x, y);
          first = false;
        } else ctx.lineTo(x, y);
      }
      ctx.stroke();
    }

    // The hot line rides the current straight into the drop...
    const ux = ca;
    const uy = sa;
    const start = { x: cx - ux * L, y: cy - uy * L };
    const red = ctx.createLinearGradient(start.x, start.y, cx, cy);
    red.addColorStop(0, 'rgba(255,45,85,0)');
    red.addColorStop(0.35, '#ff2d55');
    red.addColorStop(1, '#ff4d3a');
    ctx.strokeStyle = red;
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(start.x, start.y);
    ctx.lineTo(cx - ux * R * 0.9, cy - uy * R * 0.9);
    ctx.stroke();
    // ...and leaves as a warm ripple in its wake.
    const warm = ctx.createLinearGradient(cx, cy, cx + ux * L * 0.5, cy + uy * L * 0.5);
    warm.addColorStop(0, '#ff6a2b');
    warm.addColorStop(1, '#ffc23a');
    ctx.strokeStyle = warm;
    ctx.lineWidth = 3.2;
    ctx.beginPath();
    for (let u = R * 0.6; u <= L; u += 3) {
      const grow = Math.min(1, (u - R * 0.6) / (R * 3));
      const v = Math.sin(u * 0.05 - t * 1.8) * (2 + grow * 10) + grow * R * 0.25;
      const x = cx + u * ca - v * sa;
      const y = cy + u * sa + v * ca;
      if (u === R * 0.6) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Contact shadow: the drop presses into the current.
    const sh = ctx.createRadialGradient(cx + R * 0.15, cy + R * 0.25, R * 0.6, cx + R * 0.15, cy + R * 0.25, R * 1.9);
    sh.addColorStop(0, 'rgba(8,2,16,0.75)');
    sh.addColorStop(1, 'rgba(8,2,16,0)');
    ctx.fillStyle = sh;
    ctx.beginPath();
    ctx.arc(cx + R * 0.15, cy + R * 0.25, R * 1.9, 0, Math.PI * 2);
    ctx.fill();

    // The drop: polished chrome. The top mirrors a pale sky, the bottom mirrors the current
    // itself as curved stripes, with a dark rim, a warm glint from the line and a highlight.
    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2);
    ctx.clip();
    const sky = ctx.createLinearGradient(cx, cy - R, cx, cy + R * 0.1);
    sky.addColorStop(0, '#ffffff');
    sky.addColorStop(0.55, '#c9cbd6');
    sky.addColorStop(1, '#5a5568');
    ctx.fillStyle = sky;
    ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    const floor = ctx.createLinearGradient(cx, cy, cx, cy + R);
    floor.addColorStop(0, '#1a0c2e');
    floor.addColorStop(1, '#3a2066');
    ctx.fillStyle = floor;
    ctx.beginPath();
    ctx.ellipse(cx, cy + R * 0.08, R * 1.05, R * 0.2, 0, Math.PI, 0, true);
    ctx.lineTo(cx + R, cy + R);
    ctx.lineTo(cx - R, cy + R);
    ctx.closePath();
    ctx.fill();
    ctx.lineWidth = Math.max(1, R * 0.045);
    for (let k = 1; k <= 9; k++) {
      const f = k / 9;
      ctx.strokeStyle = k % 2 ? 'rgba(199,166,255,0.85)' : 'rgba(120,80,200,0.6)';
      ctx.beginPath();
      ctx.ellipse(cx + Math.sin(t * 0.8 + k) * R * 0.03, cy + R * (0.1 + f * f * 0.9), R * (1.05 - f * 0.1), R * (0.08 + f * 0.25), 0, 0, Math.PI);
      ctx.stroke();
    }
    const rim = ctx.createRadialGradient(cx - R * 0.25, cy - R * 0.3, R * 0.3, cx, cy, R);
    rim.addColorStop(0, 'rgba(0,0,0,0)');
    rim.addColorStop(0.78, 'rgba(0,0,0,0.08)');
    rim.addColorStop(1, 'rgba(10,4,20,0.75)');
    ctx.fillStyle = rim;
    ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    ctx.strokeStyle = 'rgba(255,110,60,0.7)';
    ctx.lineWidth = Math.max(1.5, R * 0.05);
    ctx.beginPath();
    ctx.arc(cx, cy, R * 0.95, Math.PI * 1.02, Math.PI * 1.3);
    ctx.stroke();
    ctx.restore();
    const glint = ctx.createRadialGradient(cx - R * 0.38, cy - R * 0.48, 0, cx - R * 0.38, cy - R * 0.48, R * 0.3);
    glint.addColorStop(0, 'rgba(255,255,255,0.95)');
    glint.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = glint;
    ctx.beginPath();
    ctx.arc(cx - R * 0.38, cy - R * 0.48, R * 0.3, 0, Math.PI * 2);
    ctx.fill();
  }

  const frame = (now) => {
    raf = 0;
    if (last) t += Math.min(0.05, (now - last) / 1000);
    last = now;
    draw();
    if (!still() && !document.hidden && canvas.isConnected) raf = requestAnimationFrame(frame);
  };
  const start = () => {
    if (!raf && !still()) {
      last = 0;
      raf = requestAnimationFrame(frame);
    }
  };
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      cancelAnimationFrame(raf);
      raf = 0;
    } else start();
  });
  new ResizeObserver(resize).observe(canvas);
  requestAnimationFrame(() => {
    resize();
    start();
  });
  return canvas;
}

function openUrl(url) {
  mizu.send('open_url', { url, disposition: 'new_tab' });
}

bootPage(() => {
  mizu.request('about').then((a) => {
    const rows = [
      ['Mizu', a.version],
      ['Engine', `${a.engine} ${a.engine_version || '(not found)'}`],
      ['Operating system', a.os],
      ['Profile folder', a.profile],
    ];
    const copy = h('button', { class: 'btn ghost' }, icon('copy'), h('span', { text: 'Copy' }));
    copy.addEventListener('click', () => {
      navigator.clipboard
        .writeText(rows.map(([k, v]) => `${k}: ${v}`).join('\n'))
        .then(() => showToast('Copied'));
    });

    const link = (iconName, label, url) => {
      const b = h('button', { class: 'about-link' }, icon(iconName), h('span', { text: label }));
      b.addEventListener('click', () => openUrl(url));
      return b;
    };

    main.replaceChildren(
      h('div', { class: 'page about' },
        h('section', { class: 'about-hero currents-hero', title: 'Let it happen' },
          currentsCanvas(),
          h('div', { class: 'hero-text' },
          h('div', { class: 'about-name' }, h('img', { src: '/_/logo.svg', alt: '' }), h('h1', { text: 'Mizu' }), h('span', { class: 'kanji', text: '水' })),
          h('div', { class: 'tag', text: `${a.tagline}.` }),
          h('div', { class: 'hero-motto', text: 'Go with the current.' }),
          h('div', { class: 'about-chips' },
            h('span', { class: 'pill accent', text: `Version ${a.version}` }),
            h('span', { class: 'pill', translate: 'no', text: `Chromium ${a.engine_version || '?'}` }),
            h('span', { class: 'pill', text: a.vendor })))),

        h('div', { class: 'about-grid' },
          ...FEATURES.map(([ic, title, text]) => h('div', { class: 'about-feature' }, h('div', { class: 'af-icon' }, icon(ic, { size: 20 })), h('div', { class: 'af-title', text: title }), h('div', { class: 'af-text', text }))),
        ),

        h('section', { class: 'card' },
          h('div', { class: 'card-head' }, h('h2', { text: 'System' }), copy),
          ...rows.map(([k, v]) => h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: k })), h('span', { class: k === 'Profile folder' ? 'mono' : '', translate: 'no', text: v, style: { userSelect: 'text' } })))),

        h('section', { class: 'card' },
          h('h2', { text: 'Updates' }),
          h('div', { class: 'setting' },
            h('div', { class: 'txt' },
              h('div', { class: 't', text: 'Web engine' }),
              h('div', { class: 'd', text: 'Microsoft Edge WebView2 is updated automatically by Windows, including security fixes.' })),
            h('span', { class: 'pill ok', text: 'Automatic' })),
          updateRow(a)),

        h('section', { class: 'card' },
          h('h2', { text: 'Privacy' }),
          h('p', { class: 'note', text: 'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks, passwords and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.' })),

        h('div', { class: 'about-links' },
          link('code', 'Source code', REPO),
          link('warning', 'Report a problem', `${REPO}/issues`),
          link('file', 'Changelog', `${REPO}/blob/main/CHANGELOG.md`),
          link('sparkle', 'Roadmap', `${REPO}/blob/main/ROADMAP.md`)),

        h('p', { class: 'about-credits', text: 'Content blocking: adblock-rust and adblock-resources by Brave Software (MPL-2.0). Filter lists: uBlock Origin, EasyList, Peter Lowe and their contributors. Web engine: Microsoft Edge WebView2.' }),
        h('p', { class: 'about-legal', text: `© ${new Date().getFullYear()} Onibi Lab · Mozilla Public License 2.0` }),
      ),
    );
  });
});
