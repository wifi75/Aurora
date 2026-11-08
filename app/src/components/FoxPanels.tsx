import { useEffect, useState } from 'react';
import { ApiError, postFox, postFoxClublog } from '../api/client';
import type { FoxSnapshot } from '../api/schemas';
import { useT } from '../i18n';
import { foxContinents, foxLogAdif, foxLogCabrillo, foxRates, saveText } from '../lib/foxLog';
import { replyErrorMessage } from '../lib/replyError';
import { toast } from '../lib/toast';

/** Il vecchio posto del seme (solo nel browser): si passa una volta al motore e si cancella. */
const OTP_KEY = 'aurora-fox-otp-v1';
const SEED_RE = /^[A-Z2-7]{16}$/;

/** Il seme rimasto nel browser da una versione di prima, se c'e'; lo legge una volta sola e lo toglie. */
function takeOldOtp(): { seed: string; interval: number } | null {
  try {
    const raw = window.localStorage.getItem(OTP_KEY);
    if (raw === null) return null;
    window.localStorage.removeItem(OTP_KEY);
    const v = JSON.parse(raw) as { seed?: unknown; interval?: unknown } | null;
    const seed = typeof v?.seed === 'string' ? v.seed : '';
    return SEED_RE.test(seed) ? { seed, interval: typeof v?.interval === 'number' ? v.interval : 1 } : null;
  } catch {
    return null;
  }
}

/**
 * I numeri della sessione in tesserine piccole (etichetta minuscola sopra, numero sotto) nella riga alta della testata:
 * QSO a log, QSO in corso, ritmo sugli ultimi 10 con le barre dell'ultima ora, ultima ora, chiamanti, coda, periodi, continenti.
 */
export function FoxStats({ fox }: { fox: FoxSnapshot }) {
  const t = useT();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(id);
  }, []);
  const { rate, lastHour, buckets } = foxRates(fox.log, now);
  const peak = Math.max(1, ...buckets);
  const continents = foxContinents(fox.log);
  return (
    <div className="foxc-stats">
      <div className="foxc-stat foxc-stat--ok" title={t('tip.fox.logged')}>
        <small>{t('fox.logged')}</small>
        <b className="data">{fox.logged}</b>
      </div>
      <div className="foxc-stat foxc-stat--accent" title={t('tip.fox.inProgress')}>
        <small>{t('fox.inProgress')}</small>
        <b className="data">{fox.in_progress.length}</b>
      </div>
      <div className="foxc-stat foxc-stat--rate" title={t('tip.fox.rate')}>
        <small>{t('fox.stat.rate')}</small>
        <b className="data">{rate}/h</b>
        <div className="foxc-spark" aria-hidden="true">
          {buckets.map((b, i) => (
            <i key={i} style={{ height: `${Math.max(4, (b / peak) * 100)}%`, opacity: b ? 0.85 : 0.25 }} />
          ))}
        </div>
      </div>
      <div className="foxc-stat" title={t('tip.fox.rate')}>
        <small>{t('fox.stat.hour')}</small>
        <b className="data">{lastHour}</b>
      </div>
      <div className="foxc-stat" title={t('tip.fox.callers')}>
        <small>{t('fox.callers')}</small>
        <b className="data">{fox.callers.length}</b>
      </div>
      <div className="foxc-stat foxc-stat--warn" title={t('tip.fox.queue')}>
        <small>{t('fox.queue')}</small>
        <b className="data">
          {fox.queue.length} / {fox.params.queue_max}
        </b>
      </div>
      <div className="foxc-stat" title={t('tip.fox.periods')}>
        <small>{t('fox.periods')}</small>
        <b className="data">{fox.periods}</b>
      </div>
      {continents.length > 0 ? (
        <div className="foxc-stat foxc-stat--cont" title={t('tip.fox.continents')}>
          <small>{t('fox.continents')}</small>
          <span className="foxc-conts data">
            {continents.map(([c, n]) => (
              <span key={c}>
                {c} <b>{n}</b>
              </span>
            ))}
          </span>
        </div>
      ) : null}
    </div>
  );
}

/** La verifica OTP: seme e intervallo salvati nel motore (`fox-otp.json` nella cartella dati); il seme non torna alla pagina, che sa solo se c'e'. */
export function FoxOtp({ fox }: { fox: FoxSnapshot }) {
  const t = useT();
  const [seed, setSeed] = useState('');
  const [every, setEvery] = useState(fox.otp_interval || 1);
  // l'intervallo segue quello del motore finche' non lo si cambia qui
  useEffect(() => setEvery(fox.otp_interval || 1), [fox.otp_interval]);
  // un seme lasciato nel browser da una versione di prima passa al motore una volta, poi si dimentica
  const [old] = useState(takeOldOtp);
  useEffect(() => {
    if (old) void postFox({ action: 'otp', seed: old.seed, interval: old.interval });
  }, [old]);
  const clean = seed.trim().toUpperCase();
  const bad = clean !== '' && !SEED_RE.test(clean);
  const send = (body: Parameters<typeof postFox>[0]) =>
    void postFox(body)
      .then(() => setSeed(''))
      .catch((error: unknown) => toast(replyErrorMessage(error, t), 'error'));
  // casella vuota con un seme gia' salvato: cambia solo l'intervallo
  const apply = () => {
    if (!bad) send(clean === '' && fox.otp ? { action: 'otp', keep_seed: true, interval: every } : { action: 'otp', seed: clean || null, interval: every });
  };
  return (
    <div className="foxc-otp" title={t('tip.fox.otp')}>
      <span className={`foxc-pill foxc-pill--${fox.otp ? 'ready' : 'off'}`}>{fox.otp ? t('fox.otpOn') : t('fox.otpOff')}</span>
      {fox.otp && fox.otp_code ? <span className="data muted">{fox.otp_code}</span> : null}
      <label className="foxc-label">
        {t('fox.otpSeed')}
        <input
          className="foxc-input data"
          type="password"
          autoComplete="off"
          spellCheck={false}
          maxLength={16}
          size={16}
          value={seed}
          placeholder={fox.otp ? t('fox.otpSaved') : ''}
          aria-invalid={bad}
          onChange={(e) => setSeed(e.target.value)}
        />
      </label>
      <label className="foxc-label">
        {t('fox.otpEvery')}
        <input className="foxc-input data foxc-input--num" type="number" min={1} max={100} value={every} onChange={(e) => setEvery(Math.min(100, Math.max(1, Number(e.target.value) || 1)))} />
      </label>
      <button type="button" className="foxc-btn foxc-btn--small" disabled={bad} onClick={apply}>
        {t('fox.otpSave')}
      </button>
      {fox.otp ? (
        <button type="button" className="foxc-btn foxc-btn--small" title={t('tip.fox.otpRemove')} onClick={() => send({ action: 'otp', seed: null, interval: every })}>
          {t('fox.otpRemove')}
        </button>
      ) : null}
      {bad ? <span className="foxc-warn">{t('fox.otpBad')}</span> : null}
    </div>
  );
}

/** Il pacchetto del periodo (i messaggi in onda, con la riga di verifica di UltraFOX) e i parametri fissi. */
export function FoxPacket({ fox }: { fox: FoxSnapshot }) {
  const t = useT();
  const p = fox.params;
  return (
    <>
      {fox.tx.length > 0 ? (
        <div className="foxc-packet">
          <span className="muted">{t('fox.packet')}</span>
          <br />
          <span className="data">{fox.tx.join(' · ')}</span>
        </div>
      ) : null}
      <p className="foxc-params muted" title={t('tip.fox.params')}>
        {t('fox.params', { s: p.strikes, w: p.wait, c: p.cq_every, q: p.queue_max })}
      </p>
    </>
  );
}

/** Il Fox log della sessione, il piu' recente in alto, con il salvataggio in ADIF e Cabrillo. */
export function FoxLogCard({ fox, myCall, myGrid }: { fox: FoxSnapshot; myCall: string; myGrid: string }) {
  const t = useT();
  const stamp = new Date().toISOString().slice(0, 10).replaceAll('-', '');
  const rows = [...fox.log].reverse();
  const [sending, setSending] = useState(false);
  const toClublog = () => {
    setSending(true);
    postFoxClublog(fox.log)
      .then((n) => toast(t('fox.clublogQueued', { n })))
      .catch((error: unknown) => toast(error instanceof ApiError && error.code === 'clublog_off' ? t('fox.clublogOff') : replyErrorMessage(error, t), 'error'))
      .finally(() => setSending(false));
  };
  return (
    <section className="foxc-card foxc-tone--mint">
      <header className="foxc-card-head">
        <h2 title={t('tip.fox.log')}>{t('fox.log')}</h2>
        <span className="foxc-count data">{fox.log.length}</span>
        <div className="foxc-tools">
          <button type="button" className="foxc-btn foxc-btn--small" disabled={fox.log.length === 0} title={t('tip.fox.adif')} onClick={() => saveText(`fox_${myCall.replaceAll('/', '-')}_${stamp}.adi`, foxLogAdif(fox.log, myCall, myGrid))}>
            ADIF
          </button>
          <button type="button" className="foxc-btn foxc-btn--small" disabled={fox.log.length === 0} title={t('tip.fox.cabrillo')} onClick={() => saveText(`fox_${myCall.replaceAll('/', '-')}_${stamp}.log`, foxLogCabrillo(fox.log, myCall))}>
            Cabrillo
          </button>
          <button type="button" className="foxc-btn foxc-btn--small" disabled={fox.log.length === 0 || sending} title={t('tip.fox.clublog')} onClick={toClublog}>
            Club Log
          </button>
        </div>
      </header>
      <div className="foxc-scroll foxc-scroll--log">
        {rows.length === 0 ? (
          <p className="foxc-empty">{t('fox.empty')}</p>
        ) : (
          <table className="foxc-table">
            <thead>
              <tr>
                <th>UTC</th>
                <th>{t('fox.col.call')}</th>
                <th>{t('fox.col.band')}</th>
                <th className="num">{t('fox.sent')}</th>
                <th className="num">{t('fox.rcvd')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((q) => (
                <tr key={`${q.call}-${q.band}-${q.time_ms}`}>
                  <td className="data">{new Date(q.time_ms).toISOString().slice(11, 16)}</td>
                  <td className="data foxc-call">{q.call}</td>
                  <td>{q.band}</td>
                  <td className="num data">{q.sent}</td>
                  <td className="num data">{q.rcvd}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
