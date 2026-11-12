import '../../styles/modern-displays.css';
import type { ReactNode } from 'react';
import type { CatExtraItem } from '../../api/client';
import { agcName, useRadioExtras } from '../../lib/radioExtras';
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
 * al posto delle icone). Ogni indicatore compare solo se la radio lo da' (il dato c'e') e se e' spuntato in
 * Impostazioni › Stazione › Radio > «Display esteso: cosa mostrare»; con il clic (solo in testata) comanda la radio.
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
const bw = (hz: number | null) => (hz ? `BW ${(hz / 1000).toFixed(1)}k` : null);

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

/** Cosa si puo' mostrare, dai dati della radio e dalle scelte dell'operatore. */
function useShown(data: FaceExtras | null | undefined) {
  const prefs = useRadioExtras((s) => s.prefs);
  const x = data?.extras ?? {};
  return {
    x,
    preamp: prefs.frontEnd && x.preamp_db != null,
    att: prefs.frontEnd && x.att_db != null,
    agc: prefs.agc && x.agc != null,
    nb: prefs.noise && x.nb != null,
    nr: prefs.noise && x.nr != null,
    notch: prefs.noise && x.notch != null,
    tuner: prefs.tuner && x.tuner != null,
    vfoB: prefs.vfoB && x.vfo_b_hz != null && x.vfo_b_hz > 0,
    meters: prefs.meters,
    rit: prefs.ritXit && x.rit_hz != null,
    passband: data?.passbandHz ?? null,
    split: data?.split ?? null,
  };
}

/** Le funzioni accese/spente come tasti, nell'ordine delle radio. */
function FunctionChips({ data, onAction, waiting }: { data?: FaceExtras | null; onAction?: (i: CatExtraItem) => void; waiting?: CatExtraItem | null }) {
  const s = useShown(data);
  const { x } = s;
  return (
    <>
      {s.preamp ? <Chip item="preamp" on={(x.preamp_db ?? 0) > 0} onAction={onAction} waiting={waiting}>{(x.preamp_db ?? 0) > 0 ? `P.AMP ${x.preamp_db}` : 'P.AMP OFF'}</Chip> : null}
      {s.att ? <Chip item="att" on={(x.att_db ?? 0) > 0} onAction={onAction} waiting={waiting}>{(x.att_db ?? 0) > 0 ? `ATT ${x.att_db}` : 'ATT OFF'}</Chip> : null}
      {s.agc ? <Chip item="agc" on onAction={onAction} waiting={waiting}>AGC {agcName(x.agc ?? 0)}</Chip> : null}
      {s.nb ? <Chip item="nb" on={x.nb === true} onAction={onAction} waiting={waiting}>NB</Chip> : null}
      {s.nr ? <Chip item="nr" on={x.nr === true} onAction={onAction} waiting={waiting}>NR</Chip> : null}
      {s.notch ? <Chip item="notch" on={x.notch === true} onAction={onAction} waiting={waiting}>NOTCH</Chip> : null}
      {s.tuner ? <Chip item="atu" on={x.tuner === true} onAction={onAction} waiting={waiting}>TUNER</Chip> : null}
    </>
  );
}

/** Banda passante del filtro come sulle Icom: un trapezio con la larghezza. */
function Passband({ hz, height = 40 }: { hz: number | null; height?: number }) {
  if (!hz) return null;
  const w = Math.max(30, Math.min(100, (hz / 3600) * 100));
  const left = 60 - w / 2;
  return (
    <svg className="md-pb" viewBox="0 0 120 44" height={height} aria-hidden="true">
      <line x1="2" y1="42" x2="118" y2="42" stroke="#6f92d8" />
      <path d={`M${left} 42 L${left + 10} 8 L${left + w - 10} 8 L${left + w} 42`} fill="#1e4fb0" fillOpacity=".55" stroke="#fff" strokeWidth="1.6" />
      <text x="3" y="9" fontSize="9" fill="#cfe0ff">{bw(hz)}</text>
    </svg>
  );
}

/** Barre S, Po, SWR (solo quelle che hanno un valore). */
function Bars({ model, cls }: { model: DisplayModel; cls: string }) {
  const m = model.meters;
  const s = sFraction(m);
  const po = powerFraction(m);
  const swr = swrFraction(m);
  return (
    <div className={`md-bars ${cls}`}>
      {s != null ? <><span>S</span><span className="md-bar"><i style={{ width: pct(s) }} className={s > 0.6 ? 'md-bar--over' : ''} /></span><b>{sLabel(m)}</b></> : null}
      {po != null ? <><span>Po</span><span className="md-bar"><i style={{ width: pct(po) }} /></span><b>{m?.power_w != null ? `${Math.round(m.power_w)}W` : ''}</b></> : null}
      {swr != null ? <><span>SWR</span><span className="md-bar md-bar--swr"><i style={{ width: pct(swr) }} /></span><b>{m?.swr != null ? m.swr.toFixed(1) : ''}</b></> : null}
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

/** E1 · Icom doppio ricevitore (stile IC-7610). */
export function IcDualDisplay({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-e md-icdual" ariaLabel={ariaLabel}>
      <div className="md-half">
        <div>
          <div className="md-row"><span className="md-tag md-tag--on">MAIN</span><span>{model.mode || '—'}</span>{bw(s.passband) ? <span className="md-soft">{bw(s.passband)}</span> : null}{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}</div>
          <span className="md-freq">{freq(model.dialHz)}</span>
        </div>
        {s.meters ? <Needle model={model} /> : null}
      </div>
      <div className="md-half md-half--sub">
        {s.vfoB ? (
          <div>
            <div className="md-row"><span className="md-tag">SUB</span><span>{s.x.vfo_b_mode ?? ''}</span></div>
            <span className="md-freq md-freq--sub">{freq(s.x.vfo_b_hz ?? 0)}</span>
          </div>
        ) : (
          <div className="md-row md-wrap"><FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
        )}
        {s.vfoB ? <div className="md-row md-wrap md-col md-chips-side"><FunctionChips data={data} onAction={onAction} waiting={waiting} /></div> : null}
      </div>
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/** E2 · Icom blu con banda passante. */
export function IcBlueDisplay({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-e md-icblue" ariaLabel={ariaLabel}>
      <div>
        <div className="md-row"><span className="md-tag md-tag--on">{s.x.vfo ? s.x.vfo.replace('VFO', 'VFO ') : 'VFO A'}</span><span className="md-tag md-tag--on">{model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}{s.split ? <span className="md-tag">SPLIT</span> : null}</div>
        <span className="md-freq">{freq(model.dialHz)}</span>
      </div>
      <div className="md-mid">
        <Passband hz={s.passband} height={34} />
        <div className="md-row md-wrap"><FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
      </div>
      {s.meters ? <Bars model={model} cls="md-bars--blue" /> : <span />}
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/** E3 · LED essenziale (stile K4): tasti in alto, sotto la frequenza con gli strumenti in linea, a destra il VFO B. */
export function LedLineDisplay({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-e md-ledline" ariaLabel={ariaLabel}>
      <div className="md-led-left">
        <div className="md-row md-amber md-led-chips"><span>{model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}<FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
        <div className="md-led-line">
          <span className="md-freq md-mono">{formatRadioFrequency(model.dialHz)}</span>
          {s.meters ? <Bars model={model} cls="md-bars--led md-led-bars" /> : null}
        </div>
      </div>
      {s.vfoB ? <div className="md-right"><span className="md-soft">VFO B</span><span className="md-freq md-freq--sub md-mono">{formatRadioFrequency(s.x.vfo_b_hz ?? 0)}</span></div> : <span />}
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/* ---------- Quasi quadrati (330 x 140) ---------- */

/** Q1 · Icom blu: tasti su una riga, frequenza con la banda passante a fianco, strumenti sotto (sta in 330 x 130). */
export function IcBlueSquare({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-q md-icblue md-q1" ariaLabel={ariaLabel}>
      <div className="md-row md-q1-chips"><FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-row"><span className="md-tag md-tag--on">{model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}{s.split ? <span className="md-tag">SPLIT</span> : null}{s.vfoB ? <span className="md-soft md-push">B {freq(s.x.vfo_b_hz ?? 0)}</span> : null}</div>
      <div className="md-q1-main">
        <span className="md-freq md-freq--big">{freq(model.dialHz)}</span>
        <Passband hz={s.passband} height={28} />
      </div>
      {s.meters ? <Bars model={model} cls="md-bars--blue md-q1-bars" /> : null}
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}

/** Q2 · Icom nero con lancetta (stile IC-7300). */
export function IcBlackSquare({ model, noRadio, ariaLabel, data, onAction, waiting }: ModernProps) {
  const s = useShown(data);
  return (
    <Frame cls="md-q md-icblack" ariaLabel={ariaLabel}>
      <div className="md-keys"><FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-body">
        <div className="md-row">{bw(s.passband) ? <span className="md-tag">{bw(s.passband)}</span> : null}{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}{s.split ? <span className="md-tag">SPLIT</span> : null}</div>
        {s.meters ? <Needle model={model} height={56} /> : <span />}
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
      <div className="md-row md-wrap"><span className="md-amber">MAIN · {model.mode || '—'}</span>{model.transmitting ? <span className="md-tag md-tag--tx">TX</span> : null}<FunctionChips data={data} onAction={onAction} waiting={waiting} /></div>
      <div className="md-row md-base"><span className="md-freq md-freq--big md-mono">{formatRadioFrequency(model.dialHz)}</span>{s.vfoB ? <span className="md-freq md-freq--sub md-mono md-blue md-push">{formatRadioFrequency(s.x.vfo_b_hz ?? 0)}</span> : null}</div>
      {s.meters ? <Bars model={model} cls="md-bars--yaesu" /> : <span />}
      <NoRadio model={model} noRadio={noRadio} />
    </Frame>
  );
}
