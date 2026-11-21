import { useEffect, useState } from 'react';
import { putWefax, type WefaxSettings, type WefaxSnapshot } from '../../api/client';
import { useT } from '../../i18n';

/**
 * I due interruttori della testata, come in SeaTTY: AFC (verde se acceso: la ricezione segue il segnale se la radio e' un po' fuori) e
 * SQL con la soglia (l'immagine parte quando c'e' il segnale e si chiude quando sparisce).
 */
export function AfcSql({ snap, onChanged }: { snap: WefaxSnapshot; onChanged: () => void }) {
  const t = useT();
  const s = snap.settings;
  const save = (patch: Partial<WefaxSettings>) => {
    putWefax({ ...s, ...patch }).then(onChanged, onChanged);
  };
  const [draft, setDraft] = useState(String(Math.round(s.squelch_level)));
  useEffect(() => setDraft(String(Math.round(s.squelch_level))), [s.squelch_level]);
  const commit = () => {
    const v = Math.round(Number(draft));
    if (!Number.isFinite(v)) {
      setDraft(String(Math.round(s.squelch_level)));
      return;
    }
    const clamped = Math.min(100, Math.max(0, v));
    setDraft(String(clamped));
    if (clamped !== s.squelch_level) save({ squelch_level: clamped });
  };
  const signal = Math.round(snap.rx.signal ?? 0);
  const afc = Math.round(snap.rx.afc_hz ?? 0);
  return (
    <div className="wefax-afcsql" role="group" aria-label={`${t('wefax.afc.label')} / ${t('wefax.sql.label')}`}>
      <button type="button" className={`wefax-tog${s.afc ? ' on' : ''}`} aria-pressed={s.afc} title={t('wefax.afc.tip')} onClick={() => save({ afc: !s.afc })}>
        {t('wefax.afc.label')}
        {s.afc && afc !== 0 ? <small>{`${afc > 0 ? '+' : ''}${afc} Hz`}</small> : null}
      </button>
      <input
        className="wefax-sql-level"
        type="number"
        inputMode="numeric"
        min={0}
        max={100}
        value={draft}
        aria-label={t('wefax.sql.level')}
        title={t('wefax.sql.levelTip')}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === 'Enter' && commit()}
      />
      <button
        type="button"
        className={`wefax-tog${s.squelch ? ' on' : ''}${s.squelch && signal >= s.squelch_level ? ' open' : ''}`}
        aria-pressed={s.squelch}
        title={`${t('wefax.sql.tip')} (${t('wefax.sql.signal')}: ${signal})`}
        onClick={() => save({ squelch: !s.squelch })}
      >
        {t('wefax.sql.label')}
      </button>
    </div>
  );
}
