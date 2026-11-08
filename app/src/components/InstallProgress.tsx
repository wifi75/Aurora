import { useEffect, useRef, useState } from 'react';
import { getUpdate } from '../api/client';
import { useT } from '../i18n';

const mb = (bytes: number) => (bytes / (1024 * 1024)).toFixed(1).replace('.', ',');

interface Sample {
  done: number;
  total: number;
  /** Velocita' media degli ultimi secondi (byte al secondo). */
  speed: number;
}

/**
 * L'avanzamento dello scaricamento durante «Installa e riavvia», come una barra di download vera: percentuale, MB arrivati su MB totali,
 * velocita' e tempo che manca. Aurora scarica il pacchetto (qualche decina di MB) prima di avviare l'installatore: si legge lo stato degli
 * aggiornamenti ogni mezzo secondo. Un Aurora vecchio che non dice l'avanzamento mostra una barra che scorre con «Scaricamento in corso».
 */
export function InstallProgress({ active }: { active: boolean }) {
  const t = useT();
  const [sample, setSample] = useState<Sample | null>(null);
  const history = useRef<Array<{ at: number; done: number }>>([]);

  useEffect(() => {
    if (!active) {
      setSample(null);
      history.current = [];
      return;
    }
    let alive = true;
    const tick = async () => {
      try {
        const status = await getUpdate();
        const d = status.download;
        if (!alive || !d) return;
        const now = performance.now();
        const h = history.current;
        h.push({ at: now, done: d.done });
        while (h.length > 2 && now - h[0]!.at > 4000) h.shift();
        const first = h[0]!;
        const speed = now > first.at ? ((d.done - first.done) * 1000) / (now - first.at) : 0;
        setSample({ done: d.done, total: d.total, speed });
      } catch {
        // Aurora si sta chiudendo per installare: resta l'ultimo valore
      }
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 500);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [active]);

  if (!active) return null;
  const known = sample !== null && sample.total > 0;
  const pct = known ? Math.min(100, Math.floor((sample.done / sample.total) * 100)) : null;
  const done = pct !== null && pct >= 100;
  const left = known && sample.speed > 0 ? Math.max(0, Math.round((sample.total - sample.done) / sample.speed)) : null;
  return (
    <div className="install-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined} aria-label={t('upd.installing')}>
      <div className="install-progress-head">
        <span className="install-progress-title">{done ? t('upd.progress.start') : t('upd.progress.title')}</span>
        <span className="install-progress-pct">{pct === null ? '' : `${pct}%`}</span>
      </div>
      <div className="install-progress-track">
        <div className={`install-progress-fill${pct === null ? ' install-progress-fill--wait' : ''}${done ? ' install-progress-fill--done' : ''}`} style={pct === null ? undefined : { width: `${pct}%` }} />
      </div>
      <div className="install-progress-meta">
        {known ? (
          <>
            <span>
              {mb(sample.done)} / {mb(sample.total)} MB
            </span>
            {!done && sample.speed > 0 ? <span>{t('upd.progress.speed', { speed: mb(sample.speed) })}</span> : null}
            {!done && left !== null ? <span>{t('upd.progress.left', { s: left })}</span> : null}
          </>
        ) : (
          <span>{t('upd.progress.connect')}</span>
        )}
      </div>
    </div>
  );
}
