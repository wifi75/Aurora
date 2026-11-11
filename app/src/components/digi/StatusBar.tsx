import { useState } from 'react';
import { useT } from '../../i18n';

interface Props {
  transmitting: boolean;
  /** «Flashing TX»: l'indicatore TX lampeggia durante la trasmissione. */
  flashTx?: boolean;
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

/** La barra di stato: RX/TX, Sq, AFC, Lock, Snap, frequenza audio, dati del modo e modo. Data e ora tolte (scelta del proprietario 2026-11-11:
 * l'ora c'e' gia' nella barra in alto di Aurora). */
export function DigiStatusBar(p: Props) {
  const t = useT();
  const [draft, setDraft] = useState<string | null>(null);
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
      <button type="button" className={`digi-st digi-st--${p.transmitting ? 'tx' : 'rx'}${p.transmitting && p.flashTx ? ' digi-st--flash' : ''}`} title={t('digi.st.txrx')} onClick={p.onToggleTx}>
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
        <span className="digi-st-unit">Hz</span>
      </label>
      {p.modeData !== '' ? (
        <button type="button" className="digi-st" title={t('digi.st.modeData')} onClick={p.onModeSettings}>
          {p.modeData}
        </button>
      ) : null}
      <span className="digi-st digi-st--info">{p.modeName}</span>
    </div>
  );
}
