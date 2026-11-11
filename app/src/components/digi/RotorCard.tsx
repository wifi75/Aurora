import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { ApiError, getDxcc, getRotator, rotatorGoto, rotatorStop, type RotatorStatus } from '../../api/client';
import { useT } from '../../i18n';
import { bearingDeg, gridCenter, type Point } from '../../lib/geo';
import { azimuthOfPoint, longPath, unwrapAngle } from '../../lib/rotator';
import { browserStorage } from '../../lib/theme';
import { useStatusPick } from '../../state/station';
import { SevenSegment } from '../radio-display/SevenSegment';

interface Props {
  /** Il nominativo e il locatore del QSO in corso: da li' si calcola la direzione del DX. */
  call: string;
  grid: string;
}

const CLOSED_KEY = 'aurora.digi.rotor.closed';
const POS_KEY = 'aurora.digi.rotor.pos';
const HIDDEN_KEY = 'aurora.digi.rotor.hidden';
const WIDTH_KEY = 'aurora.digi.rotor.w';
const PARK_KEY = 'aurora.digi.rotor.park';
const EVENT = 'aurora-rotor-card';
const MIN_W = 240;
const MAX_W = 760;

interface Pos {
  x: number;
  y: number;
}

function readPos(): Pos | null {
  try {
    const v = JSON.parse(browserStorage()?.getItem(POS_KEY) ?? 'null') as Partial<Pos> | null;
    return v && Number.isFinite(v.x) && Number.isFinite(v.y) ? { x: Number(v.x), y: Number(v.y) } : null;
  } catch {
    return null;
  }
}

function remember(key: string, value: string | null): void {
  try {
    if (value === null) browserStorage()?.removeItem(key);
    else browserStorage()?.setItem(key, value);
  } catch {
    // la scelta vale comunque finche' la pagina e' aperta
  }
}

/** La scheda e' mostrata? Si accende e si spegne dal menu Vista. */
export function useRotorCardVisible(): [boolean, (v: boolean) => void] {
  const read = () => browserStorage()?.getItem(HIDDEN_KEY) !== '1';
  const [shown, setShown] = useState(read);
  useEffect(() => {
    const sync = () => setShown(read());
    window.addEventListener(EVENT, sync);
    window.addEventListener('storage', sync);
    return () => {
      window.removeEventListener(EVENT, sync);
      window.removeEventListener('storage', sync);
    };
  }, []);
  return [
    shown,
    (v) => {
      remember(HIDDEN_KEY, v ? '0' : '1');
      window.dispatchEvent(new Event(EVENT));
    },
  ];
}

type Preset = number | 'dx' | 'park';

/** I pulsanti: ogni 45 gradi; dopo i 315 il 360 (per i rotori che vanno oltre il giro) oppure il DX, e infine il Park. */
function presets(max: number, wide: boolean): Preset[] {
  const out: Preset[] = [0, 45, 90, 135, 180, 225, 270, 315];
  out.push(wide && max >= 360 ? 360 : 'dx');
  out.push('park');
  return out;
}

const R = 108;
const polar = (az: number, r: number) => {
  const a = (az * Math.PI) / 180;
  return { x: Math.sin(a) * r, y: -Math.cos(a) * r };
};
const EIGHT = [['N', 0], ['NE', 45], ['E', 90], ['SE', 135], ['S', 180], ['SW', 225], ['W', 270], ['NW', 315]] as const;

/** La lancetta segue il giro piu' corto da dove stava, cosi' passando dal nord non fa il giro intero. */
function useUnwrapped(az: number | null | undefined): number | null {
  const last = useRef<number | null>(null);
  if (az === null || az === undefined) return last.current;
  last.current = last.current === null ? az : unwrapAngle(last.current, az);
  return last.current;
}

/** La zona di sovrapposizione di un rotore che va oltre i 360 gradi: da li' si arriva da due angoli diversi. */
function ZoneArc({ from, to }: { from: number; to: number }) {
  const r = R - 1;
  const a = polar(from, r);
  const b = polar(to, r);
  return <path d={`M ${a.x} ${a.y} A ${r} ${r} 0 ${to - from > 180 ? 1 : 0} 1 ${b.x} ${b.y}`} fill="none" stroke="#ffb41f" strokeWidth={4} opacity={0.85} />;
}

interface DialProps {
  status: RotatorStatus | undefined;
  sp: number | null;
  lp: number | null;
  zone: readonly [number, number] | null;
  label: string;
  title: string;
  disabled: boolean;
  onPick(az: number): void;
}

/** La bussola: faccia nera, tacche fini, anello chiaro, lancetta arancio-gialla, mozzo metallico e la targhetta gialla con il bersaglio. */
function Dial({ status, sp, lp, zone, label, title, disabled, onPick }: DialProps) {
  const now = useUnwrapped(status?.az);
  const target = status?.target ?? null;
  const pick = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (disabled) return;
    const box = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - (box.left + box.width / 2);
    const y = e.clientY - (box.top + box.height / 2);
    if (Math.hypot(x, y) < box.width * 0.07) return;
    onPick(azimuthOfPoint(x, y));
  };
  const ticks = Array.from({ length: 120 }, (_, i) => i * 3);
  const shownTarget = target ?? status?.az ?? null;
  return (
    <svg className={`rotor-dial${disabled ? ' rotor-dial--off' : ''}`} viewBox="-120 -120 240 240" role="img" aria-label={label} onPointerDown={pick}>
      <title>{title}</title>
      <defs>
        <linearGradient id="rotor-ring-grad" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#d4d4d4" />
          <stop offset="0.5" stopColor="#7d7d7d" />
          <stop offset="1" stopColor="#b8b8b8" />
        </linearGradient>
        <radialGradient id="rotor-hub-grad" cx="38%" cy="30%" r="80%">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="0.45" stopColor="#9d9d9d" />
          <stop offset="1" stopColor="#3b3b3b" />
        </radialGradient>
        <linearGradient id="rotor-needle-grad" gradientUnits="userSpaceOnUse" x1="0" y1="-96" x2="0" y2="24">
          <stop offset="0" stopColor="#ff8f6b" />
          <stop offset="0.45" stopColor="#d9b400" />
          <stop offset="1" stopColor="#e8d400" />
        </linearGradient>
      </defs>
      <circle r={R + 4} fill="#1a1a1a" />
      <circle r={R} fill="#161616" stroke="url(#rotor-ring-grad)" strokeWidth={7} />
      {zone ? <ZoneArc from={zone[0]} to={zone[1]} /> : null}
      {ticks.map((az) => {
        const big = az % 45 === 0;
        const mid = az % 15 === 0;
        const a = polar(az, R - 5);
        const b = polar(az, R - (big ? 17 : mid ? 13 : 9));
        return <line key={az} x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke={big ? '#f2f2f2' : mid ? '#c9c9c9' : '#9a9a9a'} strokeWidth={big ? 2.4 : mid ? 1.2 : 0.8} />;
      })}
      {EIGHT.map(([name, az]) => {
        const p = polar(az, R - 36);
        return (
          <text key={name} x={p.x} y={p.y} textAnchor="middle" dominantBaseline="central" fill="#d6d6d6" fontSize={az % 90 === 0 ? 17 : 15} fontFamily="var(--font-ui)">
            {name}
          </text>
        );
      })}
      <text x={0} y={-26} textAnchor="middle" fill="#c4c4c4" fontSize={17} fontFamily="var(--font-ui)">
        {label}
      </text>
      {lp !== null ? <circle cx={polar(lp, R + 1).x} cy={polar(lp, R + 1).y} r={5} fill="#b8a1ff" stroke="#161616" strokeWidth={1.5} /> : null}
      {sp !== null ? <circle cx={polar(sp, R + 1).x} cy={polar(sp, R + 1).y} r={5.5} fill="#34d399" stroke="#161616" strokeWidth={1.5} /> : null}
      {target !== null ? <circle cx={polar(target, R - 20).x} cy={polar(target, R - 20).y} r={3.4} fill="#ffb41f" /> : null}
      {now !== null ? (
        <g className="rotor-needle" style={{ transform: `rotate(${now}deg)` }}>
          <path d="M0 -97 L3.4 -2 L1.7 24 L-1.7 24 L-3.4 -2 Z" fill="url(#rotor-needle-grad)" stroke="#6b5a00" strokeWidth={0.6} strokeLinejoin="round" />
        </g>
      ) : null}
      <circle r={14} fill="url(#rotor-hub-grad)" stroke="#262626" strokeWidth={1.5} />
      {shownTarget !== null ? (
        <g transform="translate(0 83)">
          <rect x={-23} y={-12} width={46} height={24} rx={6} fill="#d4a900" stroke="#8c8c8c" strokeWidth={3.2} />
          <text x={0} y={1} textAnchor="middle" dominantBaseline="central" fill="#2a1d00" fontSize={17} fontWeight={600} fontFamily="var(--font-ui)">
            {Math.round(shownTarget)}
          </text>
        </g>
      ) : null}
    </svg>
  );
}

/**
 * La scheda del rotore nelle pagine PSK, RTTY e CW: come il blocco (bussola nera, striscia gialla con le cifre a sette segmenti, GO e STOP,
 * griglia di pulsanti) ma con la lancetta che si muove piano mentre il rotore gira, la zona di sovrapposizione per i rotori oltre i 360 gradi e,
 * sotto l'ingranaggio, i comandi in piu': gradi a mano, -10/+10, DX corto e lungo, Park. Si sposta e si ridimensiona; e' un riquadro a parte.
 */
export function RotorCard({ call, grid }: Props) {
  const t = useT();
  const client = useQueryClient();
  const [closed, setClosedState] = useState(() => browserStorage()?.getItem(CLOSED_KEY) === '1');
  const setClosed = (v: boolean) => {
    setClosedState(v);
    remember(CLOSED_KEY, v ? '1' : '0');
  };
  const [typed, setTyped] = useState('');
  const [more, setMore] = useState(false);
  const card = useRef<HTMLElement>(null);
  const [pos, setPos] = useState<Pos | null>(readPos);
  const [width, setWidth] = useState<number | null>(() => {
    const v = Number(browserStorage()?.getItem(WIDTH_KEY));
    return Number.isFinite(v) && v >= MIN_W && v <= MAX_W ? v : null;
  });
  const [parkAz, setParkAz] = useState(() => {
    const v = Number(browserStorage()?.getItem(PARK_KEY));
    return Number.isFinite(v) ? v : 0;
  });
  const [visible, setVisible] = useRotorCardVisible();

  // mentre il rotore gira si legge piu' spesso, cosi' la lancetta si muove di continuo
  const snap = useQuery({ queryKey: ['rotator'], queryFn: getRotator, retry: false, refetchInterval: (q) => (q.state.data?.status.moving ? 400 : 1000) });
  const station = useStatusPick(['de_grid']);
  const status = snap.data?.status;
  const config = snap.data?.config;
  const min = config?.min_az ?? 0;
  const max = config?.max_az ?? 360;
  const wide = max - min > 360;
  const connected = status?.connected === true;
  const moving = status?.moving === true;
  const cleanCall = call.trim().toUpperCase();

  // la direzione del DX: dal suo locatore, altrimenti dal centro del suo paese
  const dxcc = useQuery({ queryKey: ['dxcc', cleanCall], queryFn: () => getDxcc(cleanCall), enabled: cleanCall.length >= 3 && grid.trim() === '', staleTime: 10 * 60_000, retry: false });
  const from = station?.de_grid ? gridCenter(station.de_grid) : null;
  const to: Point | null = gridCenter(grid) ?? (typeof dxcc.data?.lat === 'number' && typeof dxcc.data.lon === 'number' ? { lat: dxcc.data.lat, lon: dxcc.data.lon } : null);
  const sp = from && to ? bearingDeg(from, to) : null;
  const lp = sp === null ? null : longPath(sp);

  const refresh = (s: RotatorStatus) => client.setQueryData(['rotator'], (old: typeof snap.data) => (old ? { ...old, status: s } : old));
  // l'angolo si manda cosi' com'e': e' il core a scegliere, dentro il campo del rotore, quello piu' vicino
  const go = useMutation({ mutationFn: (az: number) => rotatorGoto(az), onSuccess: refresh });
  const stop = useMutation({ mutationFn: rotatorStop, onSuccess: refresh });
  const base = status?.target ?? status?.az ?? 0;
  const problem = go.error ?? stop.error;
  const problemText = problem instanceof ApiError && problem.code === 'not_connected' ? t('rot.err.notConnected') : problem ? t('rot.err.failed') : '';
  const az = status?.az ?? null;
  // tre cifre (quattro solo con il segno meno o oltre il 999): cosi' le cifre restano grandi
  const lcdText = az === null ? '' : String(Math.round(az));
  const lcd = lcdText.padStart(lcdText.length > 3 ? lcdText.length : 3, ' ');
  const nudge = (step: number) => go.mutate(Math.min(max, Math.max(min, Math.round(base) + step)));
  const typedNumber = Number(typed.replace(',', '.'));
  const typedOk = typed.trim() !== '' && Number.isFinite(typedNumber);
  const help = `${t('rot.clickHint')}${wide ? ` ${t('rot.wide', { min, max })}` : ''}`;

  // se la finestra si rimpicciolisce e la scheda resta fuori, torna al suo posto
  useEffect(() => {
    const fit = () => {
      if (pos && (pos.x + 40 > window.innerWidth || pos.y + 30 > window.innerHeight)) setPos(null);
    };
    window.addEventListener('resize', fit);
    return () => window.removeEventListener('resize', fit);
  }, [pos]);

  /** Si sposta trascinando la barra del titolo; con il doppio clic torna al posto e alla misura di partenza. */
  const startDrag = (e: ReactPointerEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    const el = card.current;
    if (!el) return;
    e.preventDefault();
    const r = el.getBoundingClientRect();
    // con la dimensione del testo cambiata (zoom) le coordinate dello schermo e quelle della pagina non coincidono: si converte
    const scale = el.offsetWidth > 0 ? r.width / el.offsetWidth : 1;
    const start = { px: e.clientX, py: e.clientY, x: r.left / scale, y: r.top / scale };
    let last: Pos = { x: start.x, y: start.y };
    const move = (m: PointerEvent) => {
      const maxX = window.innerWidth / scale - el.offsetWidth;
      const maxY = window.innerHeight / scale - 30;
      last = { x: Math.min(Math.max(0, start.x + (m.clientX - start.px) / scale), Math.max(0, maxX)), y: Math.min(Math.max(0, start.y + (m.clientY - start.py) / scale), Math.max(0, maxY)) };
      setPos(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      remember(POS_KEY, JSON.stringify(last));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const startResize = (e: ReactPointerEvent) => {
    const el = card.current;
    if (!el) return;
    e.preventDefault();
    e.stopPropagation();
    const r = el.getBoundingClientRect();
    const scale = el.offsetWidth > 0 ? r.width / el.offsetWidth : 1;
    const startX = e.clientX;
    const startW = el.offsetWidth;
    let last = startW;
    const move = (m: PointerEvent) => {
      const room = window.innerWidth / scale - (pos?.x ?? r.left / scale) - 4;
      last = Math.round(Math.min(Math.max(MIN_W, startW + (m.clientX - startX) / scale), Math.min(MAX_W, Math.max(MIN_W, room))));
      setWidth(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      remember(WIDTH_KEY, String(last));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const resetPlace = () => {
    setWidth(null);
    remember(WIDTH_KEY, null);
    setPos(null);
    remember(POS_KEY, null);
  };

  if (!visible) return null;

  const press = (p: Preset) => {
    if (p === 'dx') {
      if (sp !== null) go.mutate(sp);
    } else if (p === 'park') go.mutate(parkAz);
    else go.mutate(p);
  };
  const labelOf = (p: Preset) => (p === 'dx' ? 'DX' : p === 'park' ? 'Park' : String(p));

  return (
    <section
      ref={card}
      style={{ ...(pos ? { left: pos.x, top: pos.y, right: 'auto', bottom: 'auto' } : {}), ...(width ? { width } : {}) }}
      className={`rotor-card${moving ? ' rotor-card--moving' : ''}${connected ? '' : ' rotor-card--off'}`}
      aria-label={t('digi.tab.rotor')}
    >
      <div className="rotor-bar" onPointerDown={startDrag} onDoubleClick={resetPlace} title={t('rot.move')}>
        <span className={`rotor-dot${moving ? ' rotor-dot--moving' : connected ? ' rotor-dot--on' : ''}`} aria-hidden="true" />
        <strong>{t('digi.tab.rotor')}</strong>
        <span className="rotor-bar-sp" />
        <button type="button" className="rotor-bar-btn" aria-expanded={!closed} onClick={() => setClosed(!closed)} title={closed ? t('rot.open') : t('rot.close')}>
          {closed ? '▸' : '▾'}
        </button>
        <button type="button" className="rotor-bar-btn" onClick={() => setVisible(false)} title={t('rot.hide')} aria-label={t('rot.hide')}>
          ✕
        </button>
      </div>
      {closed ? null : (
        <>
          <div className="rotor-dial-box">
            <Dial status={status} sp={sp} lp={lp} zone={wide ? [min, max - 360] : null} label={t('digi.tab.rotor')} title={help} disabled={!connected} onPick={(a) => go.mutate(a)} />
          </div>
          <div className="rotor-lcd">
            <button type="button" className="rotor-help" title={help} aria-label="?">
              ?
            </button>
            <svg className="rotor-digits" viewBox={`0 0 ${12 + lcd.length * 39} 64`} role="img" aria-label={t('rot.heading')}>
              <SevenSegment text={lcd} x={6} y={8} w={30} h={48} t={5.5} gap={9} skew={0.14} on="#3b2b00" offOpacity={0.09} />
            </svg>
            <div className="rotor-lcd-btns">
              <button type="button" className="rotor-go" disabled={!connected || !typedOk} onClick={() => typedOk && go.mutate(typedNumber)} title={t('rot.degrees')}>
                {t('rot.go')}
              </button>
              <button type="button" className="rotor-stop" disabled={!connected} onClick={() => stop.mutate()}>
                {t('rot.stop')}
              </button>
            </div>
            <button type="button" className={`rotor-gear${more ? ' rotor-gear--on' : ''}`} aria-expanded={more} onClick={() => setMore(!more)} title={t('rot.more')} aria-label={t('rot.more')}>
              ⚙<span aria-hidden="true">{more ? '▲' : '▼'}</span>
            </button>
          </div>
          {more ? (
            <div className="rotor-more">
              <div className="rotor-more-row">
                <button type="button" disabled={!connected} onClick={() => nudge(-10)}>
                  −10
                </button>
                <input
                  type="number"
                  inputMode="numeric"
                  min={min}
                  max={max}
                  step={1}
                  value={typed}
                  placeholder={String(Math.round(base))}
                  aria-label={t('rot.degrees')}
                  onChange={(e) => setTyped(e.target.value)}
                  onKeyDown={(e) => (e.key === 'Enter' && typedOk && connected ? go.mutate(typedNumber) : undefined)}
                />
                <button type="button" disabled={!connected} onClick={() => nudge(10)}>
                  +10
                </button>
              </div>
              <div className="rotor-more-row">
                <button type="button" disabled={!connected || sp === null} onClick={() => sp !== null && go.mutate(sp)} title={t('rot.sp')}>
                  {cleanCall || 'DX'} · {t('rot.sp')} {sp !== null ? `${sp}°` : '—'}
                </button>
                <button type="button" disabled={!connected || lp === null} onClick={() => lp !== null && go.mutate(lp)} title={t('rot.lp')}>
                  {t('rot.lp')} {lp !== null ? `${lp}°` : '—'}
                </button>
              </div>
              <div className="rotor-more-row">
                <button
                  type="button"
                  disabled={az === null}
                  onClick={() => {
                    if (az === null) return;
                    const v = Math.round(az);
                    setParkAz(v);
                    remember(PARK_KEY, String(v));
                  }}
                >
                  {t('rot.parkSet', { az: parkAz })}
                </button>
              </div>
            </div>
          ) : null}
          <div className="rotor-presets" role="group" aria-label={t('rot.steps')}>
            {presets(max, wide).map((p) => (
              <button key={String(p)} type="button" disabled={!connected || (p === 'dx' && sp === null)} onClick={() => press(p)} title={p === 'park' ? t('rot.parkGo', { az: parkAz }) : undefined}>
                {labelOf(p)}
              </button>
            ))}
          </div>
          {!snap.data?.config.enabled || status?.state === 'off' ? (
            <p className="rotor-note">
              {t('rot.offText')} <a href="#/impostazioni/rotator">{t('rot.openSettings')}</a>
            </p>
          ) : null}
          {problemText ? (
            <p className="rotor-note rotor-note--err" role="alert">
              {problemText}
            </p>
          ) : null}
          <span className="rotor-grip" role="separator" aria-orientation="vertical" title={t('rot.resize')} onPointerDown={startResize} />
        </>
      )}
    </section>
  );
}
