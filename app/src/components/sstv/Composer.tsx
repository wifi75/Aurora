import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent as RPointerEvent, type ReactNode, type WheelEvent as RWheelEvent } from 'react';
import { ApiError, sstvAbort, sstvTransmit, type SstvSnapshot } from '../../api/client';
import { useT, type Key } from '../../i18n';
import {
  angleFrom,
  duplicateLayer,
  emptyComposition,
  FONTS,
  formatDuration,
  historyPush,
  historyRedo,
  historyStart,
  historyUndo,
  moveLayer,
  NEUTRAL_ADJUST,
  PALETTES,
  parseTemplates,
  pointInRotatedRect,
  presetLayers,
  shapeLayer,
  textLayer,
  updateLayer,
  type Background,
  type Composition,
  type Fill,
  type Layer,
  type SavedTemplate,
  type ShapeLayer,
  type TextLayer,
  type TokenContext,
} from '../../lib/sstv';
import { layerBox, loadImageFile, renderComposition } from '../../lib/sstvRender';
import { browserStorage } from '../../lib/theme';
import { useStatusPick } from '../../state/station';

const TEMPLATES_KEY = 'aurora.sstv.templates';

export interface IncomingImage {
  blob: Blob;
  call?: string | null;
  /** Cresce a ogni richiesta, anche con la stessa immagine. */
  seq: number;
}

type Drag =
  | { kind: 'move'; id: string; dx: number; dy: number; before: Composition }
  | { kind: 'resize'; id: string; before: Composition; cx: number; cy: number; base: number; startDist: number; w0: number; h0: number }
  | { kind: 'rotate'; id: string; before: Composition; cx: number; cy: number }
  | { kind: 'pan'; before: Composition; x: number; y: number; panX: number; panY: number };

function loadTemplates(): SavedTemplate[] {
  try {
    return parseTemplates(browserStorage()?.getItem(TEMPLATES_KEY) ?? null);
  } catch {
    return [];
  }
}

/** Il compositore: foto di sfondo inquadrata, livelli di testo e forme, modelli, anteprima alla risoluzione del modo, «Trasmetti». */
export function Composer({ snap, incoming, onChanged, sendRequest }: { snap: SstvSnapshot; incoming: IncomingImage | null; onChanged: () => void; sendRequest?: number }) {
  const t = useT();
  const station = useStatusPick(['de_call', 'de_grid', 'dx_call', 'report']);
  const [modeId, setModeId] = useState(snap.settings.default_mode);
  const mode = snap.modes.find((m) => m.id === modeId) ?? snap.modes[0];
  const W = mode?.width ?? 320;
  const H = mode?.height ?? 256;
  const [hist, setHist] = useState(() => historyStart<Composition>({ ...emptyComposition(), layers: presetLayers('cq') }));
  const comp = hist.present;
  const [selected, setSelected] = useState<string | null>(null);
  const [bg, setBg] = useState<HTMLCanvasElement | null>(null);
  const [call, setCall] = useState('');
  const [problem, setProblem] = useState('');
  const [dropping, setDropping] = useState(false);
  const [templates, setTemplates] = useState<SavedTemplate[]>(loadTemplates);
  const [fontsTick, setFontsTick] = useState(0);
  const canvas = useRef<HTMLCanvasElement>(null);
  const overlay = useRef<HTMLCanvasElement>(null);
  const stage = useRef<HTMLDivElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const drag = useRef<Drag | null>(null);
  const lastEdit = useRef<{ key: string; at: number }>({ key: '', at: 0 });

  const theirCall = (call || station?.dx_call || '').toUpperCase();
  const tokens: TokenContext = useMemo(
    () => ({ mycall: station?.de_call, call: theirCall, rst: station?.report || null, grid: station?.de_grid, mode: mode?.name }),
    [station?.de_call, station?.de_grid, station?.report, theirCall, mode?.name],
  );

  /** Modifica con annulla: i cambi a raffica dello stesso campo (cursori) diventano un passo solo. */
  const commit = useCallback((next: Composition, key = '') => {
    const now = Date.now();
    const coalesce = key !== '' && lastEdit.current.key === key && now - lastEdit.current.at < 800;
    lastEdit.current = { key, at: now };
    setHist((h) => (coalesce ? { ...h, present: next } : historyPush(h, next)));
  }, []);
  const setLive = (next: Composition) => setHist((h) => ({ ...h, present: next }));

  // i caratteri inclusi si caricano quando servono: poi si ridisegna
  useEffect(() => {
    const fonts = new Set(comp.layers.filter((l): l is TextLayer => l.kind === 'text').map((l) => l.font));
    let gone = false;
    Promise.all([...fonts].map((f) => document.fonts?.load(`32px "${f}"`).catch(() => [])))
      .then(() => {
        if (!gone) setFontsTick((n) => n + 1);
      })
      .catch(() => undefined);
    return () => {
      gone = true;
    };
  }, [comp.layers]);

  // disegno dell'immagine vera (quella che si trasmette)
  useLayoutEffect(() => {
    const c = canvas.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    if (c.width !== W) c.width = W;
    if (c.height !== H) c.height = H;
    renderComposition(ctx, comp, bg, tokens);
  }, [comp, bg, tokens, W, H, fontsTick]);

  // maniglie della selezione su un canvas a parte, alla risoluzione dello schermo (nitide)
  const drawOverlay = useCallback(() => {
    const o = overlay.current;
    const c = canvas.current;
    if (!o || !c) return;
    const rect = c.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    o.width = Math.max(1, Math.round(rect.width * dpr));
    o.height = Math.max(1, Math.round(rect.height * dpr));
    const ctx = o.getContext('2d');
    const mctx = c.getContext('2d');
    if (!ctx || !mctx) return;
    ctx.clearRect(0, 0, o.width, o.height);
    const layer = comp.layers.find((l) => l.id === selected);
    if (!layer || layer.hidden) return;
    const s = o.width / W;
    const box = layerBox(mctx, layer, W, H, tokens);
    ctx.save();
    ctx.translate(box.cx * s, box.cy * s);
    ctx.rotate((layer.rotation * Math.PI) / 180);
    const w = box.w * s;
    const h = box.h * s;
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 3 * dpr;
    ctx.setLineDash([]);
    ctx.strokeRect(-w / 2, -h / 2, w, h);
    ctx.strokeStyle = '#0a84ff';
    ctx.lineWidth = 1.5 * dpr;
    ctx.setLineDash([6 * dpr, 4 * dpr]);
    ctx.strokeRect(-w / 2, -h / 2, w, h);
    ctx.setLineDash([]);
    const knob = (x: number, y: number) => {
      ctx.beginPath();
      ctx.arc(x, y, 7 * dpr, 0, Math.PI * 2);
      ctx.fillStyle = '#0a84ff';
      ctx.fill();
      ctx.lineWidth = 2 * dpr;
      ctx.strokeStyle = '#ffffff';
      ctx.stroke();
    };
    ctx.beginPath();
    ctx.moveTo(0, -h / 2);
    ctx.lineTo(0, -h / 2 - 22 * dpr);
    ctx.strokeStyle = '#0a84ff';
    ctx.stroke();
    knob(0, -h / 2 - 22 * dpr);
    knob(w / 2, h / 2);
    ctx.restore();
  }, [comp.layers, selected, tokens, W, H]);

  useEffect(() => {
    drawOverlay();
  }, [drawOverlay, fontsTick]);
  useEffect(() => {
    const el = stage.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => drawOverlay());
    ro.observe(el);
    return () => ro.disconnect();
  }, [drawOverlay]);

  const takeFile = useCallback(
    async (file: Blob) => {
      const img = await loadImageFile(file);
      if (!img) {
        setProblem(t('sstv.bg.badFile'));
        return;
      }
      setProblem('');
      setBg(img);
      setHist((h) => historyPush(h, { ...h.present, background: { ...h.present.background, zoom: 1, panX: 0, panY: 0 } }));
    },
    [t],
  );

  // «Rispondi» / «Usa come sfondo» dalla cronologia
  useEffect(() => {
    if (!incoming) return;
    void takeFile(incoming.blob);
    if (incoming.call) setCall(incoming.call);
  }, [incoming, takeFile]);

  // incolla una foto (Ctrl+V) quando il compositore e' in vista
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      if (!stage.current || stage.current.offsetParent === null) return;
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
      const item = [...(e.clipboardData?.items ?? [])].find((i) => i.type.startsWith('image/'));
      const file = item?.getAsFile();
      if (file) {
        e.preventDefault();
        void takeFile(file);
      }
    };
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, [takeFile]);

  // annulla/ripeti da tastiera
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!stage.current || stage.current.offsetParent === null) return;
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        setHist((h) => (e.shiftKey ? historyRedo(h) : historyUndo(h)));
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        setHist(historyRedo);
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && selected) {
        e.preventDefault();
        setHist((h) => historyPush(h, { ...h.present, layers: h.present.layers.filter((l) => l.id !== selected) }));
        setSelected(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selected]);

  const toImage = (e: { clientX: number; clientY: number }) => {
    const rect = canvas.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return { x: 0, y: 0, s: 1 };
    return { x: ((e.clientX - rect.left) / rect.width) * W, y: ((e.clientY - rect.top) / rect.height) * H, s: rect.width / W };
  };

  const onDown = (e: RPointerEvent<HTMLCanvasElement>) => {
    const mctx = canvas.current?.getContext('2d');
    if (!mctx) return;
    const p = toImage(e);
    e.currentTarget.setPointerCapture(e.pointerId);
    const before = comp;
    const sel = comp.layers.find((l) => l.id === selected && !l.hidden);
    if (e.button === 0 && sel) {
      const box = layerBox(mctx, sel, W, H, tokens);
      const a = (sel.rotation * Math.PI) / 180;
      const local = (lx: number, ly: number) => ({ x: box.cx + lx * Math.cos(a) - ly * Math.sin(a), y: box.cy + lx * Math.sin(a) + ly * Math.cos(a) });
      const rot = local(0, -box.h / 2 - 22 / p.s);
      const res = local(box.w / 2, box.h / 2);
      const near = (q: { x: number; y: number }) => Math.hypot(q.x - p.x, q.y - p.y) < 14 / p.s;
      if (near(rot)) {
        drag.current = { kind: 'rotate', id: sel.id, before, cx: box.cx, cy: box.cy };
        return;
      }
      if (near(res)) {
        const base = sel.kind === 'text' ? sel.size : 0;
        drag.current = { kind: 'resize', id: sel.id, before, cx: box.cx, cy: box.cy, base, startDist: Math.max(1, Math.hypot(p.x - box.cx, p.y - box.cy)), w0: sel.kind === 'shape' ? sel.w : 0, h0: sel.kind === 'shape' ? sel.h : 0 };
        return;
      }
    }
    if (e.button === 0) {
      for (let i = comp.layers.length - 1; i >= 0; i--) {
        const l = comp.layers[i];
        if (!l || l.hidden) continue;
        const box = layerBox(mctx, l, W, H, tokens);
        if (pointInRotatedRect(p.x, p.y, box.cx, box.cy, box.w, box.h, l.rotation)) {
          setSelected(l.id);
          drag.current = { kind: 'move', id: l.id, dx: p.x - l.x * W, dy: p.y - l.y * H, before };
          return;
        }
      }
      setSelected(null);
    }
    // vuoto, tasto destro o due dita: si inquadra la foto
    drag.current = { kind: 'pan', before, x: p.x, y: p.y, panX: comp.background.panX, panY: comp.background.panY };
  };

  const onMove = (e: RPointerEvent<HTMLCanvasElement>) => {
    const d = drag.current;
    if (!d) return;
    const p = toImage(e);
    if (d.kind === 'move') {
      setLive({ ...comp, layers: updateLayer(comp.layers, d.id, { x: Math.max(0, Math.min(1, (p.x - d.dx) / W)), y: Math.max(0, Math.min(1, (p.y - d.dy) / H)) }) });
    } else if (d.kind === 'rotate') {
      let deg = angleFrom(d.cx, d.cy, p.x, p.y);
      // si aggancia ogni 15 gradi se si e' vicini
      const snapTo = Math.round(deg / 15) * 15;
      if (Math.abs(snapTo - deg) < 4) deg = snapTo % 360;
      setLive({ ...comp, layers: updateLayer(comp.layers, d.id, { rotation: deg > 180 ? deg - 360 : deg }) });
    } else if (d.kind === 'resize') {
      const k = Math.max(0.05, Math.hypot(p.x - d.cx, p.y - d.cy) / d.startDist);
      const l = comp.layers.find((x) => x.id === d.id);
      if (l?.kind === 'text') setLive({ ...comp, layers: updateLayer(comp.layers, d.id, { size: Math.max(0.02, Math.min(1, d.base * k)) }) });
      else if (l) setLive({ ...comp, layers: updateLayer(comp.layers, d.id, { w: Math.max(0.02, Math.min(2, d.w0 * k)), h: Math.max(0.02, Math.min(2, d.h0 * k)) }) });
    } else if (d.kind === 'pan' && bg) {
      const dx = (p.x - d.x) / (W / 2);
      const dy = (p.y - d.y) / (H / 2);
      setLive({ ...comp, background: { ...comp.background, panX: Math.max(-1, Math.min(1, d.panX - dx)), panY: Math.max(-1, Math.min(1, d.panY - dy)) } });
    }
  };

  const onUp = () => {
    const d = drag.current;
    drag.current = null;
    if (!d) return;
    setHist((h) => (h.present === d.before ? h : { past: [...h.past, d.before].slice(-60), present: h.present, future: [] }));
  };

  const onWheel = (e: RWheelEvent<HTMLCanvasElement>) => {
    if (!bg) return;
    const zoom = Math.max(1, Math.min(6, comp.background.zoom * (e.deltaY < 0 ? 1.08 : 1 / 1.08)));
    commit({ ...comp, background: { ...comp.background, zoom } }, 'zoom');
  };

  const sel = comp.layers.find((l) => l.id === selected) ?? null;
  const patchSel = (patch: Partial<TextLayer> | Partial<ShapeLayer>, key: string) => {
    if (sel) commit({ ...comp, layers: updateLayer(comp.layers, sel.id, patch) }, `${sel.id}:${key}`);
  };
  const patchBg = (patch: Partial<Background>, key: string) => commit({ ...comp, background: { ...comp.background, ...patch } }, `bg:${key}`);
  const add = (l: Layer) => {
    commit({ ...comp, layers: [...comp.layers, l] });
    setSelected(l.id);
  };

  // trasmissione
  const sending = snap.tx.state === 'sending';
  const transmit = () => {
    setProblem('');
    canvas.current?.toBlob((blob) => {
      if (!blob || !mode) return;
      sstvTransmit(mode.id, blob, theirCall || null)
        .then(onChanged)
        .catch((e: unknown) => {
          const code = e instanceof ApiError ? e.code : '';
          setProblem(code === 'tx_disabled' ? t('sstv.tx.disabled') : code === 'busy' ? t('sstv.tx.busy') : code === 'no_transmitter' ? t('sstv.tx.notReady') : e instanceof ApiError && e.detail ? e.detail : t('sstv.tx.error'));
        });
    }, 'image/png');
  };
  // il pulsante TX della pagina di ricezione chiede di trasmettere l'immagine preparata qui: parte da sola, senza premere ancora «Trasmetti»
  const lastRequest = useRef(sendRequest ?? 0);
  useEffect(() => {
    if (sendRequest === undefined || sendRequest === lastRequest.current) return;
    lastRequest.current = sendRequest;
    if (snap.can_transmit && !sending) transmit();
  });

  const saveTemplate = () => {
    const name = window.prompt(t('sstv.tpl.name'), '')?.trim();
    if (!name) return;
    let background: string | undefined;
    if (bg) {
      const small = document.createElement('canvas');
      const k = Math.min(1, 1280 / Math.max(bg.width, bg.height));
      small.width = Math.round(bg.width * k);
      small.height = Math.round(bg.height * k);
      small.getContext('2d')?.drawImage(bg, 0, 0, small.width, small.height);
      const url = small.toDataURL('image/jpeg', 0.85);
      if (url.length < 1_500_000) background = url;
    }
    const next = [...templates.filter((x) => x.name !== name), { name, composition: comp, background }];
    try {
      browserStorage()?.setItem(TEMPLATES_KEY, JSON.stringify(next));
    } catch {
      // spazio del browser finito: si tiene il modello senza foto
      const lighter = next.map((x) => (x.name === name ? { ...x, background: undefined } : x));
      try {
        browserStorage()?.setItem(TEMPLATES_KEY, JSON.stringify(lighter));
      } catch {
        return;
      }
    }
    setTemplates(loadTemplates());
  };
  const useTemplate = async (tpl: SavedTemplate) => {
    commit(tpl.composition);
    setSelected(null);
    if (tpl.background) {
      const blob = await (await fetch(tpl.background)).blob();
      const img = await loadImageFile(blob);
      setBg(img);
    }
  };
  const deleteTemplate = (name: string) => {
    const next = templates.filter((x) => x.name !== name);
    try {
      browserStorage()?.setItem(TEMPLATES_KEY, JSON.stringify(next));
    } catch {
      return;
    }
    setTemplates(next);
  };

  const layerName = (l: Layer): string => (l.kind === 'text' ? l.text.split('\n')[0] || t('sstv.layer.text') : t(`sstv.layer.${l.shape === 'rect' ? 'rect' : l.shape}` as Key));

  return (
    <section className="sstv-composer">
      <div className="sstv-toolbar">
        <label className="sstv-field sstv-field--mode">
          <span>{t('sstv.tx.mode')}</span>
          <select value={modeId} onChange={(e) => setModeId(e.target.value)} disabled={sending}>
            {snap.modes.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name} — {m.width}×{m.height} · {formatDuration(m.seconds)}
              </option>
            ))}
          </select>
        </label>
        <label className="sstv-field sstv-field--call">
          <span>{t('sstv.tx.to')}</span>
          <input value={call} placeholder={station?.dx_call ?? ''} onChange={(e) => setCall(e.target.value.toUpperCase().replace(/[^A-Z0-9/]/g, '').slice(0, 16))} />
        </label>
        <div className="sstv-toolbar-edit">
          <button type="button" className="glass sstv-btn" disabled={hist.past.length === 0} onClick={() => setHist(historyUndo)}>
            {t('sstv.undo')}
          </button>
          <button type="button" className="glass sstv-btn" disabled={hist.future.length === 0} onClick={() => setHist(historyRedo)}>
            {t('sstv.redo')}
          </button>
          <button
            type="button"
            className="glass sstv-btn"
            onClick={() => {
              commit(emptyComposition());
              setBg(null);
              setSelected(null);
            }}
          >
            {t('sstv.clear')}
          </button>
        </div>
      </div>

      <div className="sstv-editor">
        <aside className="sstv-panel sstv-panel--left">
          <h3>{t('sstv.layers')}</h3>
          <div className="sstv-add">
            <button type="button" className="glass sstv-btn" onClick={() => add(textLayer('{MYCALL}'))}>
              {t('sstv.add.text')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => add(shapeLayer('rect', { fill: { kind: 'solid', color: '#000000' }, opacity: 0.55, stroke: 0 }))}>
              {t('sstv.add.rect')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => add(shapeLayer('ellipse'))}>
              {t('sstv.add.ellipse')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => add(shapeLayer('frame'))}>
              {t('sstv.add.frame')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => add(shapeLayer('bars', { y: 0.94, w: 1, h: 0.12, filled: true, stroke: 0 }))}>
              {t('sstv.add.bars')}
            </button>
          </div>
          {comp.layers.length === 0 ? <p className="sstv-hint">{t('sstv.layers.empty')}</p> : null}
          <ol className="sstv-layers">
            {[...comp.layers].reverse().map((l) => (
              <li key={l.id} className={l.id === selected ? 'sstv-layer sstv-layer--on' : 'sstv-layer'}>
                <button type="button" className="sstv-layer-name" onClick={() => setSelected(l.id)}>
                  <span className={`sstv-layer-kind sstv-layer-kind--${l.kind}`} aria-hidden="true" />
                  <span className={l.hidden ? 'sstv-muted' : ''}>{layerName(l)}</span>
                </button>
                <span className="sstv-layer-tools">
                  <button type="button" onClick={() => commit({ ...comp, layers: moveLayer(comp.layers, l.id, 1) })}>
                    {t('sstv.layer.up')}
                  </button>
                  <button type="button" onClick={() => commit({ ...comp, layers: moveLayer(comp.layers, l.id, -1) })}>
                    {t('sstv.layer.down')}
                  </button>
                  <button type="button" onClick={() => commit({ ...comp, layers: updateLayer(comp.layers, l.id, { hidden: !l.hidden }) })}>
                    {l.hidden ? t('sstv.layer.show') : t('sstv.layer.hide')}
                  </button>
                  <button type="button" onClick={() => commit({ ...comp, layers: duplicateLayer(comp.layers, l.id) })}>
                    {t('sstv.layer.dup')}
                  </button>
                  <button
                    type="button"
                    className="sstv-danger"
                    onClick={() => {
                      commit({ ...comp, layers: comp.layers.filter((x) => x.id !== l.id) });
                      if (selected === l.id) setSelected(null);
                    }}
                  >
                    {t('sstv.layer.del')}
                  </button>
                </span>
              </li>
            ))}
          </ol>

          <h3>{t('sstv.tpl.title')}</h3>
          <div className="sstv-add">
            {(['cq', 'reply', 'qsl'] as const).map((k) => (
              <button
                key={k}
                type="button"
                className="glass sstv-btn"
                onClick={() => {
                  commit({ ...comp, layers: presetLayers(k) });
                  setSelected(null);
                }}
              >
                {t(`sstv.tpl.${k}`)}
              </button>
            ))}
          </div>
          <button type="button" className="glass sstv-btn sstv-wide" onClick={saveTemplate}>
            {t('sstv.tpl.save')}
          </button>
          {templates.length > 0 ? (
            <>
              <h4>{t('sstv.tpl.mine')}</h4>
              <ul className="sstv-templates">
                {templates.map((tpl) => (
                  <li key={tpl.name}>
                    <button type="button" className="sstv-layer-name" onClick={() => void useTemplate(tpl)}>
                      {tpl.name}
                    </button>
                    <button type="button" className="sstv-danger" title={t('sstv.tpl.delete')} onClick={() => deleteTemplate(tpl.name)}>
                      {t('sstv.layer.del')}
                    </button>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
        </aside>

        <div className="sstv-center">
          <div
            ref={stage}
            className={`sstv-artboard${dropping ? ' sstv-artboard--drop' : ''}`}
            style={{ aspectRatio: `${W} / ${H}` }}
            onDragOver={(e) => {
              if ([...e.dataTransfer.items].some((i) => i.kind === 'file')) {
                e.preventDefault();
                setDropping(true);
              }
            }}
            onDragLeave={() => setDropping(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDropping(false);
              const file = [...e.dataTransfer.files].find((f) => f.type.startsWith('image/') || /\.(jpe?g|png|webp|gif|bmp)$/i.test(f.name));
              if (file) void takeFile(file);
              else setProblem(t('sstv.bg.badFile'));
            }}
          >
            <canvas ref={canvas} width={W} height={H} className="sstv-canvas sstv-canvas--edit" />
            <canvas
              ref={overlay}
              className="sstv-overlay"
              onPointerDown={onDown}
              onPointerMove={onMove}
              onPointerUp={onUp}
              onPointerCancel={onUp}
              onContextMenu={(e) => e.preventDefault()}
              onWheel={onWheel}
              aria-label={t('sstv.p.hint')}
            />
            {/* in trasmissione: la linea che scende dove si e' arrivati; sopra, la parte gia' inviata e' piu' scura */}
            {sending ? (
              <>
                <div className="sstv-txsent" style={{ height: `${Math.min(100, snap.tx.progress * 100)}%` }} aria-hidden="true" />
                <div className="sstv-txline" style={{ top: `${Math.min(100, snap.tx.progress * 100)}%` }} aria-hidden="true" />
              </>
            ) : null}
            {!bg && !sending ? <div className="sstv-drop-hint">{t('sstv.bg.drop')}</div> : null}
          </div>
          <p className="sstv-hint sstv-preview-cap">{t('sstv.preview', { w: W, h: H })}</p>

          <div className="sstv-sendbar">
            {sending ? (
              <>
                <div className="sstv-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(snap.tx.progress * 100)}>
                  <span style={{ width: `${snap.tx.progress * 100}%` }} />
                  <em>{t('sstv.tx.progress', { mode: snap.modes.find((m) => m.id === snap.tx.mode)?.name ?? '', done: formatDuration(snap.tx.elapsed_s), total: formatDuration(snap.tx.total_s) })}</em>
                </div>
                <button type="button" className="glass sstv-btn sstv-btn--danger sstv-btn--big" onClick={() => void sstvAbort().then(onChanged)}>
                  {t('sstv.tx.stop')}
                </button>
              </>
            ) : (
              <>
                <span className="sstv-sendinfo">
                  <b>{mode?.name}</b> · {W}×{H} · {formatDuration(mode?.seconds ?? 0)}
                  {snap.tx.state === 'done' ? <span className="sstv-ok"> · {t('sstv.tx.done')}</span> : null}
                  {snap.tx.state === 'aborted' ? <span className="sstv-warn"> · {t('sstv.tx.aborted')}</span> : null}
                  {snap.tx.state === 'error' ? <span className="sstv-bad"> · {snap.tx.message ?? t('sstv.tx.error')}</span> : null}
                </span>
                <button type="button" className="glass sstv-btn sstv-btn--primary sstv-btn--big" disabled={!snap.can_transmit} onClick={transmit}>
                  {t('sstv.tx.send')}
                </button>
              </>
            )}
          </div>
          {!snap.can_transmit ? <p className="notice">{t('sstv.tx.notReady')}</p> : null}
          {problem ? (
            <p className="notice sstv-bad" role="alert">
              {problem}
            </p>
          ) : null}
        </div>

        <aside className="sstv-panel sstv-panel--right">
          {sel ? <LayerInspector layer={sel} patch={patchSel} /> : <BackgroundInspector background={comp.background} hasPhoto={bg !== null} patch={patchBg} onPick={() => fileInput.current?.click()} onRemove={() => setBg(null)} />}
          <input
            ref={fileInput}
            type="file"
            accept="image/jpeg,image/png,image/webp,image/gif,image/bmp,.jpg,.jpeg,.png,.webp,.gif,.bmp"
            hidden
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void takeFile(f);
              e.target.value = '';
            }}
          />
          {sel ? (
            <button type="button" className="glass sstv-btn sstv-wide" onClick={() => setSelected(null)}>
              {t('sstv.bg.title')}
            </button>
          ) : null}
        </aside>
      </div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="sstv-field">
      <span>{label}</span>
      {children}
    </label>
  );
}

function Slider({ label, value, min, max, step, onChange, show }: { label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void; show?: string }) {
  return (
    <label className="sstv-field sstv-slider">
      <span>
        {label} <b>{show ?? value}</b>
      </span>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  );
}

function ColorField({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  const t = useT();
  return (
    <div className="sstv-field sstv-color">
      <span>{label}</span>
      <div className="sstv-color-row">
        <input type="color" value={/^#[0-9a-f]{6}$/i.test(value) ? value : '#ffffff'} onChange={(e) => onChange(e.target.value)} aria-label={label} />
        <div className="sstv-swatches" role="group" aria-label={t('sstv.p.palette')}>
          {PALETTES.flat().map((c, i) => (
            <button key={`${c}${i}`} type="button" className={c === value ? 'sstv-swatch sstv-swatch--on' : 'sstv-swatch'} style={{ background: c }} title={c} onClick={() => onChange(c)} />
          ))}
        </div>
      </div>
    </div>
  );
}

function FillField({ fill, onChange }: { fill: Fill; onChange: (f: Fill) => void }) {
  const t = useT();
  return (
    <div className="sstv-group">
      <div className="sstv-seg" role="group" aria-label={t('sstv.p.fill')}>
        <button type="button" className={fill.kind === 'solid' ? 'on' : ''} onClick={() => onChange({ kind: 'solid', color: fill.kind === 'solid' ? fill.color : fill.from })}>
          {t('sstv.p.solid')}
        </button>
        <button type="button" className={fill.kind === 'gradient' ? 'on' : ''} onClick={() => onChange(fill.kind === 'gradient' ? fill : { kind: 'gradient', from: fill.color, to: '#ff3b30', angle: 90 })}>
          {t('sstv.p.gradient')}
        </button>
      </div>
      {fill.kind === 'solid' ? (
        <ColorField label={t('sstv.p.color')} value={fill.color} onChange={(color) => onChange({ kind: 'solid', color })} />
      ) : (
        <>
          <ColorField label={t('sstv.p.from')} value={fill.from} onChange={(from) => onChange({ ...fill, from })} />
          <ColorField label={t('sstv.p.to')} value={fill.to} onChange={(to) => onChange({ ...fill, to })} />
          <Slider label={t('sstv.p.angle')} value={fill.angle} min={0} max={360} step={15} show={`${fill.angle}°`} onChange={(angle) => onChange({ ...fill, angle })} />
        </>
      )}
    </div>
  );
}

function LayerInspector({ layer, patch }: { layer: Layer; patch: (p: Partial<TextLayer> | Partial<ShapeLayer>, key: string) => void }) {
  const t = useT();
  if (layer.kind === 'text') {
    return (
      <div className="sstv-inspector">
        <h3>{t('sstv.layer.text')}</h3>
        <Field label={t('sstv.p.text')}>
          <textarea rows={2} value={layer.text} onChange={(e) => patch({ text: e.target.value }, 'text')} />
        </Field>
        <p className="sstv-hint sstv-tokens">{t('sstv.p.tokens')}</p>
        <Field label={t('sstv.p.font')}>
          <select value={layer.font} onChange={(e) => patch({ font: e.target.value }, 'font')} style={{ fontFamily: layer.font }}>
            {FONTS.map((f) => (
              <option key={f.id} value={f.id} style={{ fontFamily: f.id }}>
                {f.label}
              </option>
            ))}
          </select>
        </Field>
        <Slider label={t('sstv.p.size')} value={Math.round(layer.size * 100)} min={2} max={60} step={1} show={`${Math.round(layer.size * 100)} %`} onChange={(v) => patch({ size: v / 100 }, 'size')} />
        <div className="sstv-seg">
          <button type="button" className={layer.bold ? 'on' : ''} onClick={() => patch({ bold: !layer.bold }, 'bold')}>
            <b>{t('sstv.p.bold')}</b>
          </button>
          <button type="button" className={layer.italic ? 'on' : ''} onClick={() => patch({ italic: !layer.italic }, 'italic')}>
            <i>{t('sstv.p.italic')}</i>
          </button>
        </div>
        <div className="sstv-seg" role="group" aria-label={t('sstv.p.align')}>
          {(['left', 'center', 'right'] as const).map((a) => (
            <button key={a} type="button" className={layer.align === a ? 'on' : ''} onClick={() => patch({ align: a }, 'align')}>
              {t(`sstv.p.${a}`)}
            </button>
          ))}
        </div>
        <h4>{t('sstv.p.fill')}</h4>
        <FillField fill={layer.fill} onChange={(fill) => patch({ fill }, 'fill')} />
        <h4>{t('sstv.p.outline')}</h4>
        <Slider label={t('sstv.p.width')} value={Math.round(layer.outline * 100)} min={0} max={30} step={1} onChange={(v) => patch({ outline: v / 100 }, 'outline')} />
        <ColorField label={t('sstv.p.color')} value={layer.outlineColor} onChange={(outlineColor) => patch({ outlineColor }, 'outlineColor')} />
        <label className="sstv-check">
          <input type="checkbox" checked={layer.shadow} onChange={(e) => patch({ shadow: e.target.checked }, 'shadow')} />
          {t('sstv.p.shadow')}
        </label>
        {layer.shadow ? <ColorField label={t('sstv.p.color')} value={layer.shadowColor} onChange={(shadowColor) => patch({ shadowColor }, 'shadowColor')} /> : null}
        <label className="sstv-check">
          <input type="checkbox" checked={layer.box} onChange={(e) => patch({ box: e.target.checked }, 'box')} />
          {t('sstv.p.box')}
        </label>
        {layer.box ? (
          <>
            <ColorField label={t('sstv.p.color')} value={layer.boxColor} onChange={(boxColor) => patch({ boxColor }, 'boxColor')} />
            <Slider label={t('sstv.p.opacity')} value={Math.round(layer.boxOpacity * 100)} min={0} max={100} step={5} show={`${Math.round(layer.boxOpacity * 100)} %`} onChange={(v) => patch({ boxOpacity: v / 100 }, 'boxOpacity')} />
          </>
        ) : null}
        <Slider label={t('sstv.p.rotation')} value={layer.rotation} min={-180} max={180} step={1} show={`${layer.rotation}°`} onChange={(rotation) => patch({ rotation }, 'rotation')} />
        <p className="sstv-hint">{t('sstv.p.hint')}</p>
      </div>
    );
  }
  return (
    <div className="sstv-inspector">
      <h3>{t(`sstv.layer.${layer.shape}` as Key)}</h3>
      {layer.shape !== 'bars' ? (
        <>
          {layer.shape !== 'frame' ? (
            <label className="sstv-check">
              <input type="checkbox" checked={layer.filled} onChange={(e) => patch({ filled: e.target.checked }, 'filled')} />
              {t('sstv.p.filled')}
            </label>
          ) : null}
          {layer.filled || layer.shape === 'frame' ? <FillField fill={layer.fill} onChange={(fill) => patch({ fill }, 'fill')} /> : null}
          <h4>{t('sstv.p.stroke')}</h4>
          <Slider label={t('sstv.p.width')} value={Math.round(layer.stroke * 200)} min={0} max={30} step={1} onChange={(v) => patch({ stroke: v / 200 }, 'stroke')} />
          <ColorField label={t('sstv.p.color')} value={layer.strokeColor} onChange={(strokeColor) => patch({ strokeColor }, 'strokeColor')} />
        </>
      ) : null}
      <Slider label={t('sstv.p.width')} value={Math.round(layer.w * 100)} min={2} max={200} step={1} show={`${Math.round(layer.w * 100)} %`} onChange={(v) => patch({ w: v / 100 }, 'w')} />
      <Slider label={t('sstv.p.height')} value={Math.round(layer.h * 100)} min={2} max={200} step={1} show={`${Math.round(layer.h * 100)} %`} onChange={(v) => patch({ h: v / 100 }, 'h')} />
      <Slider label={t('sstv.p.opacity')} value={Math.round(layer.opacity * 100)} min={5} max={100} step={5} show={`${Math.round(layer.opacity * 100)} %`} onChange={(v) => patch({ opacity: v / 100 }, 'opacity')} />
      <Slider label={t('sstv.p.rotation')} value={layer.rotation} min={-180} max={180} step={1} show={`${layer.rotation}°`} onChange={(rotation) => patch({ rotation }, 'rotation')} />
      <p className="sstv-hint">{t('sstv.p.hint')}</p>
    </div>
  );
}

function BackgroundInspector({ background, hasPhoto, patch, onPick, onRemove }: { background: Background; hasPhoto: boolean; patch: (p: Partial<Background>, key: string) => void; onPick: () => void; onRemove: () => void }) {
  const t = useT();
  const a = background.adjust;
  return (
    <div className="sstv-inspector">
      <h3>{t('sstv.bg.title')}</h3>
      <button type="button" className="glass sstv-btn sstv-btn--primary sstv-wide" onClick={onPick}>
        {t('sstv.bg.load')}
      </button>
      <p className="sstv-hint">{t('sstv.bg.drop')}</p>
      {hasPhoto ? (
        <>
          <div className="sstv-seg" role="group" aria-label={t('sstv.bg.fit')}>
            <button type="button" className={background.fit === 'cover' ? 'on' : ''} onClick={() => patch({ fit: 'cover' }, 'fit')}>
              {t('sstv.bg.cover')}
            </button>
            <button type="button" className={background.fit === 'contain' ? 'on' : ''} onClick={() => patch({ fit: 'contain' }, 'fit')}>
              {t('sstv.bg.contain')}
            </button>
          </div>
          <Slider label={t('sstv.bg.zoom')} value={Math.round(background.zoom * 100)} min={100} max={600} step={5} show={`${Math.round(background.zoom * 100)} %`} onChange={(v) => patch({ zoom: v / 100 }, 'zoom')} />
          <Slider label={t('sstv.bg.panX')} value={Math.round(background.panX * 100)} min={-100} max={100} step={1} onChange={(v) => patch({ panX: v / 100 }, 'panX')} />
          <Slider label={t('sstv.bg.panY')} value={Math.round(background.panY * 100)} min={-100} max={100} step={1} onChange={(v) => patch({ panY: v / 100 }, 'panY')} />
          <p className="sstv-hint">{t('sstv.bg.panHint')}</p>
          <Slider label={t('sstv.bg.brightness')} value={a.brightness} min={-100} max={100} step={1} onChange={(v) => patch({ adjust: { ...a, brightness: v } }, 'brightness')} />
          <Slider label={t('sstv.bg.contrast')} value={a.contrast} min={-100} max={100} step={1} onChange={(v) => patch({ adjust: { ...a, contrast: v } }, 'contrast')} />
          <Slider label={t('sstv.bg.saturation')} value={a.saturation} min={-100} max={100} step={1} onChange={(v) => patch({ adjust: { ...a, saturation: v } }, 'saturation')} />
          <div className="sstv-add">
            <button type="button" className="glass sstv-btn" onClick={() => patch({ adjust: { ...NEUTRAL_ADJUST } }, 'reset')}>
              {t('sstv.bg.reset')}
            </button>
            <button type="button" className="glass sstv-btn sstv-btn--danger" onClick={onRemove}>
              {t('sstv.bg.remove')}
            </button>
          </div>
        </>
      ) : null}
      <ColorField label={t('sstv.bg.color')} value={background.color} onChange={(color) => patch({ color }, 'color')} />
    </div>
  );
}
