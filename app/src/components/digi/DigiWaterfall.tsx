import { useEffect, useMemo, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { registerSpectrum } from '../../lib/spectrumFeed';
import { digiWfTable, useDigiWf } from '../../lib/digiWfPrefs';
import type { SpectrumRow } from '../../lib/digiSpectrum';
import type { DigiNotch } from '../../api/client';
import { addNotch } from '../../lib/digiNotches';
import { WaterfallNotches } from './WaterfallNotches';
import { PileupMarks } from './PileupMarks';
import type { WaterfallLabel } from '../../lib/digiBand';

interface Props {
  /** Il tratto di frequenza audio mostrato (Hz). */
  minHz: number;
  maxHz: number;
  /** La frequenza di ricezione (bandierina verde) e, se e' diversa (Lock), quella di trasmissione (rossa). */
  rxHz: number;
  txHz: number | null;
  /** La distanza fra i due triangoli di ogni coppia (Hz): lo shift dell'RTTY (mark e space) o la larghezza del segnale che si decodifica. */
  bandwidthHz: number;
  /** Il filtro BPF acceso: la banda che lascia passare al decodificatore (Hz, centrata sulla ricezione); 0 = spento. */
  passbandHz?: number;
  /** CW: la larghezza del filtro del decodificatore (Hz) come banda sfumata attorno al cursore, senza bordi ne' scritte; 0 = niente. */
  shadeHz?: number;
  /** Split acceso: la coppia rossa della trasmissione sta altrove e il clic la sposta; con Maiusc si sposta la ricezione. */
  split?: boolean;
  /** «Show hairlines»: due righe sottili su tutta la cascata alle frequenze di ricezione (e di trasmissione in split). */
  hairlines?: boolean;
  /** In ricezione la bandierina e' verde, in trasmissione rossa. */
  transmitting: boolean;
  height?: number;
  /** Ogni riga che arriva (per Snap, Seek e Scan). */
  onRow?(row: SpectrumRow): void;
  /** Un clic sulla cascata (`ctrl`: con il tasto Maiusc premuto, in split sposta la ricezione; il nome resta per i chiamanti). */
  onPick(hz: number, ctrl: boolean, commit?: boolean): void;
  /** I notch della ricezione (strisce sulla cascata). */
  notches?: DigiNotch[];
  /** Notch cambiati: Ctrl+clic ne aggiunge uno, si trascinano, il tasto destro li toglie. Senza, Ctrl+clic non fa niente di speciale. */
  onNotches?(list: DigiNotch[]): void;
  /** Frequenza della radio (Hz): la scala mostra le frequenze vere in kHz (radio + audio, in USB); 0 = scala in Hz dell'audio. */
  dialHz?: number;
  /** Cambia l'altezza della cascata (si trascina la barra sul bordo alto). */
  onHeight?(px: number): void;
  /** Larghezza scelta con la maniglia sul bordo destro (null o assente: tutta la colonna); con `onWidth` la maniglia si mostra. */
  widthPx?: number | null;
  onWidth?(px: number | null): void;
  /** I nominativi dei segnali di tutta la banda, scritti sopra la loro traccia (`cq`: chiama CQ; `sure`: verificato). */
  labels?: WaterfallLabel[];
  /** «Testo sulla cascata»: sotto ogni nominativo le ultime parole del suo canale (di base si'). */
  showWords?: boolean;
  /** Le finestre RX in piu': un indicatore numerato per ognuna, che si trascina. */
  extraRx?: { id: number; freq_hz: number }[];
  onExtraMove?(id: number, hz: number, commit: boolean): void;
  /** Tasto destro sulla cascata: la frequenza sotto il mouse e il punto dello schermo (per il menu). */
  onMenu?(hz: number, x: number, y: number): void;
  /** I segni del modo DXpedition (di base si'; la pagina WEFAX li spegne). */
  pileup?: boolean;
  /** Nasconde la scala in Hz sopra la cascata (la pagina WEFAX non ne ha bisogno): l'altezza `height` resta quella totale, compresa la scala. */
  hideScale?: boolean;
}

const SCALE_H = 22;

/** La cascata della pagina dei modi testuali: scala in Hz in alto, righe che scorrono, bandierine e clic per sintonizzare. */
export function DigiWaterfall({ minHz, maxHz, rxHz, txHz, bandwidthHz, passbandHz = 0, shadeHz = 0, split = false, hairlines = false, dialHz = 0, transmitting, height = 150, onRow, onPick, onHeight, widthPx = null, onWidth, labels = [], showWords = true, notches = [], onNotches, extraRx = [], onExtraMove, onMenu, pileup = true, hideScale = false }: Props) {
  const t = useT();
  const wrap = useRef<HTMLDivElement>(null);
  const flow = useRef<HTMLCanvasElement>(null);
  // i colori scelti in Impostazioni › Cascata («Classica», guadagno 0 e zero 0 sono quelli di sempre); la tabella si rifa solo se cambiano
  const wfPrefs = useDigiWf((s) => s.prefs);
  const table = useMemo(() => digiWfTable(wfPrefs), [wfPrefs]);
  const tableRef = useRef(table);
  tableRef.current = table;
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
    c.height = height - (hideScale ? 0 : SCALE_H);
    const ctx = c.getContext('2d');
    if (ctx) {
      ctx.fillStyle = '#000040';
      ctx.fillRect(0, 0, c.width, c.height);
    }
  }, [width, height, hideScale]);

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
          const [r, g, bl] = tableRef.current[v] ?? [0, 0, 0];
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

  // la coppia di marcatori uniti da una linea, ai due lati della frequenza (mark e space dell'RTTY): il triangolo viola scuro (ricezione) sta sotto
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
    <div className="digi-wf" ref={wrap} style={widthPx ? { height, width: widthPx, maxWidth: '100%' } : { height }}>
      {onWidth ? (
        <div
          className="digi-wf-wgrip"
          role="separator"
          aria-orientation="vertical"
          title={t('digi.wf.resizeW')}
          onDoubleClick={() => onWidth(null)}
          onPointerDown={(e) => {
            const el = wrap.current;
            const col = el?.parentElement;
            if (!el || !col) return;
            e.preventDefault();
            // con lo zoom dell'interfaccia le misure a schermo e quelle CSS differiscono: si riporta lo spostamento in pixel CSS
            const scale = el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : 1;
            const startX = e.clientX;
            const startW = el.offsetWidth;
            const full = col.clientWidth;
            const move = (m: PointerEvent) => {
              const w = Math.round(Math.min(full, Math.max(240, startW + (m.clientX - startX) / scale)));
              onWidth(w >= full - 4 ? null : w);
            };
            const up = () => {
              window.removeEventListener('pointermove', move);
              window.removeEventListener('pointerup', up);
            };
            window.addEventListener('pointermove', move);
            window.addEventListener('pointerup', up);
          }}
        />
      ) : null}
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
      <canvas ref={scale} className="digi-wf-scale" style={hideScale ? { display: 'none' } : undefined} />
      <canvas
        ref={flow}
        className="digi-wf-flow"
        title={onNotches ? t('digi.notch.addTip') : undefined}
        onPointerDown={(e) => {
          // Ctrl+clic: un notch nel punto (la frequenza di ricezione non si muove)
          if (e.ctrlKey && onNotches) {
            if (e.button === 0) onNotches(addNotch(notches, hzAt(e)));
            return;
          }
          if (e.button !== 0) return;
          e.currentTarget.setPointerCapture?.(e.pointerId);
          dragging.current = true;
          const hz = hzAt(e);
          setOver({ which: split && !e.shiftKey ? 'tx' : 'rx', hz });
          lastSent.current = Date.now();
          onPick(hz, e.shiftKey, false);
        }}
        onPointerMove={(e) => {
          if (!dragging.current) return;
          const hz = hzAt(e);
          setOver({ which: split && !e.shiftKey ? 'tx' : 'rx', hz });
          // al motore al massimo dieci volte al secondo: il marcatore segue il mouse lo stesso
          if (Date.now() - lastSent.current >= 100) {
            lastSent.current = Date.now();
            onPick(hz, e.shiftKey, false);
          }
        }}
        onPointerUp={(e) => {
          if (!dragging.current) return;
          dragging.current = false;
          const hz = hzAt(e);
          setOver({ which: split && !e.shiftKey ? 'tx' : 'rx', hz });
          onPick(hz, e.shiftKey, true);
        }}
        onPointerCancel={() => {
          dragging.current = false;
          setOver(null);
        }}
        onContextMenu={(e) => {
          if (!onMenu) return;
          e.preventDefault();
          const r = e.currentTarget.getBoundingClientRect();
          const hz = Math.round(Math.min(maxHz, Math.max(minHz, minHz + ((e.clientX - r.left) / r.width) * (maxHz - minHz))));
          onMenu(hz, e.clientX, e.clientY);
        }}
      />
      {extraRx
        .filter((x) => x.freq_hz >= minHz && x.freq_hz <= maxHz)
        .map((x) => (
          <span
            key={`xrx-${x.id}`}
            className={`digi-wf-xrx${bandwidthHz > 0 ? '' : ' digi-wf-xrx--single'}`}
            style={
              bandwidthHz > 0
                ? { left: xOf(x.freq_hz - bandwidthHz / 2), width: Math.max(16, xOf(x.freq_hz + bandwidthHz / 2) - xOf(x.freq_hz - bandwidthHz / 2)) }
                : { left: xOf(x.freq_hz) }
            }
            title={t('xrx.markerTip', { n: x.id })}
            onPointerDown={(e) => {
              if (e.button !== 0 || !onExtraMove) return;
              e.preventDefault();
              e.stopPropagation();
              const box = e.currentTarget.parentElement?.getBoundingClientRect();
              if (!box) return;
              const at = (cx: number) => Math.round(Math.min(maxHz, Math.max(minHz, minHz + ((cx - box.left) / box.width) * (maxHz - minHz))));
              let last = x.freq_hz;
              const move = (m: PointerEvent) => {
                last = at(m.clientX);
                onExtraMove(x.id, last, false);
              };
              const up = () => {
                window.removeEventListener('pointermove', move);
                window.removeEventListener('pointerup', up);
                onExtraMove(x.id, last, true);
              };
              window.addEventListener('pointermove', move);
              window.addEventListener('pointerup', up);
            }}
          >
            <b>{x.id}</b>
          </span>
        ))}
      {onNotches ? <WaterfallNotches notches={notches} minHz={minHz} maxHz={maxHz} width={width} top={SCALE_H} onChange={onNotches} /> : null}
      {passbandHz > 0 ? (
        <>
          {/* il filtro BPF acceso: fuori dalla banda che passa al decodificatore la cascata si scurisce, la banda resta chiara con i bordi gialli */}
          <span className="digi-wf-bpf-out" style={{ left: 0, width: Math.max(0, xOf(rxShown - passbandHz / 2)), top: SCALE_H }} aria-hidden="true" />
          <span className="digi-wf-bpf-out" style={{ left: xOf(rxShown + passbandHz / 2), right: 0, top: SCALE_H }} aria-hidden="true" />
          <span
            className="digi-wf-bpf"
            style={{ left: xOf(rxShown - passbandHz / 2), width: Math.max(4, xOf(rxShown + passbandHz / 2) - xOf(rxShown - passbandHz / 2)), top: SCALE_H }}
            title={t('digi.st.passband')}
            aria-hidden="true"
          >
            <b>BPF</b>
          </span>
        </>
      ) : null}
      {shadeHz > 0 ? (
        <span
          className="digi-wf-shade"
          style={{ left: xOf(rxShown - shadeHz / 2), width: Math.max(6, xOf(rxShown + shadeHz / 2) - xOf(rxShown - shadeHz / 2)), top: SCALE_H }}
          aria-hidden="true"
        />
      ) : null}
      {flag(rxShown)}
      {hairlines && rxShown >= minHz && rxShown <= maxHz ? <span className="digi-wf-hair digi-wf-hair--rx" style={{ left: xOf(rxShown) }} aria-hidden="true" /> : null}
      {hairlines && split && txShown >= minHz && txShown <= maxHz ? <span className="digi-wf-hair digi-wf-hair--tx" style={{ left: xOf(txShown) }} aria-hidden="true" /> : null}
      {pileup ? <PileupMarks minHz={minHz} maxHz={maxHz} top={SCALE_H} xOf={xOf} /> : null}
      {pair(rxShown, 'rx', t('digi.wf.rxTip'))}
      {pair(txShown, 'tx', t('digi.wf.txTip'))}
      {/* i nominativi di tutta la banda appena sotto la scala, sopra la traccia (la riga nuova della cascata e' in cima); due vicini si alternano in altezza */}
      {labels
        .filter((l) => l.hz >= minHz && l.hz <= maxHz)
        .sort((a, b) => a.hz - b.hz)
        .map((l, i, all) => {
          const prev = all[i - 1];
          const low = prev !== undefined && xOf(l.hz) - xOf(prev.hz) < (showWords ? 150 : 70) && i % 2 === 1;
          const x = xOf(l.hz);
          // la riga delle parole non invade le vicine: larga quanto lo spazio fino all'etichetta piu' vicina della stessa altezza
          const sameRow = [all[i - 2], all[i + 2], ...(low ? [] : [all[i - 1], all[i + 1]].filter((n) => n !== undefined && Math.abs(xOf(n.hz) - x) >= (showWords ? 150 : 70)))];
          const gap = Math.min(...sameRow.filter((n) => n !== undefined).map((n) => Math.abs(xOf(n.hz) - x)), Infinity);
          const maxW = Math.max(60, Math.min(210, gap - 8, 2 * Math.min(x, width - x) - 4));
          const words = showWords && l.words ? l.words : null;
          const tip = l.history ? `${l.text}
${l.history}` : l.text;
          return (
            <span key={`${l.text}-${Math.round(l.hz)}`} className={`digi-wf-label${l.quiet ? ' digi-wf-label--quiet' : ''}${l.rbn ? ' digi-wf-label--rbn' : ''}`} style={{ left: x, top: SCALE_H + (low ? (showWords ? 52 : 42) : 22) }} title={tip} onPointerDown={(e) => {
              if (e.button !== 0) return;
              e.stopPropagation();
              onPick(Math.round(l.hz * 10) / 10, e.shiftKey, true);
            }}>
              <span className={`digi-wf-call${l.cq ? ' digi-wf-call--cq' : ''}${l.sure ? '' : ' digi-wf-call--maybe'}${l.color !== undefined ? ` digi-wf-call--station radar-c${l.color}` : ''}`}>{l.rbn ? <span className="digi-wf-rbn">RBN</span> : null}{l.text}</span>
              {words ? (
                <span className="digi-wf-words" style={{ maxWidth: maxW }}>
                  {words.map((w, k) => (
                    <span key={k} className={`digi-wf-w--${w.kind}${w.unsure ? ' digi-wf-w--unsure' : ''}`}>{w.text}</span>
                  ))}
                </span>
              ) : null}
            </span>
          );
        })}
    </div>
  );
}
