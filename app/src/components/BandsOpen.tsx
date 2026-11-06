import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { getOnline, getPsk, getSolar } from '../api/client';
import { useT } from '../i18n';
import { bandOf } from '../lib/format';
import { flagCode } from '../lib/flags';
import { gridCenter } from '../lib/geo';
import { formatDistance, usePrefs } from '../lib/prefs';
import { conditionGroup, continentAt, latestPerReceiver, pskRows } from '../lib/psk';
import { browserStorage } from '../lib/theme';
import { isNight } from '../lib/terminator';
import { Conditions } from '../pages/PropagationPage';
import { useStation } from '../state/station';

/**
 * «Bande aperte»: le condizioni di propagazione per gruppo di bande (giorno e notte, quella dove sei evidenziata) e, sotto, le tre
 * stazioni piu' lontane che hanno ricevuto il tuo segnale (PSK Reporter). I dati vengono dai servizi online, accesi nelle Impostazioni.
 */
export function BandsOpen() {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const status = useStation((s) => s.status);
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

  // continenti che ci ascoltano: una stazione ricevente conta una volta, il continente si stima dal suo locatore
  const continents = useMemo(() => {
    const out: Record<string, number> = { EU: 0, NA: 0, SA: 0, AF: 0, AS: 0, OC: 0 };
    for (const r of latestPerReceiver(rows)) {
      const p = gridCenter(r.locator.slice(0, 6)) ?? gridCenter(r.locator.slice(0, 4));
      if (!p) continue;
      const c = continentAt(p.lat, p.lon);
      out[c] = (out[c] ?? 0) + 1;
    }
    return out;
  }, [rows]);
  // record di distanza: il piu' lontano di oggi (solo gli spot di oggi UTC: la finestra di PSK Reporter arriva anche a ieri) e degli
  // ultimi 7 giorni (ricordato giorno per giorno su questo dispositivo)
  const todayBest = useMemo(() => {
    const dayStart = Math.floor(Date.now() / 86_400_000) * 86_400;
    let best: (typeof rows)[number] | null = null;
    for (const r of rows) {
      if (r.km === null || r.km === undefined || r.time_s < dayStart) continue;
      if (!best || r.km > (best.km ?? -1)) best = r;
    }
    return best;
  }, [rows]);
  const records = useMemo(() => recordsFrom(todayBest), [todayBest]);
  useEffect(() => saveRecords(records.kept), [records]);

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
              <b className="data">{r.receiver}</b>
              <span className="muted">{r.band ?? ''}</span>
              <span className="data bands-open-km">{formatDistance(r.km ?? 0, prefs.distanceUnit)}</span>
              <span className="data muted">{r.snr} dB</span>
            </li>
          ))}
        </ol>
      )}
      {online.data.psk ? (
        <>
          <h3 className="bands-open-title">{t('bands.continents')}</h3>
          <div className="bands-open-conts">
            {(['EU', 'NA', 'SA', 'AF', 'AS', 'OC'] as const).map((c) => (
              <div key={c} className={`bands-open-cont${continents[c] ? ' bands-open-cont--on' : ''}`} title={t(`bands.cont.${c}`)}>
                <b>{c}</b>
                <span className="data">{continents[c]}</span>
              </div>
            ))}
          </div>
          <h3 className="bands-open-title">{t('bands.records')}</h3>
          <div className="bands-open-records">
            {[
              { label: t('bands.today'), r: records.today },
              { label: t('bands.week'), r: records.week },
            ].map(({ label, r }) => (
              <div key={label} className="bands-open-record">
                <span className="muted">{label}</span>
                {r ? (
                  <>
                    <b className="data">{r.call}</b>
                    <span className="muted">{r.band}</span>
                    <span className="data bands-open-km">{formatDistance(r.km, prefs.distanceUnit)}</span>
                  </>
                ) : (
                  <span className="muted">—</span>
                )}
              </div>
            ))}
          </div>
        </>
      ) : null}
    </div>
  );
}

interface DistanceRecord {
  call: string;
  band: string;
  km: number;
}

const RECORDS_KEY = 'aurora-record-distanza';

function isRecord(v: unknown): v is DistanceRecord {
  if (typeof v !== 'object' || v === null) return false;
  const r = v as Record<string, unknown>;
  return typeof r.call === 'string' && typeof r.band === 'string' && typeof r.km === 'number' && Number.isFinite(r.km);
}

/** I record salvati giorno per giorno; un archivio rovinato o scritto da un'altra versione vale come vuoto (non deve far cadere la scheda). */
function loadRecords(): Record<string, DistanceRecord> {
  const out: Record<string, DistanceRecord> = {};
  try {
    const raw: unknown = JSON.parse(browserStorage()?.getItem(RECORDS_KEY) ?? '{}');
    if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return out;
    for (const [day, v] of Object.entries(raw)) if (/^\d{4}-\d{2}-\d{2}$/.test(day) && isRecord(v)) out[day] = v;
  } catch {
    // archivio illeggibile: si riparte da zero
  }
  return out;
}

function saveRecords(kept: Record<string, DistanceRecord>): void {
  try {
    browserStorage()?.setItem(RECORDS_KEY, JSON.stringify(kept));
  } catch {
    // senza archivio il record vale finche' la pagina resta aperta
  }
}

/**
 * Il record di oggi e quello degli ultimi 7 giorni di calendario (oggi compreso): ogni giorno si ricorda il piu' lontano visto, su questo
 * dispositivo. Non scrive niente: `kept` e' quello da salvare (lo fa un effetto, fuori dal disegno).
 */
function recordsFrom(best: { receiver: string; band: string | null; km: number | null } | null): {
  today: DistanceRecord | null;
  week: DistanceRecord | null;
  kept: Record<string, DistanceRecord>;
} {
  const day = new Date().toISOString().slice(0, 10);
  const firstDay = new Date(Date.now() - 6 * 86_400_000).toISOString().slice(0, 10);
  const saved = loadRecords();
  const old = saved[day];
  if (best && best.km !== null && (!old || best.km > old.km)) {
    saved[day] = { call: best.receiver, band: best.band ?? '', km: best.km };
  }
  const kept: Record<string, DistanceRecord> = {};
  for (const [d, v] of Object.entries(saved)) if (d >= firstDay && d <= day) kept[d] = v;
  const week = Object.values(kept).reduce<DistanceRecord | null>((a, b) => (!a || b.km > a.km ? b : a), null);
  return { today: kept[day] ?? null, week, kept };
}
