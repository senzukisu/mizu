// The "current": violet streamlines flowing around a chrome drop (About Mizu, Welcome).
// The drawing runs in a worker on an OffscreenCanvas, so the page itself never waits for it:
// hover and clicks stay instant at 60 fps. Without OffscreenCanvas it draws on the page at
// a lower rate. It pauses off screen, in hidden tabs and with reduced motion.
// This file is both the page script (currentsCanvas) and the worker (when there is no DOM).
'use strict';

// ── The renderer: pure canvas drawing, usable on the page or in a worker ──
function currentsRenderer(ctx) {
  let w = 0;
  let hgt = 0;
  // Geometry that only depends on the size: computed on resize, reused by every frame.
  let geo = null;

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

  // Solves every streamline once. Per line we keep, for each sample along the current, the
  // undisturbed offset and how strongly the wake ripples there; frames only add the ripple.
  function build() {
    if (!w || !hgt) return null;
    const R = Math.min(w, hgt) * 0.15;
    const cx = w * 0.7;
    const cy = hgt * 0.5;
    const a = 0.36; // the current runs diagonally, top-left to bottom-right
    const ca = Math.cos(a);
    const sa = Math.sin(a);
    const L = Math.hypot(w, hgt);
    const step = 6;
    const du = 6;
    const lines = [];
    const span = Math.ceil(L / step);
    for (let n = -span; n <= span; n++) {
      const c = (n + 0.5) * step;
      const near = Math.exp(-Math.abs(c) / (R * 3.5));
      const us = [];
      const vs = [];
      const amps = [];
      for (let u = -L; u <= L; u += du) {
        const v = solveV(u, c, R);
        const x = cx + u * ca - v * sa;
        const y = cy + u * sa + v * ca;
        // Keep a margin for the ripple; points far outside the canvas are dropped.
        if (x < -30 || x > w + 30 || y < -30 || y > hgt + 30) continue;
        const behind = Math.max(0, Math.min(1, (u - R * 0.5) / (R * 4)));
        us.push(u);
        vs.push(v);
        amps.push(0.5 + behind * near * 9);
      }
      if (us.length > 1) lines.push({ c, us: Float32Array.from(us), vs: Float32Array.from(vs), amps: Float32Array.from(amps) });
    }
    const bg = ctx.createLinearGradient(0, 0, w, hgt);
    bg.addColorStop(0, '#12061f');
    bg.addColorStop(1, '#1d0a33');
    const ink = ctx.createLinearGradient(0, 0, w, hgt);
    ink.addColorStop(0, '#6d3fb8');
    ink.addColorStop(0.55, '#a77bff');
    ink.addColorStop(1, '#d6bcff');
    return { R, cx, cy, ca, sa, L, step, lines, bg, ink };
  }

  function draw(t) {
    if (!geo) return;
    const { R, cx, cy, ca, sa, L, step, lines, bg, ink } = geo;
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, hgt);

    // All streamlines share one style: one path, one stroke.
    ctx.strokeStyle = ink;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.lineWidth = step * 0.48;
    ctx.beginPath();
    const phase = t * 1.6;
    for (const ln of lines) {
      const { c, us, vs, amps } = ln;
      const cc = c * 0.07 - phase;
      for (let k = 0; k < us.length; k++) {
        const u = us[k];
        const v = vs[k] + Math.sin(u * 0.045 + cc) * amps[k];
        const x = cx + u * ca - v * sa;
        const y = cy + u * sa + v * ca;
        if (k === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

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

  return {
    /// New CSS size and device pixel ratio; rebuilds the geometry when it really changed.
    size(nw, nh, dpr) {
      if (Math.abs(nw - w) < 0.5 && Math.abs(nh - hgt) < 0.5 && geo) return false;
      w = nw;
      hgt = nh;
      ctx.canvas.width = Math.max(1, Math.round(w * dpr));
      ctx.canvas.height = Math.max(1, Math.round(hgt * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      geo = build();
      return true;
    },
    draw,
  };
}

// A frame loop that advances time by real elapsed seconds, at most `fps` times a second.
function currentsLoop(render, fps) {
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
  // ── Worker side: owns the OffscreenCanvas and the frame loop ──
  let r = null;
  let loop = null;
  self.onmessage = (e) => {
    const m = e.data || {};
    if (m.canvas) {
      r = currentsRenderer(m.canvas.getContext('2d'));
      loop = currentsLoop((t) => r.draw(t), m.fps || 60);
    }
    if (m.size && r && r.size(m.size.w, m.size.h, m.size.dpr)) r.draw(loop.t);
    if (typeof m.run === 'boolean' && loop) loop.run(m.run);
  };
}

// ── Page side ──
function currentsCanvas(opts = {}) {
  const fps = opts.fps || 60;
  let canvas = h('canvas', { class: 'currents', 'aria-hidden': 'true' });
  const cls = opts.className || '';
  if (cls) canvas.classList.add(cls);
  const still = () => document.documentElement.dataset.motion === 'off' || matchMedia('(prefers-reduced-motion: reduce)').matches;
  let onScreen = true;
  const wanted = () => !still() && !document.hidden && onScreen && canvas.isConnected;

  let worker = null;
  let local = null;
  let scrollingUntil = 0;

  // Fallback on the page: fewer frames, and none while scrolling.
  const drawHere = () => {
    const r = currentsRenderer(canvas.getContext('2d'));
    const loop = currentsLoop((t) => {
      if (performance.now() >= scrollingUntil) r.draw(t);
    }, Math.min(fps, 30));
    local = { r, loop };
  };
  document.addEventListener('scroll', () => (scrollingUntil = performance.now() + 180), { capture: true, passive: true });

  // Preferred: a worker draws, the page only reports size and visibility.
  if (typeof OffscreenCanvas !== 'undefined' && canvas.transferControlToOffscreen) {
    try {
      const off = canvas.transferControlToOffscreen();
      worker = new Worker('/_/currents.js');
      worker.postMessage({ canvas: off, fps }, [off]);
      // The canvas now belongs to the worker: if it cannot start, put a fresh one in place.
      worker.addEventListener('error', () => {
        worker = null;
        const fresh = h('canvas', { class: canvas.className, 'aria-hidden': 'true' });
        canvas.replaceWith(fresh);
        observe(fresh);
        canvas = fresh;
        drawHere();
        size();
        update();
      });
    } catch {
      worker = null;
    }
  }
  if (!worker) drawHere();

  function size() {
    const rect = canvas.getBoundingClientRect();
    const s = { w: rect.width, h: rect.height, dpr: Math.min(1.5, window.devicePixelRatio || 1) };
    if (worker) worker.postMessage({ size: s });
    else if (local && local.r.size(s.w, s.h, s.dpr)) local.r.draw(local.loop.t);
  }
  function update() {
    const run = wanted();
    if (worker) worker.postMessage({ run });
    else if (local) local.loop.run(run);
  }
  const io = new IntersectionObserver((entries) => {
    onScreen = entries.some((e) => e.isIntersecting);
    update();
  });
  const ro = new ResizeObserver(size);
  function observe(el) {
    io.disconnect();
    ro.disconnect();
    io.observe(el);
    ro.observe(el);
  }
  observe(canvas);
  document.addEventListener('visibilitychange', update);
  requestAnimationFrame(() => {
    size();
    update();
  });
  return canvas;
}
