import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react';
import { useT } from '../i18n';
import { hzAt, leftPercent } from '../lib/markers';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  /** Frequenze di ricezione e trasmissione che riporta il motore (Hz); assenti = nessun segnaposto. */
  rxHz?: number | null;
  txHz?: number | null;
  /** Larghezza del segnale del modo in uso (Hz): le barre la occupano a destra della frequenza. 0 o assente = barra sottile. */
  bandwidthHz?: number;
  /** Con il Filtro acceso: Hz sotto e sopra Rx della finestra decodificata (asimmetrica come in JTDX); fuori la cascata si scurisce. */
  filter?: { below: number; above: number } | null;
  /** «Bars» del Wide Graph: le linee verticali Rx e Tx e le due linee che seguono il puntatore. Spento restano solo le etichette. */
  showBars?: boolean;
  /** «Freq» del Wide Graph: la frequenza sotto il puntatore. */
  showFreq?: boolean;
  onRx(hz: number): void;
  onTx(hz: number): void;
}

const STEP = 10;
const BIG_STEP = 50;

/**
 * Le due barre del motore sopra la cascata: verde = frequenza di ricezione, rossa = trasmissione, larghe quanto il segnale del modo.
 * Si trascinano; un clic sinistro sulla cascata sposta la verde, il tasto destro (o Maiusc+clic) la rossa, anche durante la trasmissione.
 */
/** Ogni quanto si manda la frequenza al motore mentre si trascina (ms): la barra si muove subito, i comandi non si accavallano. */
const SEND_EVERY_MS = 80;
/** Dopo il rilascio la barra resta dove l'ha lasciata l'operatore finche' il motore non conferma (al massimo questo tempo). */
const HOLD_MS = 1500;

export function FrequencyMarkers({ fMinHz, fMaxHz, rxHz: rxEngine, txHz: txEngine, bandwidthHz = 0, filter = null, showBars = true, showFreq = false, onRx: sendRx, onTx: sendTx }: Props) {
  const t = useT();
  // posizione mostrata subito durante il trascinamento: non si aspetta la risposta del motore a ogni movimento (prima la barra andava a scatti)
  const [local, setLocal] = useState<{ rx?: number; tx?: number }>({});
  const pending = useRef<{ rx?: number; tx?: number }>({});
  const timers = useRef<{ rx?: ReturnType<typeof setTimeout>; tx?: ReturnType<typeof setTimeout> }>({});
  const lastSent = useRef<{ rx: number; tx: number }>({ rx: 0, tx: 0 });
  const holdUntil = useRef<{ rx: number; tx: number }>({ rx: 0, tx: 0 });

  const throttled = (kind: 'rx' | 'tx', send: (hz: number) => void) => (hz: number) => {
    const rounded = Math.round(hz);
    setLocal((l) => ({ ...l, [kind]: rounded }));
    holdUntil.current[kind] = Date.now() + HOLD_MS;
    pending.current[kind] = rounded;
    const flush = () => {
      timers.current[kind] = undefined;
      const value = pending.current[kind];
      if (value === undefined) return;
      pending.current[kind] = undefined;
      lastSent.current[kind] = Date.now();
      send(value);
    };
    const wait = SEND_EVERY_MS - (Date.now() - lastSent.current[kind]);
    if (wait <= 0) flush();
    else if (timers.current[kind] === undefined) timers.current[kind] = setTimeout(flush, wait);
  };
  const onRx = throttled('rx', sendRx);
  const onTx = throttled('tx', sendTx);

  // quando il motore conferma (o passa il tempo di attesa) torna a valere il valore del motore
  useEffect(() => {
    const id = setInterval(() => {
      setLocal((l) => {
        const now = Date.now();
        const next = { ...l };
        if (l.rx !== undefined && (l.rx === rxEngine || now > holdUntil.current.rx) && timers.current.rx === undefined) next.rx = undefined;
        if (l.tx !== undefined && (l.tx === txEngine || now > holdUntil.current.tx) && timers.current.tx === undefined) next.tx = undefined;
        return next.rx === l.rx && next.tx === l.tx ? l : next;
      });
    }, 100);
    return () => clearInterval(id);
  }, [rxEngine, txEngine]);
  useEffect(() => () => {
    clearTimeout(timers.current.rx);
    clearTimeout(timers.current.tx);
  }, []);
  const rxHz = local.rx ?? rxEngine;
  const txHz = local.tx ?? txEngine;
  const box = useRef<HTMLDivElement | null>(null);
  // posizione del puntatore sulla cascata (0-1): le linee bianche di JTDX (`HoverOverlayPixmap`) e il suggerimento «Freq»
  const [hover, setHover] = useState<number | null>(null);

  const hzFromPointer = (clientX: number): number => {
    const r = box.current?.getBoundingClientRect();
    if (!r || r.width <= 0) return fMinHz;
    return hzAt((clientX - r.left) / r.width, fMinHz, fMaxHz);
  };

  // la barra che si sta trascinando: la sua etichetta diventa piena del suo colore per leggere bene gli Hz, al rilascio torna trasparente
  const [dragging, setDragging] = useState<'rx' | 'tx' | null>(null);

  const drag = (apply: (hz: number) => void, kind: 'rx' | 'tx', anyButton = false) => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0 && !(anyButton && e.button === 2)) return;
    e.stopPropagation();
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    setDragging(kind);
    const move = (ev: PointerEvent) => apply(hzFromPointer(ev.clientX));
    const stop = () => {
      setDragging(null);
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  const keys = (hz: number, apply: (hz: number) => void) => (e: KeyboardEvent<HTMLElement>) => {
    const step = e.shiftKey ? BIG_STEP : STEP;
    if (e.key === 'ArrowRight') apply(Math.min(fMaxHz, hz + step));
    else if (e.key === 'ArrowLeft') apply(Math.max(fMinHz, hz - step));
    else return;
    e.preventDefault();
  };

  const marker = (kind: 'rx' | 'tx', hz: number | null | undefined, apply: (hz: number) => void, name: string) =>
    hz == null ? null : (
      <div
        key={kind}
        className={`wf-marker wf-marker--${kind}${bandwidthHz > 0 ? ' wf-marker--band' : ''}${showBars ? '' : ' wf-marker--nobars'}${dragging === kind ? ' wf-marker--dragging' : ''}`}
        style={{
          left: `${leftPercent(hz, fMinHz, fMaxHz)}%`,
          ...(bandwidthHz > 0 ? { width: `${(bandwidthHz / (fMaxHz - fMinHz)) * 100}%` } : {}),
        }}
        role="slider"
        tabIndex={0}
        aria-label={name}
        title={kind === 'rx' ? t('tip.marker.rx') : t('tip.marker.tx')}
        aria-valuemin={fMinHz}
        aria-valuemax={fMaxHz}
        aria-valuenow={hz}
        onPointerDown={drag(apply, kind)}
        onKeyDown={keys(hz, apply)}
      >
        <span className="wf-marker-tag">{`${kind === 'rx' ? 'Rx' : 'Tx'} ${hz}`}</span>
        <span className="wf-marker-line" />
      </div>
    );

  const hoverLeft = hover === null ? null : hover * 100;
  const hoverWidth = bandwidthHz > 0 ? (bandwidthHz / (fMaxHz - fMinHz)) * 100 : 0;
  return (
    <div ref={box} className="wf-markers">
      <div
        className="wf-pad"
        onPointerMove={(e) => {
          const r = box.current?.getBoundingClientRect();
          if (r && r.width > 0 && (showBars || showFreq)) setHover(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)));
        }}
        onPointerLeave={() => setHover(null)}
        onPointerDown={(e) => {
          // come JTDX: tasto sinistro = Rx, tasto destro (o Maiusc+clic) = Tx; tenendo premuto e trascinando la barra segue il puntatore.
          // Vale anche mentre si trasmette: il motore sposta subito il tono (con «cambi di frequenza Tx durante la trasmissione» acceso)
          if (e.button !== 0 && e.button !== 2) return;
          const apply = e.button === 2 || e.shiftKey ? onTx : onRx;
          apply(hzFromPointer(e.clientX));
          drag(apply, apply === onTx ? 'tx' : 'rx', true)(e);
        }}
        onContextMenu={(e) => e.preventDefault()}
      />
      {filter != null && rxHz != null ? (
        <>
          <div className="wf-filter-shade" aria-hidden="true" style={{ left: 0, width: `${leftPercent(Math.max(fMinHz, rxHz - filter.below), fMinHz, fMaxHz)}%` }} />
          <div className="wf-filter-shade" aria-hidden="true" style={{ left: `${leftPercent(Math.min(fMaxHz, rxHz + filter.above), fMinHz, fMaxHz)}%`, right: 0 }} />
        </>
      ) : null}
      {showBars && hoverLeft !== null ? (
        <>
          <div className="wf-hover-line" aria-hidden="true" style={{ left: `${hoverLeft}%` }} />
          {hoverWidth > 0 ? <div className="wf-hover-line" aria-hidden="true" style={{ left: `${Math.min(100, hoverLeft + hoverWidth)}%` }} /> : null}
        </>
      ) : null}
      {showFreq && hoverLeft !== null ? (
        <div className="wf-hover-freq" aria-hidden="true" style={{ left: `${hoverLeft}%` }}>
          {hzAt(hover ?? 0, fMinHz, fMaxHz)}
        </div>
      ) : null}
      {marker('rx', rxHz, onRx, t('marker.rx'))}
      {marker('tx', txHz, onTx, t('marker.tx'))}
    </div>
  );
}
