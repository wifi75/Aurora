import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getEngineSettings, postSpotDxsummit } from '../../api/client';
import { useT } from '../../i18n';
import { toast } from '../../lib/toast';

interface Props {
  /** Il nominativo gia' nel QSO in corso (si puo' cambiare). */
  call: string;
  /** Frequenza della radio e del segnale ricevuto (Hz audio): la frequenza dello spot e' la somma. */
  dialHz: number;
  audioHz: number;
  /** Il modo (PSK31, RTTY...) e' l'inizio del testo dello spot. */
  modeName: string;
  onClose(): void;
}

/** La finestra «Invia spot»: nominativo, la propria stazione, frequenza in kHz e testo; parte verso DXSummit. */
export function SpotDialog({ call, dialHz, audioHz, modeName, onClose }: Props) {
  const t = useT();
  const engine = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false, staleTime: 30_000 });
  const mine = engine.data?.values?.MyCall ?? '';
  const [dx, setDx] = useState(call);
  const [khz, setKhz] = useState(dialHz > 0 ? ((dialHz + audioHz) / 1000).toFixed(1) : '');
  const [info, setInfo] = useState(modeName);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');

  const send = () => {
    const f = Number(khz.replace(',', '.'));
    if (dx.trim() === '' || !Number.isFinite(f) || f <= 0) {
      setProblem(t('digi.spot.bad'));
      return;
    }
    setBusy(true);
    setProblem('');
    postSpotDxsummit({ dx_call: dx.trim(), freq_khz: f, info: info.trim() })
      .then((res) => {
        toast(t('ctl.spotOk', { call: res.dx_call, khz: (res.frequency_hz / 1000).toFixed(1) }), 'ok');
        onClose();
      })
      .catch((e: unknown) => {
        setBusy(false);
        setProblem(e instanceof Error && e.message ? e.message : t('digi.spot.fail'));
      });
  };

  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.spot.title')}>
      <div className="digi-dialog digi-dialog--small">
        <header>
          <strong>{t('digi.spot.title')}</strong>
        </header>
        <label>
          <span>{t('digi.spot.call')}</span>
          <input value={dx} maxLength={20} autoFocus onChange={(e) => setDx(e.target.value.toUpperCase())} />
        </label>
        <label>
          <span>{t('digi.spot.de')}</span>
          <input value={mine} readOnly />
        </label>
        <label>
          <span>{t('digi.spot.freq')}</span>
          <input value={khz} inputMode="decimal" onChange={(e) => setKhz(e.target.value)} />
        </label>
        <label>
          <span>{t('digi.spot.info')}</span>
          <input value={info} maxLength={120} onChange={(e) => setInfo(e.target.value)} />
        </label>
        {problem ? (
          <p className="notice sstv-bad" role="alert">
            {problem}
          </p>
        ) : null}
        <footer>
          <button type="button" className="glass sstv-btn sstv-btn--primary" disabled={busy} onClick={send}>
            {t('digi.spot.ok')}
          </button>
          <button type="button" className="glass sstv-btn" onClick={onClose}>
            {t('digi.spot.cancel')}
          </button>
        </footer>
      </div>
    </div>
  );
}
