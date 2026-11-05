import '../styles/first-run.css';
import '../styles/log-prompt.css';
import '../styles/check-all.css';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getCat, getClock, getNetworkCheck } from '../api/client';
import { useT, type Key } from '../i18n';
import { LEVEL_STALE_MS } from '../lib/audioLevel';
import { periodSecs } from '../lib/period';
import { useStation } from '../state/station';

type Verdict = 'ok' | 'warn' | 'bad' | 'wait';
interface Row {
  id: 'station' | 'radio' | 'audio' | 'clock' | 'phone';
  verdict: Verdict;
  href: string;
}

const ICON: Record<Verdict, string> = { ok: '✓', warn: '!', bad: '×', wait: '…' };

/** Riquadro «Verifica tutto» di Impostazioni › Assistenza: apre una scheda con stazione, radio, audio, orologio e telefono, ognuno con cosa non va e dove sistemarlo. */
export function CheckAllBox() {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <section className="about-fact about-support">
      <h3>{t('ca.title')}</h3>
      <p>{t('ca.intro')}</p>
      <div className="about-support-actions">
        <button type="button" className="glass about-support-main" onClick={() => setOpen(true)}>
          {t('ca.run')}
        </button>
      </div>
      {open ? <CheckAll onClose={() => setOpen(false)} /> : null}
    </section>
  );
}

function CheckAll({ onClose }: { onClose: () => void }) {
  const t = useT();
  const status = useStation((s) => s.status);
  const audio = useStation((s) => s.audioLevel);
  const audioAt = useStation((s) => s.audioLevelAt);
  const notices = useStation((s) => s.notices);
  const cat = useQuery({ queryKey: ['check-cat'], queryFn: getCat, retry: false });
  const clock = useQuery({ queryKey: ['check-clock'], queryFn: getClock, retry: false });
  const net = useQuery({ queryKey: ['check-net'], queryFn: getNetworkCheck, retry: false });
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const now = Date.now();
  const station: Verdict = !status ? 'wait' : status.de_call.trim() === '' || status.de_grid.trim() === '' ? 'bad' : 'ok';
  const catState = cat.data?.status.state;
  const radio: Verdict = cat.isPending ? 'wait' : cat.isError ? 'bad' : catState === 'connected' ? (notices.omnirig && now - notices.omnirig.at < 600_000 ? 'warn' : 'ok') : catState === 'error' ? 'bad' : 'warn';
  const fresh = audio && now - audioAt <= LEVEL_STALE_MS ? audio : null;
  const lossy = notices.audio_loss && now - notices.audio_loss.at < 600_000;
  const audioV: Verdict = !fresh ? 'bad' : fresh.state === 'overdrive' || fresh.state === 'silent' ? 'bad' : fresh.state === 'low' || lossy ? 'warn' : 'ok';
  const offset = clock.data?.last?.offset_ms;
  const frac = offset == null ? null : Math.abs(offset) / (periodSecs(status?.mode ?? 'FT8') * 1000);
  const clockV: Verdict = clock.isPending ? 'wait' : frac === null ? 'warn' : frac >= 0.08 ? 'bad' : frac >= 0.04 ? 'warn' : 'ok';
  const n = net.data;
  const phone: Verdict = net.isPending ? 'wait' : !n ? 'warn' : n.firewall_allowed === false ? 'bad' : n.category?.toLowerCase() === 'public' || n.addresses.length === 0 ? 'warn' : 'ok';
  const rows: Row[] = [
    { id: 'station', verdict: station, href: '#/impostazioni/general' },
    { id: 'radio', verdict: radio, href: '#/impostazioni/radio' },
    { id: 'audio', verdict: audioV, href: '#/impostazioni/audio' },
    { id: 'clock', verdict: clockV, href: '#/impostazioni/clock' },
    { id: 'phone', verdict: phone, href: '#/impostazioni/access' },
  ];
  const allOk = rows.every((r) => r.verdict === 'ok');
  return (
    <div className="first-run log-prompt" role="dialog" aria-modal="true" aria-labelledby="check-all-title">
      <div className="first-run__card lp-card">
        <div className="first-run__glow" aria-hidden="true" />
        <header className="lp-head">
          <span className="first-run__brand">Aurora</span>
        </header>
        <div className="lp-body">
          <h2 id="check-all-title" className="lp-title">
            {t('ca.title')}
          </h2>
          <p className="ca-summary">{t(allOk ? 'ca.allOk' : 'ca.someBad')}</p>
          <ul className="ca-list">
            {rows.map((r) => (
              <li key={r.id} className={`ca-row ca-row--${r.verdict}`}>
                <span className="ca-icon" aria-hidden="true">
                  {ICON[r.verdict]}
                </span>
                <span className="ca-text">
                  <strong>{t(`ca.${r.id}` as Key)}</strong>
                  <span>{t((r.id === 'clock' && r.verdict === 'warn' && offset == null ? 'ca.clock.none' : `ca.${r.id}.${r.verdict === 'wait' ? 'ok' : r.verdict}`) as Key, { detail: r.id === 'clock' ? Math.round(Math.abs(offset ?? 0)) : '' })}</span>
                </span>
                {r.verdict === 'warn' || r.verdict === 'bad' ? (
                  <a className="ca-fix" href={r.href} onClick={onClose}>
                    {t('diag.fix')}
                  </a>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
        <footer className="lp-foot">
          <button type="button" className="first-run__next lp-log" autoFocus onClick={onClose}>
            {t('ca.close')}
          </button>
        </footer>
      </div>
    </div>
  );
}
