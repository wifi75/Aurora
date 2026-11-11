import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getDxcc, getRotator, rotatorGoto, rotatorStop, type RotatorStatus } from '../../api/client';
import { useT } from '../../i18n';
import { bearingDeg, gridCenter, type Point } from '../../lib/geo';
import { compassPoint, longPath } from '../../lib/rotator';
import { browserStorage } from '../../lib/theme';
import { useStatusPick } from '../../state/station';
import { Rose } from '../../pages/RotatorPage';
import { SevenSegment } from '../radio-display/SevenSegment';
import '../../styles/rotator.css';

interface Props {
  /** Il nominativo e il locatore del QSO in corso: da li' si calcola la direzione del DX. */
  call: string;
  grid: string;
}

const CLOSED_KEY = 'aurora.digi.rotor.closed';

/** Gli angoli fissi dei pulsanti rapidi: ogni 45 gradi, e per i rotori che vanno oltre il giro anche quelli oltre il nord (360, 405, 450...). */
function presets(min: number, max: number): number[] {
  const out: number[] = [];
  for (let a = Math.ceil(min / 45) * 45; a <= max; a += 45) out.push(a);
  return out.length > 10 ? out.slice(0, 10) : out;
}

/**
 * La scheda del rotore nelle pagine PSK, RTTY e CW, sempre in vista in fondo alla colonna di destra. Dopo il blocco (bussola, display
 * giallo, GO, STOP e pulsanti a gradi fissi), ma con la lancetta che si muove piano mentre il rotore gira, la zona di sovrapposizione per i rotori
 * che vanno oltre i 360 gradi, la direzione del DX del QSO in corso (percorso corto e lungo) e un clic sulla bussola per puntare li'.
 */
export function RotorCard({ call, grid }: Props) {
  const t = useT();
  const client = useQueryClient();
  const [closed, setClosedState] = useState(() => browserStorage()?.getItem(CLOSED_KEY) === '1');
  const setClosed = (v: boolean) => {
    setClosedState(v);
    try {
      browserStorage()?.setItem(CLOSED_KEY, v ? '1' : '0');
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  const [typed, setTyped] = useState('');
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
  const shown = az === null ? '' : String(Math.round(az));
  const lcd = shown.padStart(4, ' ');
  const target = status?.target ?? null;
  const nudge = (step: number) => go.mutate(Math.min(max, Math.max(min, Math.round(base) + step)));

  const header = (
    <button type="button" className="digi-rotor-head" aria-expanded={!closed} onClick={() => setClosed(!closed)} title={wide ? t('rot.wide', { min, max }) : closed ? t('rot.open') : t('rot.close')}>
      <span className={`digi-rotor-dot${moving ? ' digi-rotor-dot--moving' : connected ? ' digi-rotor-dot--on' : ''}`} aria-hidden="true" />
      <strong>{t('digi.tab.rotor')}</strong>
      <span className="digi-rotor-headval">{az === null ? '—' : `${shown}° ${compassPoint(((az % 360) + 360) % 360)}`}</span>
      <span aria-hidden="true">{closed ? '▸' : '▾'}</span>
    </button>
  );

  return (
    <section className={`digi-rotor${moving ? ' digi-rotor--moving' : ''}${connected ? '' : ' digi-rotor--off'}`} aria-label={t('digi.tab.rotor')}>
      {header}
      {closed ? null : (
        <div className="digi-rotor-body">
          <div className="digi-rotor-dial">
            <Rose card zone={wide ? [min, max - 360] : null} status={status} sp={sp} lp={lp} dxCall={cleanCall} disabled={!connected} onPick={(a) => go.mutate(a)} label={t('rot.rose')} />
            {target !== null ? <span className="digi-rotor-badge">{Math.round(target)}</span> : null}
          </div>
          <div className="digi-rotor-panel">
            <svg className="digi-rotor-lcd" viewBox="0 0 168 70" role="img" aria-label={t('rot.heading')}>
              <rect x="0" y="0" width="168" height="70" rx="6" fill="#d8a400" />
              <rect x="3" y="3" width="162" height="64" rx="4" fill="#e0ac00" />
              <SevenSegment text={lcd} x={14} y={10} w={28} h={50} t={5} gap={9} skew={0.1} on="#3b2a00" offOpacity={0.1} />
            </svg>
            <form
              className="digi-rotor-go"
              onSubmit={(e) => {
                e.preventDefault();
                const n = Number(typed.replace(',', '.'));
                if (typed.trim() !== '' && Number.isFinite(n)) go.mutate(n);
              }}
            >
              <button type="button" className="digi-rotor-btn" disabled={!connected} onClick={() => nudge(-10)} title="−10°">
                −10
              </button>
              <input type="number" inputMode="numeric" min={min} max={max} step={1} value={typed} placeholder={String(Math.round(base))} aria-label={t('rot.degrees')} onChange={(e) => setTyped(e.target.value)} />
              <button type="button" className="digi-rotor-btn" disabled={!connected} onClick={() => nudge(10)} title="+10°">
                +10
              </button>
              <button type="submit" className="digi-rotor-btn digi-rotor-btn--go" disabled={!connected || typed.trim() === ''}>
                {t('rot.go')}
              </button>
              <button type="button" className="digi-rotor-btn digi-rotor-btn--stop" disabled={!connected} onClick={() => stop.mutate()}>
                {t('rot.stop')}
              </button>
            </form>
            <div className="digi-rotor-presets" role="group" aria-label={t('rot.steps')}>
              {presets(min, max).map((a) => (
                <button key={a} type="button" className="digi-rotor-btn" disabled={!connected} onClick={() => go.mutate(a)}>
                  {a}
                </button>
              ))}
            </div>
            <div className="digi-rotor-dx">
              <button type="button" className="digi-rotor-btn digi-rotor-btn--dx" disabled={!connected || sp === null} onClick={() => sp !== null && go.mutate(sp)} title={t('rot.sp')}>
                {cleanCall || 'DX'} {t('rot.sp')} {sp !== null ? `${sp}°` : '—'}
              </button>
              <button type="button" className="digi-rotor-btn digi-rotor-btn--lp" disabled={!connected || lp === null} onClick={() => lp !== null && go.mutate(lp)} title={t('rot.lp')}>
                {t('rot.lp')} {lp !== null ? `${lp}°` : '—'}
              </button>
            </div>
          </div>
          {!snap.data?.config.enabled || status?.state === 'off' ? (
            <p className="digi-rotor-note muted">
              {t('rot.offText')} <a href="#/impostazioni/rotator">{t('rot.openSettings')}</a>
            </p>
          ) : null}
          {problemText ? <p className="digi-rotor-note notice" role="alert">{problemText}</p> : null}
        </div>
      )}
    </section>
  );
}
