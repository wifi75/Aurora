import { useEffect, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { registerSpectrum } from '../../lib/spectrumFeed';
import type { SpectrumRow } from '../../lib/digiSpectrum';

interface Props {
  /** Il tratto di frequenza audio mostrato (Hz). */
  minHz: number;
  maxHz: number;
  /** La frequenza di ricezione (bandierina verde) e, se e' diversa (Lock), quella di trasmissione (rossa). */
  rxHz: number;
  txHz: number | null;
  /** La distanza fra i due triangoli di ogni coppia (Hz): lo shift dell'RTTY (mark e space) o la larghezza del segnale che si decodifica. */
  bandwidthHz: number;
  /** Split acceso: la coppia rossa della trasmissione sta altrove e il clic la sposta; con Ctrl si sposta la ricezione. */
  split?: boolean;
  /** «Show hairlines»: due righe sottili su tutta la cascata alle frequenze di ricezione (e di trasmissione in split). */
  hairlines?: boolean;
  /** In ricezione la bandierina e' verde, in trasmissione rossa. */
  transmitting: boolean;
  height?: number;
  /** Ogni riga che arriva (per Snap, Seek e Scan). */
  onRow?(row: SpectrumRow): void;
  /** Un clic sulla cascata (`ctrl`: con il tasto Ctrl premuto). */
  onPick(hz: number, ctrl: boolean, commit?: boolean): void;
  /** Frequenza della radio (Hz): la scala mostra le frequenze vere in kHz (radio + audio, in USB); 0 = scala in Hz dell'audio. */
  dialHz?: number;
  /** Cambia l'altezza della cascata (si trascina la barra sul bordo alto). */
  onHeight?(px: number): void;
}

/** 0..255 -> colore: blu scuro, azzurro, verde, giallo, rosso. */
function paletteColor(v: number): [number, number, number] {
  const x = v / 255;
  if (x < 0.25) return [0, 0, Math.round(60 + x * 4 * 150)];
  if (x < 0.5) return [0, Math.round((x - 0.25) * 4 * 255), 210];
  if (x < 0.75) return [Math.round((x - 0.5) * 4 * 255), 255, Math.round(210 * (1 - (x - 0.5) * 4))];
  return [255, Math.round(255 * (1 - (x - 0.75) * 4)), 0];
}

const PALETTE = Array.from({ length: 256 }, (_, i) => paletteColor(i));
const SCALE_H = 22;

/** La cascata della pagina dei modi testuali: scala in Hz in alto, righe che scorrono, bandierine e clic per sintonizzare. */
export function DigiWaterfall({ minHz, maxHz, rxHz, txHz, bandwidthHz, split = false, hairlines = false, dialHz = 0, transmitting, height = 150, onRow, onPick, onHeight }: Props) {
  const t = useT();
  const wrap = useRef<HTMLDivElement>(null);
  const flow = useRef<HTMLCanvasElement>(null);
  const scale = useRef<HTMLCanvasElement>(null);
  const [width, setWidth] = useState(800);
  // spostamento fluido: mentre si trascina i marcatori seguono il mouse subito (senza aspettare il motore); il valore vero torna dopo
  const [over, setOver] = useState<{ which: 'rx' | 'tx'; hz: number } | null>(null);
  const dragging = useRef(false);
  const lastSent = useRef(0);
  useEffect(() => {
    if (over === null || dragging.current) return;
    const timer = window.setTimeout(() => setOver(null), 700);
    return () => window.clearTimeout(timer);
  }, [over]);
  const view = useRef({ minHz, maxHz });
  view.current = { minHz, maxHz };
  const rowCb = useRef(onRow);
  rowCb.current = onRow;

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(200, Math.floor(el.clientWidth))));
    ro.observe(el);
    setWidth(Math.max(200, Math.floor(el.clientWidth)));
    return () => ro.disconnect();
  }, []);

  // la scala: un tacco ogni 100 Hz, le cifre ogni 500
  useEffect(() => {
    const c = scale.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    c.width = width;
    c.height = SCALE_H;
    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, width, SCALE_H);
    ctx.fillStyle = '#fff';
    ctx.strokeStyle = '#fff';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    const hzToX = (hz: number) => ((hz - minHz) / (maxHz - minHz)) * width;
    if (dialHz > 0) {
      // frequenze vere: un tacco ogni 100 Hz, le cifre ogni kHz (14022, 14023...); radio + audio, in USB
      const first = Math.ceil((dialHz + minHz) / 100) * 100;
      for (let real = first; real <= dialHz + maxHz; real += 100) {
        const x = Math.round(hzToX(real - dialHz)) + 0.5;
        const major = real % 1000 === 0;
        ctx.beginPath();
        ctx.moveTo(x, SCALE_H);
        ctx.lineTo(x, SCALE_H - (major ? 9 : 4));
        ctx.stroke();
        if (major) ctx.fillText(String(real / 1000), x, 11);
      }
      return;
    }
    for (let hz = Math.ceil(minHz / 100) * 100; hz <= maxHz; hz += 100) {
      const x = Math.round(hzToX(hz)) + 0.5;
      const major = hz % 500 === 0;
      ctx.beginPath();
      ctx.moveTo(x, SCALE_H);
      ctx.lineTo(x, SCALE_H - (major ? 9 : 4));
      ctx.stroke();
      if (major) ctx.fillText(String(hz), x, 11);
    }
  }, [width, minHz, maxHz, dialHz]);

  useEffect(() => {
    const c = flow.current;
    if (!c) return;
    c.width = width;
    c.height = height - SCALE_H;
    const ctx = c.getContext('2d');
    if (ctx) {
      ctx.fillStyle = '#000040';
      ctx.fillRect(0, 0, c.width, c.height);
    }
  }, [width, height]);

  useEffect(
    () =>
      registerSpectrum((bins, fMinHz, fMaxHz) => {
        rowCb.current?.({ bins, fMinHz, fMaxHz });
        const c = flow.current;
        const ctx = c?.getContext('2d');
        if (!c || !ctx || c.width < 2) return;
        // tutto scende di una riga, la nuova va in cima
        ctx.drawImage(c, 0, 0, c.width, c.height - 1, 0, 1, c.width, c.height - 1);
        const img = ctx.createImageData(c.width, 1);
        const { minHz: a, maxHz: b } = view.current;
        const hzPerBin = (fMaxHz - fMinHz) / Math.max(1, bins.length);
        for (let x = 0; x < c.width; x++) {
          const hz = a + (x / c.width) * (b - a);
          const v = bins[Math.min(bins.length - 1, Math.max(0, Math.floor((hz - fMinHz) / hzPerBin)))] ?? 0;
          const [r, g, bl] = PALETTE[v] ?? [0, 0, 0];
          const o = x * 4;
          img.data[o] = r;
          img.data[o + 1] = g;
          img.data[o + 2] = bl;
          img.data[o + 3] = 255;
        }
        ctx.putImageData(img, 0, 0);
      }),
    [],
  );

  const xOf = (hz: number) => ((hz - minHz) / (maxHz - minHz)) * width;
  const rxShown = over?.which === 'rx' ? over.hz : rxHz;
  const txShown = split ? (over?.which === 'tx' ? over.hz : (txHz ?? rxShown)) : rxShown;

  // la bandierina sta sul centro della ricezione: verde in ricezione, rossa in trasmissione
  const flag = (hz: number) =>
    hz >= minHz && hz <= maxHz ? (
      <span key="flag" className="digi-wf-flag" style={{ left: xOf(hz) }} title={t('digi.wf.rxTip')}>
        <i style={{ background: transmitting ? '#ff3b3b' : '#22c55e' }} />
      </span>
    ) : null;

  // la coppia di marcatori uniti da una linea, ai due lati della frequenza (mark e space dell'RTTY): il triangolo blu (ricezione) sta sotto
  // la linea e il rosso (trasmissione) sopra, cosi' quando coincidono formano un rombo; in split la coppia rossa va per conto suo
  const pair = (hz: number, kind: 'rx' | 'tx', title: string) => {
    // senza coppia di toni (PSK, CW) si mostra un solo rombo sulla frequenza: la banda passante non si disegna
    const single = bandwidthHz <= 0;
    const left = xOf(hz - (single ? 0 : bandwidthHz / 2));
    const right = xOf(hz + (single ? 0 : bandwidthHz / 2));
    if (right < 0 || left > width) return null;
    return <span key={kind} className={`digi-wf-pair digi-wf-pair--${kind}${single ? ' digi-wf-pair--single' : ''}`} style={{ left, width: single ? 0 : Math.max(8, right - left) }} title={title} aria-hidden="true" />;
  };

  const hzAt = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    return Math.round(Math.min(maxHz, Math.max(minHz, minHz + ((e.clientX - r.left) / r.width) * (maxHz - minHz))) * 10) / 10;
  };

  return (
    <div className="digi-wf" ref={wrap} style={{ height }}>
      {onHeight ? (
        <div
          className="digi-wf-grip"
          role="separator"
          aria-orientation="horizontal"
          title={t('digi.wf.resize')}
          onPointerDown={(e) => {
            e.preventDefault();
            const startY = e.clientY;
            const startH = height;
            const move = (m: PointerEvent) => onHeight(startH + (startY - m.clientY));
            const up = () => {
              window.removeEventListener('pointermove', move);
              window.removeEventListener('pointerup', up);
            };
            window.addEventListener('pointermove', move);
            window.addEventListener('pointerup', up);
          }}
        />
      ) : null}
      <canvas ref={scale} className="digi-wf-scale" />
      <canvas
        ref={flow}
        className="digi-wf-flow"
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          dragging.current = true;
          const hz = hzAt(e);
          setOver({ which: split && !e.ctrlKey ? 'tx' : 'rx', hz });
          lastSent.current = Date.now();
          onPick(hz, e.ctrlKey, false);
        }}
        onPointerMove={(e) => {
          if (!dragging.current) return;
          const hz = hzAt(e);
          setOver({ which: split && !e.ctrlKey ? 'tx' : 'rx', hz });
          // al motore al massimo dieci volte al secondo: il marcatore segue il mouse lo stesso
          if (Date.now() - lastSent.current >= 100) {
            lastSent.current = Date.now();
            onPick(hz, e.ctrlKey, false);
          }
        }}
        onPointerUp={(e) => {
          if (!dragging.current) return;
          dragging.current = false;
          const hz = hzAt(e);
          setOver({ which: split && !e.ctrlKey ? 'tx' : 'rx', hz });
          onPick(hz, e.ctrlKey, true);
        }}
        onPointerCancel={() => {
          dragging.current = false;
          setOver(null);
        }}
      />
      {flag(rxShown)}
      {hairlines && rxShown >= minHz && rxShown <= maxHz ? <span className="digi-wf-hair digi-wf-hair--rx" style={{ left: xOf(rxShown) }} aria-hidden="true" /> : null}
      {hairlines && split && txShown >= minHz && txShown <= maxHz ? <span className="digi-wf-hair digi-wf-hair--tx" style={{ left: xOf(txShown) }} aria-hidden="true" /> : null}
      {pair(rxShown, 'rx', t('digi.wf.rxTip'))}
      {pair(txShown, 'tx', t('digi.wf.txTip'))}
    </div>
  );
}
