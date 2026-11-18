import '../../styles/modern-displays.css';
import type { ReactNode } from 'react';
import type { CatExtraItem } from '../../api/client';
import {
  formatRadioFrequency,
  hasFrequency,
  powerFraction,
  sFraction,
  sIsEstimated,
  sLabel,
  swrFraction,
  type DisplayModel,
} from '../../lib/radioDisplay';
import { actionTip } from './RadioExtras';
import type { FaceExtras } from './RigFaceDisplays';
import { useT } from '../../i18n';

/**
 * Sei display «moderni»: tre estesi (640 x 90, nella riga dei comandi) e tre quasi quadrati (330 x 130, nell'angolo della testata
 * al posto delle icone). Mostrano solo dati e funzioni che esistono davvero: frequenza (VFO A e, se la radio lo da', VFO B), modo, TX,
 * S-meter in ricezione, potenza e ROS in trasmissione, e il tasto TUNE (con il clic, solo in testata, accorda).
 */

export interface ModernProps {
  model: DisplayModel;
  noRadio: string;
  ariaLabel: string;
  data?: FaceExtras | null;
  onAction?: (item: CatExtraItem) => void;
  waiting?: CatExtraItem | null;
}

const pct = (f: number | null) => `${Math.round(Math.min(1, Math.max(0, f ?? 0)) * 100)}%`;

/** Un indicatore: pulsante che comanda la radio in testata, scritta nelle anteprime. */
function Chip({ item, on, children, onAction, waiting, cls = '' }: { item?: CatExtraItem; on?: boolean; children: ReactNode; onAction?: (i: CatExtraItem) => void; waiting?: CatExtraItem | null; cls?: string }) {
  const t = useT();
  const className = `md-chip${on ? ' md-chip--on' : ''}${cls ? ` ${cls}` : ''}${item && waiting === item ? ' md-chip--wait' : ''}`;
  if (item && onAction) {
    return (
      <button type="button" className={className} title={t(actionTip(item))} disabled={waiting !== null && waiting !== undefined} onClick={() => onAction(item)}>
        {children}
      </button>
    );
  }
  return <span className={className}>{children}</span>;
}

/** Cosa si puo' mostrare, dai dati della radio. */
function useShown(data: FaceExtras | null | undefined) {
  const x = data?.extras ?? {};
  return {
    x,
    vfoB: x.vfo_b_hz != null && x.vfo_b_hz > 0,
  };
}

/** L'unico tasto che resta: TUNE (acceso quando la radio sta accordando). */
function TuneChip({ data, onAction, waiting }: { data?: FaceExtras | null; onAction?: (i: CatExtraItem) => void; waiting?: CatExtraItem | null }) {
  return <Chip item="tune" on={data?.extras.tuner === true} onAction={onAction} waiting={waiting}>TUNE</Chip>;
}

/** Barre: S in ricezione; in trasmissione la potenza (Po) e il ROS (solo quelle che hanno un valore). */
function Bars({ model, cls }: { model: DisplayModel; cls: string }) {
  const m = model.meters;
  const s = sFraction(m);
  const po = powerFraction(m);
  const swr = swrFraction(m);
  const tx = model.transmitting;
  return (
    <div className={`md-bars ${cls}`}>
      {!tx && s != null ? <><span>S</span><span className="md-bar"><i style={{ width: pct(s) }} className={s > 0.6 ? 'md-bar--over' : ''} /></span><b>{sLabel(m)}</b></> : null}
      {tx && po != null ? <><span>Po</span><span className="md-bar"><i style={{ width: pct(po) }} /></span><b>{m?.power_w != null ? `${Math.round(m.power_w)}W` : ''}</b></> : null}
      {tx && swr != null ? <><span>SWR</span><span className="md-bar md-bar--swr"><i style={{ width: pct(swr) }} /></span><b>{m?.swr != null ? m.swr.toFixed(1) : ''}</b></> : null}
    </div>
  );
}

/** S-meter a lancetta come sulle Icom. */
function Needle({ model, height = 54, dim = false }: { model: DisplayModel; height?: number; dim?: boolean }) {
  const f = sFraction(model.meters) ?? 0;
  const a = Math.PI * (1 - Math.min(1, Math.max(0, f)));
  const x = 60 + 54 * Math.cos(a);
  const y = 62 - 54 * Math.sin(a);
  const ink = dim ? '#9fb8ee' : '#fff';
  return (
    <svg className="md-needle" viewBox="0 0 120 64" height={height} aria-hidden="true">
      <path d="M8 60 A52 52 0 0 1 112 60" fill="none" stroke={ink} strokeWidth="1.3" />
      <path d="M86 15 A52 52 0 0 1 112 60" fill="none" stroke="#ff5252" strokeWidth="3" />
      <g fontSize="9" fill={ink}><text x="8" y="54">1</text><text x="36" y="22">5</text><text x="66" y="12">9</text></g>
      <text x="92" y="20" fontSize="9" fill="#ff5252">+40</text>
      <line x1="60" y1="62" x2={x} y2={y} stroke={ink} strokeWidth="1.8" />
      {/* la lettura, con la tilde se l'S-meter e' stimato dall'audio */}
      {sIsEstimated(model.meters) ? <text x="60" y="44" fontSize="10" textAnchor="middle" fill={ink}>{sLabel(model.meters)}</text> : null}
    </svg>
  );
}

const freq = (hz: number) => formatRadioFrequency(hz, 'hz');

function Frame({ cls, ariaLabel, children }: { cls: string; ariaLabel: string; children: ReactNode }) {
  return (
    <div className={`md ${cls}`} role="img" aria-label={ariaLabel}>
      {children}
    </div>
  );
}

function NoRadio({ model, noRadio }: { model: DisplayModel; noRadio: string }) {
  return hasFrequency(model.dialHz) ? null : <span className="md-noradio">{noRadio}</span>;
}

/* ---------- Estesi (640 x 90) ---------- */

/* ---------- Quasi quadrati (330 x 140) ---------- */

/** Q1 · Icom blu: modo e TX su una riga, frequenza grande, strumenti sotto (sta in 330 x 130). */
export function IcBlueSquare({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-q md-icblue md-q1" ariaLabel={ariaLabel}>
      <div className="md-row md-q1-chips"><TuneChip data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-row"><span className="md-tag md-tag--on">{model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}{s.vfoB ? <span className="md-soft md-push">B {freq(s.x.vfo_b_hz ?? 0)}</span> : null}</div>
      <div className="md-q1-main">
        <span className="md-freq md-freq--big">{freq(model.dialHz)}</span>
      </div>
      <Bars model={model} cls="md-bars--blue md-q1-bars" />
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/** Q2 · Icom nero con lancetta (stile IC-7300). */
export function IcBlackSquare({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  return (
    <Frame cls="md-q md-icblack" ariaLabel={ariaLabel}>
      <div className="md-keys"><TuneChip data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-body">
        <div className="md-row">{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}</div>
        <Needle model={model} height={56} />
        <div className="md-row md-base"><span className="md-tag md-tag--on">{model.mode || '—'}</span><span className="md-freq">{freq(model.dialHz)}</span></div>
      </div>
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/** Q3 · Yaesu ambra doppio ricevitore. */
export function YaesuSquare({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-q md-yaesu" ariaLabel={ariaLabel}>
      <div className="md-row md-wrap"><span className="md-amber">MAIN · {model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}<TuneChip data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-row md-base"><span className="md-freq md-freq--big md-mono">{formatRadioFrequency(model.dialHz)}</span>{s.vfoB ? <span className="md-freq md-freq--sub md-mono md-blue md-push">{formatRadioFrequency(s.x.vfo_b_hz ?? 0)}</span> : null}</div>
      <Bars model={model} cls="md-bars--yaesu" />
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}
