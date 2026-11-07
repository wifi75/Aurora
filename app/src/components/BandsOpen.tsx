import { useQuery } from '@tanstack/react-query';
import { useMemo, memo } from 'react';
import { getOnline, getPsk, getSolar } from '../api/client';
import { useT } from '../i18n';
import { bandOf } from '../lib/format';
import { flagCode } from '../lib/flags';
import { gridCenter } from '../lib/geo';
import { formatDistance, usePrefs } from '../lib/prefs';
import { conditionGroup, pskRows } from '../lib/psk';
import { isNight } from '../lib/terminator';
import { Conditions } from '../pages/PropagationPage';
import { useStatusPick } from '../state/station';

/**
 * «Bande aperte»: le condizioni di propagazione per gruppo di bande (giorno e notte, quella dove sei evidenziata) e, sotto, le tre
 * stazioni piu' lontane che hanno ricevuto il tuo segnale (PSK Reporter). I dati vengono dai servizi online, accesi nelle Impostazioni.
 */
function BandsOpenInner() {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const status = useStatusPick(['de_grid', 'dial_hz']);
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const solar = useQuery({ queryKey: ['online-solar'], queryFn: getSolar, retry: false, refetchInterval: 60_000, enabled: online.data?.solar === true });
  const psk = useQuery({ queryKey: ['online-psk'], queryFn: getPsk, retry: false, refetchInterval: 60_000, enabled: online.data?.psk === true });
  const pskState = psk.data?.psk ?? null;
  const rows = useMemo(() => (pskState ? pskRows(pskState.reports, status?.de_grid) : []), [pskState, status?.de_grid]);
  // le tre piu' lontane, una per stazione ricevente
  const farthest = useMemo(() => {
    const seen = new Set<string>();
    const out: typeof rows = [];
    for (const r of [...rows].sort((a, b) => (b.km ?? -1) - (a.km ?? -1))) {
      if (r.km === null || r.km === undefined || seen.has(r.receiver)) continue;
      seen.add(r.receiver);
      out.push(r);
      if (out.length === 3) break;
    }
    return out;
  }, [rows]);


  if (online.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!online.data) return <p className="muted">…</p>;
  if (!online.data.solar && !online.data.psk) {
    return (
      <div className="bands-open">
        <p className="muted">{t('prop.off')}</p>
        <a className="glass glass--link" href="#/impostazioni/online">{t('prop.settings')}</a>
      </div>
    );
  }
  const data = solar.data?.solar.data ?? null;
  const group = conditionGroup(status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null);
  const home = status?.de_grid ? gridCenter(status.de_grid) : null;
  const nightNow = home ? isNight(home.lat, home.lon, new Date()) : null;
  return (
    <div className="bands-open">
      {data ? <Conditions data={data} group={group} nightNow={nightNow} /> : <p className="muted">{solar.data?.solar.error ?? '…'}</p>}
      <h3 className="bands-open-title">{t('bands.far3')}</h3>
      {farthest.length === 0 ? (
        <p className="muted bands-open-none">{online.data.psk ? t('bands.noPsk') : t('prop.off')}</p>
      ) : (
        <ol className="bands-open-far">
          {farthest.map((r) => (
            <li key={r.receiver} title={r.country ?? undefined}>
              {flagCode(r.country) ? <span className={`fi fi-${flagCode(r.country)}`} aria-hidden="true" /> : <span className="fi" />}
              <b className="data bands-open-call">{r.receiver}</b>
              <span className="bands-open-country">{r.country ?? ''}</span>
              <span className="muted bands-open-band">{r.band ?? ''}</span>
              <span className="data bands-open-km">{formatDistance(r.km ?? 0, prefs.distanceUnit)}</span>
              <span className="data muted bands-open-snr">{typeof r.snr === 'number' && Number.isFinite(r.snr) ? `${r.snr} dB` : ''}</span>
            </li>
          ))}
        </ol>
      )}
      {/* continenti e record di distanza tolti: al loro posto, nella Stazione, i Comandi (scelta del proprietario) */}
    </div>
  );
}

/** Senza proprieta': si ridisegna solo per i suoi dati, non ogni volta che si ridisegna la Stazione. */
export const BandsOpen = memo(BandsOpenInner);
