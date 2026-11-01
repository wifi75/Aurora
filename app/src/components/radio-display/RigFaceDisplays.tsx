import type { CatExtraItem, RigExtras } from '../../api/client';
import { useT } from '../../i18n';
import { agcName } from '../../lib/radioExtras';
import { formatRadioFrequency, hasFrequency, mainMeter, meterReadouts, S_TICKS, S9_FRACTION, type DisplayModel } from '../../lib/radioDisplay';
import { actionTip } from './RadioExtras';
import { DisplayFrame, FONT_UI, Label, linkState, polar, arcPath, SegBar } from './instruments';
import { SevenSegment } from './SevenSegment';

/**
 * Display a pannello unico ispirati a due radio vere: un solo schermo largo con VFO A, VFO B, strumenti e indicatori.
 * Il disegno e' un SVG di misura fissa (960x96, come la testata: 40rem x 4rem); gli indicatori che si possono cambiare
 * dalla radio hanno sopra un pulsante trasparente nella stessa posizione.
 */

export const FACE_W = 960;
export const FACE_H = 96;

/** I dati in piu' della radio che i pannelli disegnano (ognuno solo se la radio lo da'). */
export interface FaceExtras {
  extras: RigExtras;
  passbandHz: number | null;
  split: boolean | null;
}

/** Dati di prova per le anteprime nelle Impostazioni. */
export const DEMO_FACE_EXTRAS: FaceExtras = {
  extras: { preamp_db: 0, att_db: 0, agc: 2, nb: true, nr: false, notch: false, tuner: false, rit_hz: 0, vfo: 'VFOA', vfo_b_hz: 21_076_500, vfo_b_mode: 'USB' },
  passbandHz: 3000,
  split: false,
};

const NO_EXTRAS: FaceExtras = { extras: {}, passbandHz: null, split: null };

interface Hit {
  key: CatExtraItem;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}

interface FaceProps {
  model: DisplayModel;
  noRadio: string;
  ariaLabel: string;
  data?: FaceExtras | null;
  /** Con il clic: gli indicatori diventano pulsanti (in testata); senza, il disegno e' solo un'immagine (anteprime). */
  onAction?: (item: CatExtraItem) => void;
  waiting?: CatExtraItem | null;
}

const khz = (hz: number) => `${(hz / 1000).toFixed(1)}k`;
const signed = (n: number) => `${n > 0 ? '+' : n < 0 ? '−' : ''}${Math.abs(n)}`;

/** Pulsanti trasparenti sopra il disegno, nelle posizioni degli indicatori (in percentuale del disegno). */
function FaceHits({ hits, onAction, waiting }: { hits: Hit[]; onAction?: (item: CatExtraItem) => void; waiting?: CatExtraItem | null }) {
  const t = useT();
  if (!onAction) return null;
  return (
    <>
      {hits.map((h) => (
        <button
          key={`${h.key}-${h.x}`}
          type="button"
          className={`rd-face-hit${waiting === h.key ? ' rd-face-hit--wait' : ''}`}
          style={{ left: `${(h.x / FACE_W) * 100}%`, top: `${(h.y / FACE_H) * 100}%`, width: `${(h.w / FACE_W) * 100}%`, height: `${(h.h / FACE_H) * 100}%` }}
          title={t(actionTip(h.key))}
          aria-label={`${h.text}: ${t(actionTip(h.key))}`}
          aria-busy={waiting === h.key}
          disabled={waiting != null}
          onClick={() => onAction(h.key)}
        />
      ))}
    </>
  );
}

/* ---------- stile FT-2000: cifre a 7 segmenti azzurre, riquadri con valori ambra, spie colorate ---------- */

const FT = { cyan: '#5ce8ff', glow: '#1aa8d0', amber: '#ffb428', ink: '#d8f4ff', dim: '#3b5560', box: '#2a6a80', red: '#ff3b2f', yellow: '#ffd23a' };

function FtSpy({ x, y, text, color, on }: { x: number; y: number; text: string; color: string; on: boolean }) {
  return (
    <g aria-hidden="true">
      <rect x={x} y={y} width={36} height={13} rx={1.5} fill={on ? color : 'none'} stroke={color} strokeWidth={1} opacity={on ? 1 : 0.25} />
      <text x={x + 18} y={y + 9.8} textAnchor="middle" fontSize={8.5} fontWeight={700} fontFamily={FONT_UI} fill={on ? '#05080a' : color} opacity={on ? 1 : 0.35}>
        {text}
      </text>
    </g>
  );
}

export function Ft2000Display({ model, noRadio, ariaLabel, data, onAction, waiting }: FaceProps) {
  const { extras: x, passbandHz } = data ?? NO_EXTRAS;
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const readouts = meterReadouts(model.meters).filter((r) => r.key === 'PO' || r.key === 'SWR');
  const hits: Hit[] = [];

  const boxes: Array<{ label: string; value: string | null; key?: CatExtraItem }> = [
    { label: 'ANT', value: null },
    { label: 'ATT', value: x.att_db == null ? null : x.att_db > 0 ? `${Math.round(x.att_db)}dB` : 'OFF', key: 'att' },
    { label: 'FLT', value: passbandHz ? khz(passbandHz) : null },
    { label: 'IPO', value: x.preamp_db == null ? null : x.preamp_db > 0 ? `P${Math.round(x.preamp_db)}` : 'ON', key: 'preamp' },
    { label: 'R.FLT', value: null },
    { label: 'AGC', value: x.agc == null ? null : agcName(x.agc), key: 'agc' },
  ];
  boxes.forEach((b, i) => {
    if (b.key && b.value !== null) hits.push({ key: b.key, x: 12 + i * 38, y: 9, w: 36, h: 34, text: `${b.label} ${b.value}` });
  });

  const spies: Array<{ text: string; color: string; on: boolean; key?: CatExtraItem; known: boolean }> = [
    { text: 'PROC', color: FT.red, on: model.transmitting && (model.meters?.comp_db ?? 0) > 0, known: true },
    { text: 'TUNER', color: FT.cyan, on: x.tuner === true, key: 'atu', known: x.tuner != null },
    { text: 'BUSY', color: FT.yellow, on: link.busy, known: true },
    { text: 'NB', color: FT.yellow, on: x.nb === true, key: 'nb', known: x.nb != null },
    { text: 'NR', color: FT.yellow, on: x.nr === true, key: 'nr', known: x.nr != null },
    { text: 'NOTCH', color: FT.yellow, on: x.notch === true, key: 'notch', known: x.notch != null },
  ];
  spies.forEach((s, i) => {
    if (s.key && s.known) hits.push({ key: s.key, x: 12 + i * 38, y: 49, w: 36, h: 15, text: s.text });
  });

  const rit = x.rit_hz;
  const ritPos = 361 + Math.max(-1, Math.min(1, (rit ?? 0) / 1000)) * 100;
  if (rit != null) hits.push({ key: 'rit', x: 480, y: 60, w: 110, h: 28, text: `CLAR ${signed(rit)}` });
  const vfoTag = x.vfo ?? null;
  if (vfoTag) hits.push({ key: 'vfo', x: 246, y: 8, w: 48, h: 16, text: vfoTag });

  const bHz = x.vfo_b_hz ?? 0;
  return (
    <div className="rd-face">
      <DisplayFrame width={FACE_W} height={FACE_H} radius={10} inset={3} screen="#020405" ariaLabel={ariaLabel}>
        {/* riquadri con etichetta e valore */}
        {boxes.map((b, i) => (
          <g key={b.label} aria-hidden="true">
            <rect x={12 + i * 38} y={9} width={36} height={34} rx={2} fill="#04121a" stroke={FT.box} strokeWidth={1} />
            <text x={30 + i * 38} y={21} textAnchor="middle" fontSize={8.5} fontWeight={600} fontFamily={FONT_UI} fill={FT.ink} opacity={b.value === null ? 0.45 : 1}>
              {b.label}
            </text>
            <text x={30 + i * 38} y={37} textAnchor="middle" fontSize={10} fontWeight={700} fontFamily={FONT_UI} fill={b.value === null ? FT.dim : FT.amber}>
              {b.value ?? '—'}
            </text>
          </g>
        ))}
        {spies.map((s, i) => (
          <FtSpy key={s.text} x={12 + i * 38} y={50} text={s.text} color={s.color} on={s.on} />
        ))}
        <FtSpy x={12} y={72} text="RX" color="#47d13a" on={link.rx && known} />
        <FtSpy x={50} y={72} text="TX" color={FT.red} on={link.tx} />
        <Label x={94} y={82} text={readouts.map((r) => `${r.key} ${r.text}`).join('  ')} fill={FT.cyan} size={10} weight={600} />

        {/* VFO A */}
        <Label x={250} y={20} text="VFO-A" fill={FT.cyan} size={9} weight={700} />
        <Label x={250} y={36} text={model.mode || ''} fill={FT.amber} size={10} weight={700} />
        <SevenSegment text={formatRadioFrequency(model.dialHz)} x={300} y={9} w={26} h={44} t={4.6} gap={7} skew={0.08} on={FT.cyan} glow={FT.glow} offOpacity={0.1} />
        {known ? (
          <g aria-hidden="true" opacity={rit == null ? 0.35 : 1}>
            <text x={252} y={80} fontSize={14} fontWeight={700} fill={FT.ink} fontFamily={FONT_UI}>−</text>
            <text x={462} y={80} fontSize={14} fontWeight={700} fill={FT.ink} fontFamily={FONT_UI}>+</text>
            <rect x={263} y={75} width={196} height={1.4} fill={FT.ink} />
            {Array.from({ length: 21 }, (_, i) => (
              <rect key={i} x={263 + i * 9.7} y={i % 5 === 0 ? 68 : 71} width={1.2} height={i % 5 === 0 ? 8 : 5} fill={FT.ink} />
            ))}
            <polygon points={`${ritPos - 5},86 ${ritPos + 5},86 ${ritPos},78`} fill={FT.red} />
          </g>
        ) : (
          <Label x={252} y={78} text={noRadio} fill={FT.amber} size={12} weight={600} />
        )}
        <g aria-hidden="true" opacity={rit == null ? 0.35 : 1}>
          <rect x={480} y={60} width={110} height={28} rx={2} fill="none" stroke={FT.box} />
          <text x={485} y={70} fontSize={8} fontWeight={700} fill={FT.ink} fontFamily={FONT_UI}>RX</text>
          <text x={585} y={70} textAnchor="end" fontSize={8} fontWeight={700} fill={FT.ink} fontFamily={FONT_UI}>CLAR</text>
        </g>
        <Label x={535} y={84} text={rit == null ? '-.---' : `${rit < 0 ? '-' : ''}${(Math.abs(rit) / 1000).toFixed(3)}`} fill={rit == null ? FT.dim : FT.cyan} size={13} weight={700} anchor="middle" />

        {/* S-meter a barra con la scala */}
        {S_TICKS.map((tk) => (
          <Label key={tk.label} x={612 + tk.fraction * 196} y={16} text={tk.label} fill={tk.fraction > S9_FRACTION ? FT.red : FT.ink} size={8} weight={600} anchor="middle" />
        ))}
        <SegBar x={608} y={20} count={40} segW={3.6} segH={9} step={5} lit={main.fraction === null ? 0 : Math.round(main.fraction * 40)} color={(i) => (i >= 40 * S9_FRACTION ? FT.red : FT.cyan)} off="#0e2630" />
        <Label x={820} y={46} text={main.label ? (main.kind === 'S' ? main.label : `PO ${main.label}`) : ''} fill={FT.amber} size={10} weight={700} anchor="end" />

        {/* VFO B */}
        <Label x={610} y={46} text="VFO-B" fill={FT.cyan} size={9} weight={700} opacity={bHz ? 1 : 0.4} />
        <Label x={660} y={46} text={x.vfo_b_mode ?? ''} fill={FT.amber} size={9} weight={700} />
        <SevenSegment text={formatRadioFrequency(bHz)} x={612} y={54} w={15} h={28} t={3} gap={5} skew={0.08} on={FT.cyan} glow={bHz ? FT.glow : undefined} offOpacity={0.1} />

        {/* modo e AGC */}
        <rect x={830} y={9} width={120} height={78} rx={2} fill="none" stroke={FT.box} />
        <Label x={890} y={40} text={model.mode || '—'} fill={model.mode ? FT.ink : FT.dim} size={22} weight={700} anchor="middle" />
        <Label x={890} y={60} text="AGC" fill={FT.ink} size={9} weight={600} anchor="middle" />
        <Label x={890} y={78} text={x.agc == null ? '—' : agcName(x.agc)} fill={x.agc == null ? FT.dim : FT.amber} size={13} weight={700} anchor="middle" />
      </DisplayFrame>
      <FaceHits hits={hits} onAction={onAction} waiting={waiting} />
    </div>
  );
}

/* ---------- stile IC-7610: due VFO affiancati con S-meter ad arco, colonna di pulsanti a sinistra ---------- */

const IC = { white: '#f4f6fa', label: '#c7d2e2', blue: '#1aa0e8', val: '#7fd4ff', dim: '#3a4656', red: '#ff3a2a', green: '#3fe07a', panel: '#05080d', edge: '#1d2a3a' };

function IcArc({ cx, cy, r, fraction, lit, label }: { cx: number; cy: number; r: number; fraction: number | null; lit: boolean; label: string }) {
  const a0 = 150;
  const span = 120;
  const a9 = a0 - span * S9_FRACTION;
  const needle = fraction === null ? null : a0 - span * Math.max(0, Math.min(1, fraction));
  return (
    <g aria-hidden="true" opacity={lit ? 1 : 0.35}>
      <path d={arcPath(cx, cy, r, a0, a9)} stroke={IC.white} strokeWidth={2} fill="none" />
      <path d={arcPath(cx, cy, r, a9, a0 - span)} stroke={IC.red} strokeWidth={3} fill="none" />
      {S_TICKS.map((tk) => {
        const a = a0 - span * tk.fraction;
        const [x1, y1] = polar(cx, cy, r, a);
        const [x2, y2] = polar(cx, cy, r + 4, a);
        const [tx, ty] = polar(cx, cy, r + 11, a);
        const col = tk.fraction > S9_FRACTION ? IC.red : IC.white;
        return (
          <g key={tk.label}>
            <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={col} strokeWidth={1.2} />
            <text x={tx} y={ty + 3} textAnchor="middle" fontSize={7.5} fontWeight={600} fill={col} fontFamily={FONT_UI}>
              {tk.label}
            </text>
          </g>
        );
      })}
      <text x={cx - r + 2} y={cy - 4} fontSize={8} fontWeight={700} fill={IC.label} fontFamily={FONT_UI}>
        {label}
      </text>
      {needle !== null ? <line x1={cx} y1={cy} x2={polar(cx, cy, r + 2, needle)[0]} y2={polar(cx, cy, r + 2, needle)[1]} stroke={IC.white} strokeWidth={1.6} strokeLinecap="round" /> : null}
    </g>
  );
}

function IcTag({ x, y, w, text, fill, fg, on = true }: { x: number; y: number; w: number; text: string; fill: string; fg: string; on?: boolean }) {
  return (
    <g aria-hidden="true" opacity={on ? 1 : 0.3}>
      <rect x={x} y={y} width={w} height={13} rx={2} fill={on ? fill : 'none'} stroke={fill} strokeWidth={1} />
      <text x={x + w / 2} y={y + 9.8} textAnchor="middle" fontSize={8.5} fontWeight={700} fontFamily={FONT_UI} fill={on ? fg : fill}>
        {text}
      </text>
    </g>
  );
}

export function Ic7610Display({ model, noRadio, ariaLabel, data, onAction, waiting }: FaceProps) {
  const { extras: x, passbandHz, split } = data ?? NO_EXTRAS;
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const hits: Hit[] = [];
  const comp = model.meters?.comp_db;

  const buttons: Array<{ label: string; value: string | null; key?: CatExtraItem }> = [
    { label: 'ANT', value: null },
    { label: 'P.AMP', value: x.preamp_db == null ? null : x.preamp_db > 0 ? `${Math.round(x.preamp_db)}dB` : 'OFF', key: 'preamp' },
    { label: 'ATT', value: x.att_db == null ? null : x.att_db > 0 ? `${Math.round(x.att_db)}dB` : 'OFF', key: 'att' },
    { label: 'IP+', value: null },
    { label: 'AGC', value: x.agc == null ? null : agcName(x.agc), key: 'agc' },
    { label: 'VOX', value: null },
    { label: 'COMP', value: model.transmitting && comp != null ? `${Math.round(comp)}dB` : null },
  ];
  const pos = (i: number) => ({ x: 10 + (i % 2) * 57, y: 8 + Math.floor(i / 2) * 21 });
  buttons.forEach((b, i) => {
    if (b.key && b.value !== null) hits.push({ key: b.key, ...pos(i), w: 54, h: 18, text: `${b.label} ${b.value}` });
  });

  const noise: Array<{ text: string; on: boolean | null | undefined; key: CatExtraItem }> = [
    { text: 'NB', on: x.nb, key: 'nb' },
    { text: 'NR', on: x.nr, key: 'nr' },
    { text: 'NOTCH', on: x.notch, key: 'notch' },
    { text: 'ATU', on: x.tuner, key: 'atu' },
  ];
  noise.forEach((n, i) => {
    if (n.on != null) hits.push({ key: n.key, x: 300 + i * 40, y: 7, w: 38, h: 15, text: n.text });
  });
  if (x.vfo) hits.push({ key: 'vfo', x: 132, y: 7, w: 36, h: 15, text: x.vfo });

  const bHz = x.vfo_b_hz ?? 0;
  const panel = (x0: number, sub: boolean) => {
    const freq = sub ? bHz : model.dialHz;
    const mode = sub ? x.vfo_b_mode ?? '' : model.mode;
    const has = sub ? bHz > 0 : known;
    return (
      <g key={sub ? 'sub' : 'main'}>
        <rect x={x0} y={6} width={408} height={84} rx={3} fill={IC.panel} stroke={IC.edge} />
        <IcTag x={x0 + 4} y={8} w={32} text={sub ? 'SUB' : 'MAIN'} fill={!sub && link.tx ? IC.red : IC.blue} fg="#02060c" on={has} />
        <Label x={x0 + 44} y={18} text="ANT1" fill={IC.dim} size={8.5} weight={600} />
        <Label x={x0 + 76} y={18} text={`BW ${!sub && passbandHz ? khz(passbandHz) : '—'}`} fill={!sub && passbandHz ? IC.label : IC.dim} size={8.5} weight={600} />
        <Label x={x0 + 130} y={18} text="SFT —" fill={IC.dim} size={8.5} weight={600} />
        <IcTag x={x0 + 4} y={30} w={40} text={mode || '—'} fill={IC.blue} fg="#02060c" on={has && mode !== ''} />
        {has ? (
          <Label x={x0 + 52} y={66} text={formatRadioFrequency(freq)} fill={IC.white} size={sub ? 30 : 34} weight={700} />
        ) : (
          <>
            <Label x={x0 + 52} y={62} text={formatRadioFrequency(0)} fill={IC.dim} size={30} weight={700} />
            {!sub ? <Label x={x0 + 52} y={82} text={noRadio} fill="#ffd34a" size={10} weight={600} /> : null}
          </>
        )}
        <IcArc cx={x0 + 340} cy={86} r={44} fraction={sub ? null : main.fraction} lit={sub ? false : known} label={!sub && main.kind === 'PO' ? 'Po' : 'S'} />
      </g>
    );
  };

  return (
    <div className="rd-face">
      <DisplayFrame width={FACE_W} height={FACE_H} radius={10} inset={3} screen="#000000" ariaLabel={ariaLabel}>
        {buttons.map((b, i) => {
          const p = pos(i);
          return (
            <g key={b.label} aria-hidden="true">
              <rect x={p.x} y={p.y} width={54} height={18} rx={2} fill="#141c28" stroke="#33445a" />
              <text x={p.x + 4} y={p.y + 12} fontSize={8} fontWeight={700} fontFamily={FONT_UI} fill={IC.label} opacity={b.value === null ? 0.45 : 1}>
                {b.label}
              </text>
              <text x={p.x + 50} y={p.y + 12} textAnchor="end" fontSize={8} fontWeight={700} fontFamily={FONT_UI} fill={b.value === null ? IC.dim : IC.val}>
                {b.value ?? '—'}
              </text>
            </g>
          );
        })}
        <IcTag x={67} y={71} w={54} text="TX" fill={IC.red} fg="#ffffff" on={link.tx} />
        {panel(128, false)}
        {panel(544, true)}
        {noise.map((n, i) => (
          <IcTag key={n.text} x={300 + i * 40} y={8} w={38} text={n.text} fill={IC.green} fg="#02140a" on={n.on === true} />
        ))}
        {main.label ? <Label x={530} y={18} text={main.kind === 'S' ? main.label : `PO ${main.label}`} fill={IC.label} size={9} weight={700} anchor="end" /> : null}
        <IcTag x={132} y={50} w={40} text="DUAL-W" fill={IC.green} fg="#02140a" on={false} />
        <IcTag x={132} y={68} w={40} text="SPLIT" fill={IC.green} fg="#02140a" on={split === true} />
      </DisplayFrame>
      <FaceHits hits={hits} onAction={onAction} waiting={waiting} />
    </div>
  );
}
