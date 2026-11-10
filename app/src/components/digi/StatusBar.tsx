import { useEffect, useState } from 'react';
import { useT } from '../../i18n';

interface Props {
  transmitting: boolean;
  squelchOn: boolean;
  /** Il segnale e' sopra la soglia: nella barra si legge «Sq *». */
  squelchOpen: boolean;
  afc: boolean;
  /** Lock: la frequenza di trasmissione e' fissa e diversa da quella di ricezione. */
  lock: boolean;
  snap: boolean;
  /** La frequenza audio su cui si ascolta (Hz). */
  freqHz: number;
  /** Velocita' e spostamento del modo, da mostrare («45 bd, 170 Hz»); vuoto per i modi che non ne hanno. */
  modeData: string;
  modeName: string;
  onToggleTx(): void;
  onToggleSquelch(): void;
  onToggleAfc(): void;
  onToggleLock(): void;
  onToggleSnap(): void;
  onFreq(hz: number): void;
  /** Clic sui dati del modo: la finestra delle impostazioni del modo. */
  onModeSettings(): void;
  /** Clic destro sulla barra: soglia dello squelch. */
  onThreshold(): void;
}

const pad = (n: number) => String(n).padStart(2, '0');

/** La barra di stato: RX/TX, Sq, AFC, Lock, Snap, frequenza audio, dati del modo, modo, data e ora. */
export function DigiStatusBar(p: Props) {
  const t = useT();
  const [now, setNow] = useState(() => new Date());
  const [utc, setUtc] = useState(true);
  const [draft, setDraft] = useState<string | null>(null);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const d = utc ? { day: now.getUTCDate(), month: now.getUTCMonth() + 1, year: now.getUTCFullYear(), h: now.getUTCHours(), m: now.getUTCMinutes(), s: now.getUTCSeconds() } : { day: now.getDate(), month: now.getMonth() + 1, year: now.getFullYear(), h: now.getHours(), m: now.getMinutes(), s: now.getSeconds() };
  const freqText = draft ?? p.freqHz.toFixed(1).replace('.', ',');
  const commit = () => {
    if (draft === null) return;
    const hz = Number(draft.replace(',', '.'));
    setDraft(null);
    if (Number.isFinite(hz) && hz >= 200 && hz <= 3500) p.onFreq(hz);
  };

  return (
    <div
      className="digi-status"
      role="status"
      onContextMenu={(e) => {
        e.preventDefault();
        p.onThreshold();
      }}
    >
      <button type="button" className={`digi-st digi-st--${p.transmitting ? 'tx' : 'rx'}`} title={t('digi.st.txrx')} onClick={p.onToggleTx}>
        {p.transmitting ? 'TX' : 'RX'}
      </button>
      <button type="button" className={`digi-st${p.squelchOn ? ' on' : ''}`} aria-pressed={p.squelchOn} title={t('digi.st.squelch')} onClick={p.onToggleSquelch}>
        Sq{p.squelchOn && p.squelchOpen ? ' *' : ''}
      </button>
      <button type="button" className={`digi-st${p.afc ? ' on' : ''}`} aria-pressed={p.afc} title={t('digi.st.afc')} onClick={p.onToggleAfc}>
        AFC
      </button>
      <button type="button" className={`digi-st${p.lock ? ' on' : ''}`} aria-pressed={p.lock} title={t('digi.st.lock')} onClick={p.onToggleLock}>
        Lock
      </button>
      <button type="button" className={`digi-st${p.snap ? ' on' : ''}`} aria-pressed={p.snap} title={t('digi.st.snap')} onClick={p.onToggleSnap}>
        Snap
      </button>
      <label className="digi-st digi-st--freq" title={t('digi.st.freq')}>
        <input
          value={freqText}
          inputMode="decimal"
          aria-label={t('digi.st.freq')}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur();
            if (e.key === 'Escape') setDraft(null);
          }}
        />
        <span> Hz</span>
      </label>
      {p.modeData !== '' ? (
        <button type="button" className="digi-st" title={t('digi.st.modeData')} onClick={p.onModeSettings}>
          {p.modeData}
        </button>
      ) : null}
      <span className="digi-st digi-st--info">{p.modeName}</span>
      <span className="digi-st digi-st--info">{`${pad(d.day)}/${pad(d.month)}/${d.year}`}</span>
      <button type="button" className="digi-st digi-st--info" title={t('digi.st.clock')} onClick={() => setUtc((v) => !v)}>
        {`${pad(d.h)}:${pad(d.m)}:${pad(d.s)}${utc ? ' z' : ''}`}
      </button>
    </div>
  );
}
