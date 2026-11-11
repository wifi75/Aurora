import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { ApiError, getRotator, rotatorGoto, rotatorStop, type RotatorStatus } from '../api/client';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { useT, type T } from '../i18n';
import { useGroups } from '../lib/detached';
import { REMOTE_HOST } from '../lib/phone';
import { azimuthOfPoint, compassPoint, longPath, normalizeAz, nudge, shortPath, unwrapAngle } from '../lib/rotator';
import { useStatusPick } from '../state/station';
import '../styles/rotator.css';

/** La pagina Rotore: rosa dei venti grande, lettura dell'antenna e della destinazione, pulsanti con il testo. */
export function RotatorPage({ solo = false }: { solo?: boolean } = {}) {
  const groups = useGroups();
  const elsewhere = groups.some((g) => g.open && g.items.includes('rotator'));
  if (elsewhere && !solo) return <DetachedNote item="rotator" />;
  return <RotatorBody solo={solo} />;
}

function errorText(e: unknown, t: T): string {
  if (e instanceof ApiError && e.code === 'not_connected') return t('rot.err.notConnected');
  if (e instanceof ApiError && e.detail) return e.detail;
  return t('rot.err.failed');
}

const fmt = (az: number | null | undefined) => (az === null || az === undefined ? '—' : `${Math.round(normalizeAz(az))}°`);

function RotatorBody({ solo }: { solo: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['rotator'], queryFn: getRotator, retry: false, refetchInterval: 1000 });
  const station = useStatusPick(['de_grid', 'dx_call', 'dx_grid']);
  const [problem, setProblem] = useState('');
  const [typed, setTyped] = useState('');

  const status = snap.data?.status;
  const connected = status?.connected === true;
  const sp = shortPath(station?.de_grid, station?.dx_grid);
  const lp = sp === null ? null : longPath(sp);
  const dxCall = station?.dx_call ?? '';

  const refresh = (s: RotatorStatus) => {
    setProblem('');
    client.setQueryData(['rotator'], (old: typeof snap.data) => (old ? { ...old, status: s } : old));
  };
  const go = useMutation({ mutationFn: (az: number) => rotatorGoto(normalizeAz(az)), onSuccess: refresh, onError: (e) => setProblem(errorText(e, t)) });
  const stop = useMutation({ mutationFn: rotatorStop, onSuccess: refresh, onError: (e) => setProblem(errorText(e, t)) });
  const base = status?.target ?? status?.az ?? 0;

  const stateKey = !status ? 'rot.state.off' : status.state === 'connected' ? (status.moving ? 'rot.state.moving' : 'rot.state.connected') : (`rot.state.${status.state}` as const);
  const stateClass = !status ? 'off' : status.state === 'connected' ? (status.moving ? 'moving' : 'on') : status.state;

  return (
    <div className={`rot-page${solo ? ' rot-page--solo' : ''}`}>
      <header className="rot-head">
        <h1>{t('rot.title')}</h1>
        <span className={`rot-badge rot-badge--${stateClass}`} role="status">
          <span className="rot-dot" aria-hidden="true" />
          {t(stateKey)}
        </span>
        {solo ? null : <DetachButton item="rotator" />}
      </header>

      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {status && status.state === 'off' ? (
        <div className="notice rot-off">
          <p>{t('rot.offText')}</p>
          {REMOTE_HOST ? null : (
            <a className="glass rot-btn" href="#/impostazioni/rotator">
              {t('rot.openSettings')}
            </a>
          )}
        </div>
      ) : null}
      {status?.state === 'error' && status.error ? <p className="notice rot-error" role="alert">{status.error}</p> : null}
      {problem ? <p className="notice rot-error" role="alert">{problem}</p> : null}

      <div className="rot-grid">
        <Rose status={status} sp={sp} lp={lp} dxCall={dxCall} disabled={!connected} onPick={(az) => go.mutate(az)} label={t('rot.rose')} />

        <section className="rot-side">
          <div className="rot-readout">
            <div className="rot-read rot-read--now">
              <span className="rot-read-label">{t('rot.heading')}</span>
              <span className="rot-read-value">{fmt(status?.az)}</span>
              <span className="rot-read-sub">{status?.az === null || status?.az === undefined ? '' : compassPoint(status.az)}</span>
            </div>
            <div className="rot-read rot-read--target">
              <span className="rot-read-label">{t('rot.target')}</span>
              <span className="rot-read-value">{fmt(status?.target)}</span>
              <span className="rot-read-sub">{status?.target === null || status?.target === undefined ? '' : compassPoint(status.target)}</span>
            </div>
          </div>

          <div className="rot-dx">
            <span className="rot-read-label">{t('rot.dx')}</span>
            {sp !== null ? (
              <span className="rot-dx-line">
                <strong className="data">{dxCall}</strong>
                <span className="data">{station?.dx_grid}</span>
                <span className="rot-dx-path rot-dx-path--sp">
                  {t('rot.sp')} {sp}°
                </span>
                <span className="rot-dx-path rot-dx-path--lp">
                  {t('rot.lp')} {lp}°
                </span>
              </span>
            ) : (
              <span className="muted">{station?.de_grid ? t('rot.noDx') : t('rot.noMyGrid')}</span>
            )}
          </div>

          <div className="rot-actions">
            <button type="button" className="glass rot-btn rot-btn--main" disabled={!connected || sp === null} onClick={() => sp !== null && go.mutate(sp)}>
              {t('rot.short')}
              {sp !== null ? <span className="rot-btn-val">{sp}°</span> : null}
            </button>
            <button type="button" className="glass rot-btn" disabled={!connected || lp === null} onClick={() => lp !== null && go.mutate(lp)}>
              {t('rot.long')}
              {lp !== null ? <span className="rot-btn-val">{lp}°</span> : null}
            </button>
            <button type="button" className="rot-btn rot-btn--stop" disabled={!connected} onClick={() => stop.mutate()}>
              {t('rot.stop')}
            </button>
          </div>

          <div className="rot-steps" role="group" aria-label={t('rot.steps')}>
            {[-10, -2, 2, 10].map((step) => (
              <button key={step} type="button" className="glass rot-btn rot-btn--step" disabled={!connected} onClick={() => go.mutate(nudge(base, step))}>
                {step > 0 ? `+${step}°` : `−${-step}°`}
              </button>
            ))}
          </div>

          <form
            className="rot-manual"
            onSubmit={(e) => {
              e.preventDefault();
              const n = Number(typed.replace(',', '.'));
              if (typed.trim() !== '' && Number.isFinite(n)) go.mutate(n);
            }}
          >
            <label>
              <span className="rot-read-label">{t('rot.degrees')}</span>
              <input type="number" inputMode="numeric" min={0} max={359} step={1} value={typed} onChange={(e) => setTyped(e.target.value)} />
            </label>
            <button type="submit" className="glass rot-btn" disabled={!connected || typed.trim() === ''}>
              {t('rot.go')}
            </button>
          </form>
          <p className="muted rot-hint">{t('rot.clickHint')}</p>
        </section>
      </div>
    </div>
  );
}

const R = 100;

/** Un punto sulla rosa a `r` dal centro nella direzione `az`. */
function polar(az: number, r: number): { x: number; y: number } {
  const a = (az * Math.PI) / 180;
  return { x: Math.sin(a) * r, y: -Math.cos(a) * r };
}

/** L'angolo da disegnare: segue il giro piu' corto rispetto a quello mostrato prima, cosi' passando dal nord l'ago non fa il giro intero. */
function useUnwrapped(az: number | null | undefined): number | null {
  const last = useRef<number | null>(null);
  if (az === null || az === undefined) return last.current;
  last.current = last.current === null ? az : unwrapAngle(last.current, az);
  return last.current;
}

/** Le otto direzioni della bussola a scheda (N, NE, E...). */
const EIGHT = [['N', 0], ['NE', 45], ['E', 90], ['SE', 135], ['S', 180], ['SW', 225], ['W', 270], ['NW', 315]] as const;

/** `card`: la bussola scura della scheda nelle pagine PSK, RTTY e CW (lettere a otto direzioni, una tacca ogni 5 gradi, mozzo metallico). */
export function Rose({ status, sp, lp, dxCall, disabled, onPick, label, card = false, zone = null }: { status: RotatorStatus | undefined; sp: number | null; lp: number | null; dxCall: string; disabled: boolean; onPick(az: number): void; label: string; card?: boolean; zone?: readonly [number, number] | null }) {
  const now = useUnwrapped(status?.az);
  const target = useUnwrapped(status?.target);
  const pick = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (disabled) return;
    const box = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - (box.left + box.width / 2);
    const y = e.clientY - (box.top + box.height / 2);
    if (Math.hypot(x, y) < box.width * 0.06) return;
    onPick(azimuthOfPoint(x, y));
  };
  const ticks = Array.from({ length: card ? 72 : 36 }, (_, i) => i * (card ? 5 : 10));
  return (
    <div className="rot-rose-box">
      <svg className={`rot-rose${disabled ? ' rot-rose--off' : ''}${card ? ' rot-rose--card' : ''}`} viewBox="-118 -118 236 236" role="img" aria-label={label} onPointerDown={pick}>
        <defs>
          <radialGradient id="rot-hub-grad" cx="35%" cy="30%" r="75%">
            <stop offset="0" stopColor="#f4f4f4" />
            <stop offset="0.55" stopColor="#8d8d8d" />
            <stop offset="1" stopColor="#3a3a3a" />
          </radialGradient>
        </defs>
        <circle className="rot-face" r={R} />
        <circle className="rot-ring" r={R} />
        <circle className="rot-inner" r={R * 0.62} />
        {zone ? <OverlapArc from={zone[0]} to={zone[1]} /> : null}
        {ticks.map((az) => {
          const major = card ? az % 45 === 0 : az % 30 === 0;
          const a = polar(az, R);
          const b = polar(az, major ? R - 10 : R - 5);
          return <line key={az} className={major ? 'rot-tick rot-tick--major' : 'rot-tick'} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
        })}
        {card
          ? EIGHT.map(([name, az]) => {
              const q = polar(az, R - 22);
              return (
                <text key={az} className={az % 90 === 0 ? 'rot-label rot-label--card' : 'rot-label rot-label--eight'} x={q.x} y={q.y} textAnchor="middle" dominantBaseline="central">
                  {name}
                </text>
              );
            })
          : null}
        {(card ? [] : ticks.filter((az) => az % 30 === 0)).map((az) => {
            const p = polar(az, R - 20);
            const cardinal = az % 90 === 0;
            const text = cardinal ? ({ 0: 'N', 90: 'E', 180: 'S', 270: 'W' } as Record<number, string>)[az] : String(az);
            return (
              <text key={az} className={cardinal ? 'rot-label rot-label--card' : 'rot-label'} x={p.x} y={p.y} textAnchor="middle" dominantBaseline="central">
                {text}
              </text>
            );
          })}
        {lp !== null ? <DxMark az={lp} text="LP" kind="lp" /> : null}
        {sp !== null ? <DxMark az={sp} text={dxCall || 'SP'} kind="sp" /> : null}
        {target !== null && status?.target !== null && status?.target !== undefined ? (
          <g className="rot-needle rot-needle--target" style={{ transform: `rotate(${target}deg)` }}>
            <line x1={0} y1={0} x2={0} y2={-(R - 6)} />
            <circle cx={0} cy={-(R - 6)} r={4} />
          </g>
        ) : null}
        {now !== null ? (
          <g className="rot-needle rot-needle--now" style={{ transform: `rotate(${now}deg)` }}>
            <path d={`M0 ${-(R - 8)} L9 -52 L3.2 -52 L3.2 14 L-3.2 14 L-3.2 -52 L-9 -52 Z`} />
          </g>
        ) : null}
        <circle className="rot-hub" r={7} />
      </svg>
    </div>
  );
}

/** La zona di sovrapposizione di un rotore che va oltre i 360 gradi: da li' si arriva da due angoli diversi. */
function OverlapArc({ from, to }: { from: number; to: number }) {
  const r = R - 4;
  const a = polar(from, r);
  const b = polar(to, r);
  const large = to - from > 180 ? 1 : 0;
  return <path className="rot-overlap" d={`M ${a.x} ${a.y} A ${r} ${r} 0 ${large} 1 ${b.x} ${b.y}`} />;
}

function DxMark({ az, text, kind }: { az: number; text: string; kind: 'sp' | 'lp' }) {
  const dot = polar(az, R + 7);
  const from = polar(az, R * 0.62);
  const to = polar(az, R);
  // l'etichetta sta dentro il cerchio interno, sulla stessa direzione: non si sovrappone ai numeri della scala
  const at = polar(az, R * 0.45);
  return (
    <g className={`rot-dxmark rot-dxmark--${kind}`}>
      <line x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
      <circle cx={dot.x} cy={dot.y} r={5} />
      <text x={at.x} y={at.y} textAnchor="middle" dominantBaseline="central">
        {text}
      </text>
    </g>
  );
}
