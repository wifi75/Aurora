import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getCat, getEngineSettings, getHealth } from '../api/client';
import { useT } from '../i18n';
import { periodSecs } from '../lib/period';
import { useStation } from '../state/station';

/** Dove siamo nel periodo (secondi trascorsi e durata), agganciati all'orologio come i periodi del motore. */
function usePeriodClock(mode: string | undefined): { elapsed: number; total: number } {
  const total = periodSecs(mode ?? 'FT8');
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  return { elapsed: (now / 1000) % total, total };
}

/**
 * La barra in fondo alla finestra: a sinistra le decodifiche
 * dell'ultimo periodo, l'avanzamento del periodo con i secondi, il profilo della radio e il watchdog; a destra nome, versione e autore
 * dal core (mai scritti a mano qui).
 */
export function Footer() {
  const t = useT();
  const { data, isError } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, staleTime: 30_000 });
  const engine = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false, staleTime: 30_000 });
  const { elapsed, total } = usePeriodClock(status?.mode);

  // le decodifiche dell'ultimo periodo: quelle con la stessa ora della piu' recente
  const periodDecodes = decodes.length > 0 ? decodes.filter((d) => d.time_ms === decodes[0]!.time_ms).length : 0;
  const profile = cat.data?.active_profile ?? null;
  const watchdog = Number(engine.data?.values.TxWatchdogTimer ?? 0);
  const shown = Math.min(Math.ceil(total), Math.floor(elapsed) + 1);

  return (
    <footer className="app-footer app-bar">
      {status ? (
        <div className="app-bar-left">
          <span className="bar-item">
            {t('bar.decodes')}: <b>{periodDecodes}</b>
          </span>
          <span className="bar-progress" role="progressbar" aria-label={t('bar.period')} aria-valuemin={0} aria-valuemax={total} aria-valuenow={Math.floor(elapsed)}>
            <i style={{ width: `${Math.min(100, (elapsed / total) * 100)}%` }} />
          </span>
          <span className="bar-item bar-count">{`${shown}/${Math.ceil(total)}`}</span>
          {profile ? (
            <span className="bar-item">
              {t('bar.profile')}: <b>{profile}</b>
            </span>
          ) : null}
          {watchdog > 0 ? (
            <span className="bar-item" title={t('bar.watchdogTip')}>
              {t('bar.watchdog')}: <b>{watchdog}m</b>
            </span>
          ) : null}
        </div>
      ) : (
        <div className="app-bar-left" />
      )}
      <small>
        {data
          ? t('footer.credit', { name: data.name, version: data.version })
          : isError
            ? t('footer.coreDown')
            : t('footer.connecting')}
      </small>
    </footer>
  );
}
