import { HeaderAlertSwitch } from './AlertSwitch';
import { ThemeToggle } from './ThemeToggle';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getCat, getEngineSettings, getHealth } from '../api/client';
import { useT } from '../i18n';
import { periodSecs } from '../lib/period';
import { useStation, useStatusPick } from '../state/station';

/** Dove siamo nel periodo (secondi trascorsi e durata), agganciati all'orologio come i periodi del motore. */
function usePeriodClock(mode: string | undefined): { elapsed: number; total: number; now: number } {
  const total = periodSecs(mode ?? 'FT8');
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    // due volte al secondo bastano per l'orologio e la barra del periodo; a finestra nascosta non si aggiorna
    const timer = window.setInterval(() => {
      if (!document.hidden) setNow(Date.now());
    }, 500);
    return () => window.clearInterval(timer);
  }, []);
  return { elapsed: (now / 1000) % total, total, now };
}

/**
 * La barra in fondo alla finestra: a sinistra le decodifiche
 * dell'ultimo periodo, l'avanzamento del periodo con i secondi, il profilo della radio e il watchdog; a destra nome, versione e autore
 * dal core (mai scritti a mano qui).
 */
export function Footer({ page }: { page?: string } = {}) {
  // decodifiche, avanzamento del periodo e watchdog sono cose dell'FT8: nelle pagine degli altri modi non servono
  const otherMode = page === 'sstv' || page === 'digi' || page === 'sat';
  const t = useT();
  const { data, isError } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const status = useStatusPick(['mode', 'transmitting']);
  const decodes = useStation((s) => s.decodes);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, staleTime: 30_000 });
  const engine = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false, staleTime: 30_000 });
  const { elapsed, total, now } = usePeriodClock(status?.mode);

  // le decodifiche dell'ultimo periodo: quelle con la stessa ora della piu' recente
  const periodDecodes = decodes.length > 0 ? decodes.filter((d) => d.time_ms === decodes[0]!.time_ms).length : 0;
  const profile = cat.data?.active_profile ?? null;
  const watchdog = Number(engine.data?.values.TxWatchdogTimer ?? 0);
  const shown = Math.min(Math.ceil(total), Math.floor(elapsed) + 1);

  return (
    <footer className="app-footer app-bar">
      {status && !otherMode ? (
        <div className="app-bar-left">
          <span className="bar-item">
            {t('bar.decodes')}: <b>{periodDecodes}</b>
          </span>
          <span className={`bar-progress${status.transmitting ? ' is-tx' : ''}`} role="progressbar" aria-label={t('bar.period')} aria-valuemin={0} aria-valuemax={total} aria-valuenow={Math.floor(elapsed)}>
            <i className={elapsed < 0.6 ? 'is-reset' : undefined} style={{ width: `${Math.min(100, (elapsed / total) * 100)}%` }} />
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
      {/* avvisi e tema: nel menu nuovo stanno qui, a destra, vicino all'ora */}
      <span className="bar-tools">
        <HeaderAlertSwitch />
        <ThemeToggle />
      </span>
      <span className="bar-utc num" title="UTC">
        {new Date(now).toISOString().slice(11, 19)} <i>UTC</i>
      </span>
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
