// "Prism": a slow psychedelic sun — petal-shaped bands of warm-to-violet colour breathing
// around a glowing core, with a little film grain (What's new). Like the currents, it draws in a
// worker on an OffscreenCanvas so the page stays instant, pauses when hidden or off screen, and
// stays still with reduced motion. This file is both the page script and the worker.
'use strict';

// ── The renderer: pure canvas drawing, usable on the page or in a worker ──
function prismRenderer(ctx) {
  let w = 0;
  let hgt = 0;
  let dpr = 1;
  let px = 0.5;
  let py = 0.5;
  let grain = null;
  const RINGS = 30;
  const SEG = 180;
  // Sunset orange → coral → magenta → violet → indigo → teal, and around again.
  const PAL = [[255, 168, 76], [255, 104, 106], [232, 70, 160], [158, 78, 246], [84, 92, 240], [38, 178, 214]];

  function mix(f, dark) {
    const x = (((f % 1) + 1) % 1) * PAL.length;
    const i = Math.floor(x);
    const k = x - i;
    const a = PAL[i];
    const b = PAL[(i + 1) % PAL.length];
    const c = (n) => Math.round((a[n] + (b[n] - a[n]) * k) * (1 - dark));
    return `rgb(${c(0)},${c(1)},${c(2)})`;
  }

  function makeGrain() {
    const size = 160;
    const g = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(size, size) : Object.assign(document.createElement('canvas'), { width: size, height: size });
    const gc = g.getContext('2d');
    const img = gc.createImageData(size, size);
    for (let i = 0; i < img.data.length; i += 4) {
      const v = Math.random() * 255;
      img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
      img.data[i + 3] = 255;
    }
    gc.putImageData(img, 0, 0);
    return ctx.createPattern(g, 'repeat');
  }

  return {
    size(W, H, ratio) {
      if (!W || !H) return false;
      dpr = ratio || 1;
      w = W;
      hgt = H;
      ctx.canvas.width = Math.round(W * dpr);
      ctx.canvas.height = Math.round(H * dpr);
      return true;
    },
    pointer(x, y) {
      px = x;
      py = y;
    },
    draw(t) {
      if (!w || !hgt) return;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.globalCompositeOperation = 'source-over';
      ctx.globalAlpha = 1;
      ctx.fillStyle = '#0b0614';
      ctx.fillRect(0, 0, w, hgt);
      // The sun leans a little towards the pointer.
      const cx = w * (0.5 + (px - 0.5) * 0.08);
      const cy = hgt * (0.62 + (py - 0.5) * 0.08);
      const R = Math.hypot(w, hgt) * 0.75;
      const breathe = 1 + 0.03 * Math.sin(t * 0.6);
      for (let k = RINGS - 1; k >= 0; k--) {
        const f = k / RINGS;
        const base = R * (0.035 + 0.965 * Math.pow(f, 1.25)) * breathe;
        const amp = 0.05 + 0.035 * Math.sin(t * 0.35 + k * 0.6);
        const spin = t * 0.045 * (k % 2 ? 1 : -1);
        ctx.beginPath();
        for (let j = 0; j <= SEG; j++) {
          const th = (j / SEG) * Math.PI * 2;
          const r = base * (1 + amp * Math.sin(7 * (th + spin) + t * 0.5 + k * 0.42) + 0.022 * Math.sin(13 * th - t * 0.9 + k));
          const x = cx + Math.cos(th) * r;
          const y = cy + Math.sin(th) * r;
          if (j) ctx.lineTo(x, y);
          else ctx.moveTo(x, y);
        }
        ctx.closePath();
        ctx.fillStyle = mix(f * 1.15 - t * 0.035, k % 2 ? 0.42 : 0.04);
        ctx.fill();
      }
      // The glowing core.
      ctx.globalCompositeOperation = 'screen';
      const core = ctx.createRadialGradient(cx, cy, 0, cx, cy, R * 0.16);
      core.addColorStop(0, 'rgba(255,240,214,0.95)');
      core.addColorStop(0.35, 'rgba(255,186,120,0.45)');
      core.addColorStop(1, 'rgba(255,120,120,0)');
      ctx.fillStyle = core;
      ctx.fillRect(0, 0, w, hgt);
      // Film grain.
      if (!grain) grain = makeGrain();
      ctx.globalCompositeOperation = 'overlay';
      ctx.globalAlpha = 0.07;
      ctx.setTransform(dpr, 0, 0, dpr, (t * 37) % 160, (t * 23) % 160);
      ctx.fillStyle = grain;
      ctx.fillRect(-160, -160, w + 320, hgt + 320);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = 'source-over';
    },
  };
}

function prismLoop(render, fps) {
  const minFrame = 1000 / fps - 2;
  let t = 0;
  let last = 0;
  let raf = 0;
  let on = false;
  const frame = (now) => {
    raf = 0;
    if (!on) return;
    if (!last || now - last >= minFrame) {
      if (last) t += Math.min(0.05, (now - last) / 1000);
      last = now;
      render(t);
    }
    raf = requestAnimationFrame(frame);
  };
  return {
    get t() {
      return t;
    },
    run(yes) {
      on = yes;
      if (yes && !raf) {
        last = 0;
        raf = requestAnimationFrame(frame);
      } else if (!yes && raf) {
        cancelAnimationFrame(raf);
        raf = 0;
      }
    },
  };
}

if (typeof document === 'undefined') {
  // ── Worker side ──
  let r = null;
  let loop = null;
  self.onmessage = (e) => {
    const m = e.data || {};
    if (m.canvas) {
      r = prismRenderer(m.canvas.getContext('2d'));
      loop = prismLoop((t) => r.draw(t), m.fps || 60);
    }
    if (m.size && r && r.size(m.size.w, m.size.h, m.size.dpr)) r.draw(loop.t);
    if (m.pointer && r) r.pointer(m.pointer.x, m.pointer.y);
    if (typeof m.run === 'boolean' && loop) loop.run(m.run);
  };
}

// ── Page side ──
function prismCanvas(opts = {}) {
  const fps = opts.fps || 60;
  const canvas = h('canvas', { class: 'prism', 'aria-hidden': 'true' });
  const still = () => document.documentElement.dataset.motion === 'off' || matchMedia('(prefers-reduced-motion: reduce)').matches;
  let onScreen = true;
  const wanted = () => !still() && !document.hidden && onScreen && canvas.isConnected;
  let worker = null;
  let local = null;
  if (typeof OffscreenCanvas !== 'undefined' && canvas.transferControlToOffscreen) {
    try {
      const off = canvas.transferControlToOffscreen();
      worker = new Worker('/_/prism.js');
      worker.postMessage({ canvas: off, fps }, [off]);
    } catch {
      worker = null;
    }
  }
  if (!worker) {
    const r = prismRenderer(canvas.getContext('2d'));
    local = { r, loop: prismLoop((t) => r.draw(t), Math.min(fps, 30)) };
  }
  const send = (m) => {
    if (worker) worker.postMessage(m);
    else if (local) {
      if (m.size && local.r.size(m.size.w, m.size.h, m.size.dpr)) local.r.draw(local.loop.t);
      if (m.pointer) local.r.pointer(m.pointer.x, m.pointer.y);
      if (typeof m.run === 'boolean') local.loop.run(m.run);
    }
  };
  const size = () => {
    const rect = canvas.getBoundingClientRect();
    send({ size: { w: rect.width, h: rect.height, dpr: Math.min(1.5, window.devicePixelRatio || 1) } });
  };
  const update = () => send({ run: wanted() });
  let pending = 0;
  window.addEventListener('pointermove', (e) => {
    if (pending) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      send({ pointer: { x: e.clientX / Math.max(1, window.innerWidth), y: e.clientY / Math.max(1, window.innerHeight) } });
    });
  }, { passive: true });
  new IntersectionObserver((entries) => {
    onScreen = entries.some((x) => x.isIntersecting);
    update();
  }).observe(canvas);
  new ResizeObserver(size).observe(canvas);
  document.addEventListener('visibilitychange', update);
  requestAnimationFrame(() => {
    size();
    update();
  });
  return canvas;
}
