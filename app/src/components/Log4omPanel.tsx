import { useQuery, useQueryClient } from '@tanstack/react-query';
import { SetupCta } from './SetupCta';
import { useEffect, useRef, useState } from 'react';
import { getLog4om } from '../api/client';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';

/** Quanto resta acceso il lampeggio del QSO appena messo a log (tre lampeggi lenti da 1,4 s). */
const FLASH_MS = 4300;

/** Gli ultimi QSO del log di Log4OM2 (sola lettura): quanti, si sceglie in Impostazioni → Generali. */
export function Log4omPanel() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 10_000, retry: false });
  const client = useQueryClient();
  // appena un QSO viene messo a log il pannello si rilegge subito, senza aspettare il giro dei 10 secondi
  const logged = useStation((st) => st.qsosLogged);
  useEffect(() => {
    if (logged > 0) {
      void client.invalidateQueries({ queryKey: ['log4om'] });
    }
  }, [logged, client]);
  // un QSO nuovo in cima: tutta la riga lampeggia tre volte, piano
  const topKey = log.data?.qsos[0] ? `${log.data.qsos[0].date}-${log.data.qsos[0].call}` : null;
  const previous = useRef<string | null>(null);
  const [flash, setFlash] = useState<string | null>(null);
  useEffect(() => {
    if (topKey === null) return;
    if (previous.current !== null && previous.current !== topKey) {
      setFlash(topKey);
      const timer = window.setTimeout(() => setFlash(null), FLASH_MS);
      previous.current = topKey;
      return () => window.clearTimeout(timer);
    }
    previous.current = topKey;
  }, [topKey]);

  if (log.isError) return <p className="muted log4om-note">{t('log.unavailable')}</p>;
  if (!log.data) return <p className="muted log4om-note">…</p>;
  if (log.data.qsos.length === 0) return <SetupCta className="setup-cta--log" href="#/statistiche" title="cta.log.none" action="cta.log.go" />;
  return (
    <div className="log4om">
      <table className="log4om-table">
        <thead>
          <tr>
            <th>{t('log.col.date')}</th>
            <th>{t('log.col.call')}</th>
            <th>{t('log.col.band')}</th>
            <th>{t('log.col.mode')}</th>
            <th title={t('tip.ui.colRst')}>{t('log.col.rst')}</th>
            <th>{t('log.col.country')}</th>
          </tr>
        </thead>
        <tbody>
          {log.data.qsos.map((q, i) => (
            <tr key={`${q.date}-${q.call}-${i}`} className={flash !== null && flash === `${q.date}-${q.call}` ? 'log4om-row--flash' : undefined}>
              {/* giorno/mese e ora, senza l'anno: la data intera nel suggerimento */}
              <td className="data log4om-date" title={q.date.replace('Z', '').slice(0, 16)}>
                {`${q.date.slice(8, 10)}/${q.date.slice(5, 7)} ${q.date.slice(11, 16)}`}
              </td>
              <td className="data log4om-call">{q.call}</td>
              <td className="data">{q.band}</td>
              <td className="data">{q.mode}</td>
              <td className="data log4om-rst">{[q.rst_sent, q.rst_rcvd].filter(Boolean).join('/')}</td>
              <td>{q.country}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Nella barra del pannello Log: quanti QSO (l'esportazione ADIF sta nella pagina Log del menu in alto). */
export function LogHeader() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 10_000, retry: false });
  return (
    <div className="dock-grip-extra dock-grip-extra--right" onPointerDown={(e) => e.stopPropagation()}>
      {log.data ? <span className="muted log4om-total">{t('log.total', { n: log.data.total })}</span> : null}
    </div>
  );
}
