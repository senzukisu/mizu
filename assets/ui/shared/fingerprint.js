// Mizu fingerprinting defence (standard). Gives high-entropy APIs protective values: a little
// noise in canvas, WebGL and audio readback, rounded hardware numbers, no battery or local
// fonts API, fewer devices and voices before permission, and reduced user-agent hints.
// Noise depends on a key per site and session: one site always sees the same values during a
// session, two sites see different ones, and nothing is stable across restarts.
// Mizu injects this into web pages (`mizuFingerprintProtect(window, key)`); the check page
// mizu://fingerprint applies it to blank frames to show the difference.
'use strict';

function mizuFingerprintProtect(win, seed) {
  if (!win || win.__mizuFp) return;
  try {
    Object.defineProperty(win, '__mizuFp', { value: true });
  } catch {
    return;
  }
  const key = (Number(seed) >>> 0) || 1;
  // A small, fast hash: the same inputs always give the same noise.
  const mix = (a, b) => {
    let h = (a ^ Math.imul(b | 0, 0x9e3779b1)) >>> 0;
    h = Math.imul(h ^ (h >>> 16), 0x85ebca6b) >>> 0;
    h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35) >>> 0;
    return (h ^ (h >>> 16)) >>> 0;
  };

  // Patched functions still look built in.
  const natives = new WeakMap();
  const FnProto = win.Function.prototype;
  const realToString = FnProto.toString;
  const toString = function toString() {
    return natives.has(this) ? natives.get(this) : realToString.call(this);
  };
  natives.set(toString, realToString.call(realToString));
  try {
    Object.defineProperty(FnProto, 'toString', { value: toString, writable: true, configurable: true });
  } catch {}
  const wrap = (obj, name, make) => {
    if (!obj) return;
    const orig = obj[name];
    if (typeof orig !== 'function') return;
    const fn = make(orig);
    natives.set(fn, realToString.call(orig));
    try {
      Object.defineProperty(fn, 'name', { value: name });
      Object.defineProperty(obj, name, { value: fn, writable: true, configurable: true, enumerable: false });
    } catch {}
  };
  const getter = (obj, name, value) => {
    if (!obj || !(name in obj)) return;
    const desc = Object.getOwnPropertyDescriptor(obj, name);
    const get = function () {
      return typeof value === 'function' ? value.call(this, desc && desc.get) : value;
    };
    if (desc && desc.get) natives.set(get, realToString.call(desc.get));
    try {
      Object.defineProperty(obj, name, { get, configurable: true, enumerable: desc ? desc.enumerable : true });
    } catch {}
  };

  // ── Canvas: about 1 in 120 visible pixels changes by one step in one colour ──
  const noisePixels = (data, width, x0, y0) => {
    for (let i = 0; i < data.length; i += 4) {
      if (data[i + 3] === 0) continue;
      const p = i >> 2;
      const x = x0 + (p % width);
      const y = y0 + ((p / width) | 0);
      const r = mix(key, x * 73856093 ^ y * 19349663);
      if (r % 120 !== 0) continue;
      const c = i + (r >>> 8) % 3;
      data[c] = data[c] === 255 ? 254 : data[c] + 1;
    }
  };
  const C2D = win.CanvasRenderingContext2D && win.CanvasRenderingContext2D.prototype;
  const realGetImageData = C2D && C2D.getImageData;
  wrap(C2D, 'getImageData', (orig) => function (sx, sy, sw, sh, ...rest) {
    const img = orig.call(this, sx, sy, sw, sh, ...rest);
    try {
      noisePixels(img.data, img.width, Math.floor(sx) || 0, Math.floor(sy) || 0);
    } catch {}
    return img;
  });
  // toDataURL / toBlob read a copy with the same noise.
  const noisyCopy = (canvas) => {
    const w = canvas.width;
    const hgt = canvas.height;
    if (!w || !hgt || w * hgt > 16_000_000 || !realGetImageData) return null;
    const copy = win.document.createElement('canvas');
    copy.width = w;
    copy.height = hgt;
    const ctx = copy.getContext('2d');
    if (!ctx) return null;
    ctx.drawImage(canvas, 0, 0);
    const img = realGetImageData.call(ctx, 0, 0, w, hgt);
    noisePixels(img.data, w, 0, 0);
    ctx.putImageData(img, 0, 0);
    return copy;
  };
  const Canvas = win.HTMLCanvasElement && win.HTMLCanvasElement.prototype;
  wrap(Canvas, 'toDataURL', (orig) => function (...args) {
    let copy = null;
    try {
      copy = noisyCopy(this);
    } catch {}
    return orig.apply(copy || this, args);
  });
  wrap(Canvas, 'toBlob', (orig) => function (...args) {
    let copy = null;
    try {
      copy = noisyCopy(this);
    } catch {}
    return orig.apply(copy || this, args);
  });

  // ── WebGL: the GPU's family only, and noisy readPixels ──
  const generic = (s) => {
    const m = /ANGLE \(([^,]+),/.exec(String(s));
    return m ? `ANGLE (${m[1].trim()}, ${m[1].trim()} GPU, Direct3D11)` : String(s).split(/[ (]/)[0] || 'GPU';
  };
  for (const name of ['WebGLRenderingContext', 'WebGL2RenderingContext']) {
    const P = win[name] && win[name].prototype;
    wrap(P, 'getParameter', (orig) => function (p) {
      const v = orig.call(this, p);
      if (p === 0x9246) return generic(v);
      return v;
    });
    wrap(P, 'readPixels', (orig) => function (...args) {
      const r = orig.apply(this, args);
      try {
        const out = args[6];
        if (out && out.length) {
          for (let i = 0; i < out.length; i += 4) {
            if (mix(key, i) % 151 === 0) out[i] = out[i] === 255 ? 254 : out[i] + 1;
          }
        }
      } catch {}
      return r;
    });
  }

  // ── Audio: inaudible noise in rendered samples and analyser output ──
  const touched = new WeakSet();
  wrap(win.AudioBuffer && win.AudioBuffer.prototype, 'getChannelData', (orig) => function (...args) {
    const data = orig.apply(this, args);
    try {
      if (!touched.has(data)) {
        touched.add(data);
        for (let i = 0; i < data.length; i++) {
          const r = mix(key, i);
          if (r % 97 === 0) data[i] += ((r >>> 8) / 0xffffff - 0.5) * 2e-7;
        }
      }
    } catch {}
    return data;
  });
  wrap(win.AnalyserNode && win.AnalyserNode.prototype, 'getFloatFrequencyData', (orig) => function (arr) {
    orig.call(this, arr);
    try {
      for (let i = 0; i < arr.length; i++) arr[i] += ((mix(key, i) >>> 8) / 0xffffff - 0.5) * 1e-4;
    } catch {}
  });

  // ── Hardware: common values only ──
  const Nav = win.Navigator && win.Navigator.prototype;
  getter(Nav, 'hardwareConcurrency', (real) => {
    const n = real ? real.call(win.navigator) : 4;
    return n >= 8 ? 8 : 4;
  });
  getter(Nav, 'deviceMemory', (real) => {
    const n = real ? real.call(win.navigator) : 8;
    return n >= 8 ? 8 : 4;
  });

  // ── APIs ordinary sites do not need ──
  try {
    delete Nav.getBattery;
  } catch {}
  try {
    if ('queryLocalFonts' in win) delete win.queryLocalFonts;
    if (win.Window && 'queryLocalFonts' in win.Window.prototype) delete win.Window.prototype.queryLocalFonts;
  } catch {}

  // ── Cameras and microphones: one of each kind until you allow access ──
  const MD = win.MediaDevices && win.MediaDevices.prototype;
  wrap(MD, 'enumerateDevices', (orig) => function (...args) {
    return orig.apply(this, args).then((list) => {
      if (list.some((d) => d.label)) return list;
      const seen = new Set();
      return list.filter((d) => (seen.has(d.kind) ? false : seen.add(d.kind)));
    });
  });

  // ── Voices: the ones for your language and English ──
  wrap(win.SpeechSynthesis && win.SpeechSynthesis.prototype, 'getVoices', (orig) => function (...args) {
    const all = orig.apply(this, args);
    const lang = String(win.navigator.language || 'en').slice(0, 2).toLowerCase();
    return all.filter((v) => v.default || /^en/i.test(v.lang) || v.lang.toLowerCase().startsWith(lang)).slice(0, 4);
  });

  // ── User-agent hints: no exact Windows build, browser build or device model ──
  wrap(win.NavigatorUAData && win.NavigatorUAData.prototype, 'getHighEntropyValues', (orig) => function (hints) {
    return orig.call(this, hints).then((v) => {
      const major = (s) => `${String(s || '').split('.')[0]}.0.0.0`;
      const out = { ...v };
      if ('platformVersion' in out) out.platformVersion = '10.0.0';
      if ('uaFullVersion' in out) out.uaFullVersion = major(out.uaFullVersion);
      if (Array.isArray(out.fullVersionList)) out.fullVersionList = out.fullVersionList.map((b) => ({ brand: b.brand, version: major(b.version) }));
      if ('model' in out) out.model = '';
      if ('wow64' in out) out.wow64 = false;
      return out;
    });
  });
}
