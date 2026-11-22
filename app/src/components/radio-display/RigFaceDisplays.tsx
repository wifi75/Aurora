import type { CatExtraItem, RigExtras } from "../../api/client";
import { useT } from "../../i18n";
import {
  formatRadioFrequency,
  hasFrequency,
  mainMeter,
  meterReadouts,
  S_TICKS,
  S9_FRACTION,
  sIsEstimated,
  type DisplayModel,
} from "../../lib/radioDisplay";
import { actionTip } from "./RadioExtras";
import {
  DisplayFrame,
  FONT_UI,
  Label,
  linkState,
  polar,
  arcPath,
  SegBar,
  SpyTip,
} from "./instruments";
import { SevenSegment, digitsWidth } from "./SevenSegment";

/**
 * Display a pannello unico ispirati a due radio vere: un solo schermo largo con VFO A, VFO B e gli strumenti. Restano solo dati e funzioni
 * che esistono davvero: frequenza (VFO A e, se la radio lo da', VFO B), modo, stati RX/TX/DEC, S-meter in ricezione e potenza in
 * trasmissione (con il ROS), e il tasto TUNE, l'unico con sopra un pulsante trasparente (in testata).
 * Il disegno e' un SVG alto 96; la larghezza dipende dallo stile (FT-2000 e IC-7610 hanno la loro, ritagliata sul contenuto).
 */

export const FACE_H = 96;
/** Misure del disegno FT-2000 compatto (2026-11-19): frequenza, stati, S-meter/potenza e VFO B senza spazi vuoti. */
export const FT_FACE_W = 504;
/** Spazio in piu' dello stile con potenza, ROS e ALC: tre righe di lettura tra gli indicatori del VFO A e l'S-meter. */
export const FT_POWER_BLOCK_W = 112;
export const FT_FACE_WP = FT_FACE_W + FT_POWER_BLOCK_W;
export const FT_FACE_H = 66;
/** Larghezza del disegno IC-7610 (due pannelli VFO affiancati, senza colonna di tasti). */
export const IC_FACE_W = 684;

/** I dati in piu' della radio che i pannelli disegnano (ognuno solo se la radio lo da'). */
export interface FaceExtras {
  extras: RigExtras;
}

/** Dati di prova per le anteprime nelle Impostazioni. */
export const DEMO_FACE_EXTRAS: FaceExtras = {
  extras: {
    tuner: false,
    vfo_b_hz: 21_076_500,
    vfo_b_mode: "USB",
  },
};

const NO_EXTRAS: FaceExtras = { extras: {} };

/** Un pulsante trasparente sopra il disegno: l'unico che resta e' TUNE. */
interface Hit {
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
  /** Con il clic: TUNE diventa un pulsante (in testata); senza, il disegno e' solo un'immagine (anteprime). */
  onAction?: (item: CatExtraItem) => void;
  /** Stile con la lettura grande di ROS, ALC e potenza in trasmissione tra il VFO A e l'S-meter (richiesta di Pascal). */
  power?: boolean;
  waiting?: CatExtraItem | null;
}

/** Pulsante trasparente di TUNE sopra il disegno, nella posizione dell'indicatore (in percentuale del disegno). */
function FaceHits({
  hits,
  onAction,
  waiting,
  width,
  height = FACE_H,
}: {
  width: number;
  height?: number;
  hits: Hit[];
  onAction?: (item: CatExtraItem) => void;
  waiting?: CatExtraItem | null;
}) {
  const t = useT();
  if (!onAction) return null;
  return (
    <>
      {hits.map((h) => (
        <button
          key={`tune-${h.x}`}
          type="button"
          className={`rd-face-hit${waiting === "tune" ? " rd-face-hit--wait" : ""}`}
          style={{
            left: `${(h.x / width) * 100}%`,
            top: `${(h.y / height) * 100}%`,
            width: `${(h.w / width) * 100}%`,
            height: `${(h.h / height) * 100}%`,
          }}
          title={t(actionTip("tune"))}
          aria-label={`${h.text}: ${t(actionTip("tune"))}`}
          aria-busy={waiting === "tune"}
          disabled={waiting != null}
          onClick={() => onAction("tune")}
        />
      ))}
    </>
  );
}

/* ---------- stile FT-2000: cifre a 7 segmenti azzurre, riquadri con valori ambra, spie colorate ---------- */

const FT = {
  cyan: "#5ce8ff",
  glow: "#1aa8d0",
  amber: "#ffb428",
  ink: "#d8f4ff",
  dim: "#3b5560",
  box: "#2a6a80",
  red: "#ff3b2f",
  yellow: "#ffd23a",
};

function FtSpy({
  x,
  y,
  text,
  color,
  on,
}: {
  x: number;
  y: number;
  text: string;
  color: string;
  on: boolean;
}) {
  return (
    <g aria-hidden="true">
      <SpyTip text={text} />
      <rect
        x={x}
        y={y}
        width={36}
        height={13}
        rx={1.5}
        fill={on ? color : "none"}
        stroke={color}
        strokeWidth={1}
        opacity={on ? 1 : 0.6}
      />
      <text
        x={x + 18}
        y={y + 9.8}
        textAnchor="middle"
        fontSize={8.5}
        fontWeight={700}
        fontFamily={FONT_UI}
        fill={on ? "#05080a" : color}
        opacity={on ? 1 : 0.75}
      >
        {text}
      </text>
    </g>
  );
}

/** Il testo della lettura principale: S in ricezione, PO in trasmissione (con il ROS se la radio lo da'). Lo usa il pannello IC-7610. */
function mainReading(model: DisplayModel, main: ReturnType<typeof mainMeter>): string {
  const base = main.label ? (main.kind === "S" ? main.label : `PO ${main.label}`) : "";
  const swr = model.transmitting
    ? meterReadouts(model.meters).find((r) => r.key === "SWR")?.text
    : undefined;
  return swr ? `${base}${base ? "  " : ""}SWR ${swr}` : base;
}

export function Ft2000Display({
  model,
  noRadio: _noRadio,
  ariaLabel,
  data,
  onAction,
  waiting,
  power = false,
}: FaceProps) {
  const { extras: x } = data ?? NO_EXTRAS;
  const faceW = power ? FT_FACE_WP : FT_FACE_W;
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const sending = known && link.tx;
  const swrNow = typeof model.meters?.swr === "number" ? model.meters.swr : null;
  const swrColor = swrNow !== null && swrNow >= 3 ? FT.red : swrNow !== null && swrNow >= 2 ? FT.amber : "#5be08a";

  // VFO A: l'etichetta sta sopra la frequenza, che occupa 8 cifre piene; RX, TX, BUSY e TUNE le stanno subito accanto; a destra l'S-meter
  // (in trasmissione la potenza, con sotto la barra del ROS) e il VFO B. Niente scritta del modo ne' lettura numerica dell'S-meter.
  const freqStyle = { w: 22, h: 44, t: 4.4, gap: 5, skew: 0.08 };
  const FREQ_X = 8;
  const FREQ_Y = 15;
  const spyX = FREQ_X + digitsWidth("000.000.00", freqStyle) + 6;
  const blockX = spyX + 34 + 8;
  const R0 = spyX + 34 + 10 + (power ? FT_POWER_BLOCK_W : 0);
  const readouts = meterReadouts(model.meters);
  const readout = (key: 'PO' | 'SWR' | 'ALC') => readouts.find((r) => r.key === key)?.text ?? '--';
  const swrText = readout('SWR') === '--' ? '--' : `1:${readout('SWR')}`;
  const bHz = x.vfo_b_hz ?? 0;
  const hits: Hit[] = [{ x: spyX - 1, y: 47, w: 36, h: 14, text: "TUNE" }];
  return (
    <div className="rd-face rd-face--ft">
      <DisplayFrame
        width={faceW}
        height={FT_FACE_H}
        radius={9}
        inset={3}
        screen="#020405"
        ariaLabel={ariaLabel}
      >
        {/* VFO A */}
        <Label x={FREQ_X + 1} y={12} text="VFO-A" fill={FT.cyan} size={9} weight={700} />
        <SevenSegment
          text={formatRadioFrequency(model.dialHz)}
          x={FREQ_X}
          y={FREQ_Y}
          {...freqStyle}
          on={FT.cyan}
          glow={FT.glow}
          offOpacity={0.1}
        />
        {/* stati: RX, TX, BUSY (decodifica) e il tasto TUNE */}
        <FtSpy x={spyX} y={6} text="RX" color="#47d13a" on={link.rx && known} />
        <FtSpy x={spyX} y={20} text="TX" color={FT.red} on={link.tx} />
        <FtSpy x={spyX} y={34} text="BUSY" color={FT.yellow} on={link.busy} />
        <FtSpy x={spyX} y={48} text="TUNE" color={FT.cyan} on={x.tuner === true} />

        {/* ROS, ALC e potenza in grande, con una riga sottile a sinistra */}
        {power ? (
          <g>
            <rect x={blockX - 5} y={9} width={1} height={46} fill={FT.cyan} opacity={0.35} />
            <Label x={blockX} y={23} text="SWR" fill={FT.ink} size={11} weight={700} />
            <Label x={blockX + FT_POWER_BLOCK_W - 14} y={23} text={swrText} fill={swrColor} size={13} weight={700} anchor="end" />
            <Label x={blockX} y={38} text="ALC" fill={FT.ink} size={11} weight={700} />
            <Label x={blockX + FT_POWER_BLOCK_W - 14} y={38} text={readout('ALC').replace(' ', '')} fill={FT.cyan} size={13} weight={700} anchor="end" />
            <Label x={blockX} y={53} text="PWR TX" fill={FT.ink} size={11} weight={700} />
            <Label x={blockX + FT_POWER_BLOCK_W - 14} y={53} text={readout('PO').replace(' ', '')} fill={sending ? FT.amber : FT.cyan} size={13} weight={700} anchor="end" />
          </g>
        ) : null}

        {/* in ricezione l'S-meter con la scala; in trasmissione diventa il misuratore della potenza, e sotto compare la barra del ROS */}
        {sending
          ? null
          : S_TICKS.map((tk) => (
              <Label
                key={tk.label}
                x={R0 + 4 + tk.fraction * 196}
                y={11}
                text={tk.label}
                fill={tk.fraction > S9_FRACTION ? FT.red : FT.ink}
                size={7.5}
                weight={600}
                anchor="middle"
              />
            ))}
        <SegBar
          x={R0}
          y={14}
          count={40}
          segW={3.6}
          segH={8}
          step={5}
          lit={main.fraction === null ? 0 : Math.round(main.fraction * 40)}
          color={(i) => (sending ? FT.amber : i >= 40 * S9_FRACTION ? FT.red : FT.cyan)}
          off="#0e2630"
        />
        {/* S-meter stimato dall'audio (la radio non da' il vero): una tilde davanti alla barra, senza la lettura numerica */}
        {!sending && sIsEstimated(model.meters) ? <Label x={R0 - 8} y={21} text="~" fill={FT.amber} size={11} weight={700} /> : null}
        {sending ? (
          <SegBar
            x={R0}
            y={25}
            count={40}
            segW={3.6}
            segH={5}
            step={5}
            lit={swrNow === null ? 0 : Math.round(Math.min(1, Math.max(0, (swrNow - 1) / 2)) * 40)}
            color={() => swrColor}
            off="#0e2630"
          />
        ) : null}

        {/* VFO B: l'etichetta e il modo stanno a sinistra della seconda frequenza */}
        <Label x={R0} y={46} text="VFO-B" fill={FT.cyan} size={9} weight={700} opacity={bHz ? 1 : 0.4} />
        <Label x={R0} y={57} text={x.vfo_b_mode ?? ""} fill={FT.amber} size={9} weight={700} />
        <SevenSegment
          text={formatRadioFrequency(bHz)}
          x={R0 + 42}
          y={33}
          w={14}
          h={26}
          t={3}
          gap={4.5}
          skew={0.08}
          on={FT.cyan}
          glow={bHz ? FT.glow : undefined}
          offOpacity={0.1}
        />
      </DisplayFrame>
      <FaceHits hits={hits} onAction={onAction} waiting={waiting} width={faceW} height={FT_FACE_H} />
    </div>
  );
}

/* ---------- stile IC-7610: due VFO affiancati con S-meter ad arco, colonna di pulsanti a sinistra ---------- */

const IC = {
  white: "#f4f6fa",
  label: "#c7d2e2",
  blue: "#1aa0e8",
  val: "#7fd4ff",
  dim: "#3a4656",
  red: "#ff3a2a",
  green: "#3fe07a",
  panel: "#05080d",
  edge: "#1d2a3a",
};

function IcArc({
  cx,
  cy,
  r,
  fraction,
  lit,
  label,
}: {
  cx: number;
  cy: number;
  r: number;
  fraction: number | null;
  lit: boolean;
  label: string;
}) {
  const a0 = 150;
  const span = 120;
  const a9 = a0 - span * S9_FRACTION;
  const needle =
    fraction === null ? null : a0 - span * Math.max(0, Math.min(1, fraction));
  return (
    <g aria-hidden="true" opacity={lit ? 1 : 0.35}>
      <path
        d={arcPath(cx, cy, r, a0, a9)}
        stroke={IC.white}
        strokeWidth={2}
        fill="none"
      />
      <path
        d={arcPath(cx, cy, r, a9, a0 - span)}
        stroke={IC.red}
        strokeWidth={3}
        fill="none"
      />
      {S_TICKS.map((tk) => {
        const a = a0 - span * tk.fraction;
        const [x1, y1] = polar(cx, cy, r, a);
        const [x2, y2] = polar(cx, cy, r + 4, a);
        const [tx, ty] = polar(cx, cy, r + 11, a);
        const col = tk.fraction > S9_FRACTION ? IC.red : IC.white;
        return (
          <g key={tk.label}>
            <line
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke={col}
              strokeWidth={1.2}
            />
            <text
              x={tx}
              y={ty + 3}
              textAnchor="middle"
              fontSize={7.5}
              fontWeight={600}
              fill={col}
              fontFamily={FONT_UI}
            >
              {tk.label}
            </text>
          </g>
        );
      })}
      <text
        x={cx - r + 2}
        y={cy - 4}
        fontSize={8}
        fontWeight={700}
        fill={IC.label}
        fontFamily={FONT_UI}
      >
        {label}
      </text>
      {needle !== null ? (
        <line
          x1={cx}
          y1={cy}
          x2={polar(cx, cy, r + 2, needle)[0]}
          y2={polar(cx, cy, r + 2, needle)[1]}
          stroke={IC.white}
          strokeWidth={1.6}
          strokeLinecap="round"
        />
      ) : null}
    </g>
  );
}

function IcTag({
  x,
  y,
  w,
  text,
  fill,
  fg,
  on = true,
}: {
  x: number;
  y: number;
  w: number;
  text: string;
  fill: string;
  fg: string;
  on?: boolean;
}) {
  return (
    <g aria-hidden="true" opacity={on ? 1 : 0.3}>
      <SpyTip text={text} />
      <rect
        x={x}
        y={y}
        width={w}
        height={13}
        rx={2}
        fill={on ? fill : "none"}
        stroke={fill}
        strokeWidth={1}
      />
      <text
        x={x + w / 2}
        y={y + 9.8}
        textAnchor="middle"
        fontSize={8.5}
        fontWeight={700}
        fontFamily={FONT_UI}
        fill={on ? fg : fill}
      >
        {text}
      </text>
    </g>
  );
}

export function Ic7610Display({
  model,
  noRadio: _noRadio,
  ariaLabel,
  data,
  onAction,
  waiting,
}: FaceProps) {
  const { extras: x } = data ?? NO_EXTRAS;
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);

  const MAIN_X = 8;
  const MAIN_W = 408;
  const SUB_X = 424;
  const SUB_W = 252;
  const bHz = x.vfo_b_hz ?? 0;
  const hits: Hit[] = [{ x: MAIN_X + 148, y: 7, w: 40, h: 15, text: "TUNE" }];

  const subHas = bHz > 0;
  const subMode = x.vfo_b_mode ?? "";
  const reading = mainReading(model, main);
  return (
    <div className="rd-face rd-face--ic">
      <DisplayFrame
        width={IC_FACE_W}
        height={FACE_H}
        radius={10}
        inset={3}
        screen="#000000"
        ariaLabel={ariaLabel}
      >
        {/* VFO A (MAIN): stati RX/TX/BUSY, TUNE, modo, frequenza e S-meter ad arco (in trasmissione la potenza) */}
        <rect x={MAIN_X} y={6} width={MAIN_W} height={84} rx={3} fill={IC.panel} stroke={IC.edge} />
        <IcTag
          x={MAIN_X + 4}
          y={8}
          w={32}
          text="MAIN"
          fill={link.tx ? IC.red : IC.blue}
          fg="#02060c"
          on={known}
        />
        <IcTag x={MAIN_X + 40} y={8} w={28} text="RX" fill={IC.green} fg="#02140a" on={link.rx && known} />
        <IcTag x={MAIN_X + 72} y={8} w={28} text="TX" fill={IC.red} fg="#ffffff" on={link.tx} />
        <IcTag x={MAIN_X + 104} y={8} w={40} text="BUSY" fill={IC.blue} fg="#02060c" on={link.busy} />
        <IcTag x={MAIN_X + 148} y={8} w={40} text="TUNE" fill={IC.green} fg="#02140a" on={x.tuner === true} />
        <IcTag
          x={MAIN_X + 4}
          y={30}
          w={40}
          text={model.mode || "—"}
          fill={IC.blue}
          fg="#02060c"
          on={known && model.mode !== ""}
        />
        <Label
          x={MAIN_X + 52}
          y={known ? 66 : 62}
          text={formatRadioFrequency(known ? model.dialHz : 0)}
          fill={known ? IC.white : IC.dim}
          size={known ? 34 : 30}
          weight={700}
        />
        <IcArc
          cx={MAIN_X + 340}
          cy={86}
          r={44}
          fraction={main.fraction}
          lit={known}
          label={main.kind === "PO" ? "Po" : "S"}
        />
        {reading ? (
          <Label x={MAIN_X + 402} y={18} text={reading} fill={IC.label} size={9} weight={700} anchor="end" />
        ) : null}

        {/* VFO B (SUB): solo frequenza e modo, se la radio li da' */}
        <rect x={SUB_X} y={6} width={SUB_W} height={84} rx={3} fill={IC.panel} stroke={IC.edge} />
        <IcTag x={SUB_X + 4} y={8} w={32} text="SUB" fill={IC.blue} fg="#02060c" on={subHas} />
        <IcTag
          x={SUB_X + 4}
          y={30}
          w={40}
          text={subMode || "—"}
          fill={IC.blue}
          fg="#02060c"
          on={subHas && subMode !== ""}
        />
        <Label
          x={SUB_X + 52}
          y={subHas ? 66 : 62}
          text={formatRadioFrequency(subHas ? bHz : 0)}
          fill={subHas ? IC.white : IC.dim}
          size={30}
          weight={700}
        />
      </DisplayFrame>
      <FaceHits hits={hits} onAction={onAction} waiting={waiting} width={IC_FACE_W} />
    </div>
  );
}
