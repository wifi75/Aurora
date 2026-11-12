import { useCallback, useEffect, useState } from 'react';
import { useT } from '../../i18n';
import { PopupMenu } from '../PopupMenu';
import { CW_BANDWIDTHS, CW_WIDE_HZ, RTTY_BAUDS, RTTY_SHIFTS, roundBaud, withCurrent } from './ModeSettings';
import '../../styles/call-menu.css';

/** In RTTY la barra ha le tendine di spostamento e velocita' e il tasto «Rev», che salvano subito come il menu Modo. */
export interface RttyStatus {
  shiftHz: number;
  baud: number;
  /** «Rev» acceso a mano. */
  reverse: boolean;
  /** La radio e' in LSB: mark e space sono gia' invertiti dal motore. */
  radioLsb: boolean;
  onShift(hz: number): void;
  onBaud(baud: number): void;
  onReverse(): void;
}

/** In CW la barra ha la tendina della banda del filtro, che salva subito come il menu Modo, e il tasto «TX = RX». */
export interface CwStatus {
  bandwidthHz: number;
  onBandwidth(hz: number): void;
  /** La velocita' capita in ricezione (WPM interi), se c'e' un segnale. */
  rxWpm?: number | null;
  /** La velocita' di trasmissione diventa quella ricevuta. */
  onTxAtRx?(): void;
}

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
  /** Le misure del segnale («S/N 12 dB · IMD -26 dB», «S/N -8 dB · 22 WPM»); vuoto con l'ascolto spento. */
  measures?: string;
  /** Solo in RTTY: al posto dei dati del modo, tendine di spostamento e velocita' e il tasto «Rev». */
  rtty?: RttyStatus;
  /** Solo in CW: la tendina della banda del filtro accanto ai dati del modo. */
  cw?: CwStatus;
  modeName: string;
  /** Clic sul nome del modo (in RTTY: la finestra «Impostazioni RTTY»); senza, il nome non si clicca. */
  onModeClick?(): void;
  onToggleTx(): void;
  onToggleSquelch(): void;
  onToggleAfc(): void;
  onToggleLock(): void;
  onToggleSnap(): void;
  onFreq(hz: number): void;
  /** Clic sui dati del modo: la finestra delle impostazioni del modo. */
  onModeSettings(): void;
  /** «Soglia dello squelch…» nel menu del tasto destro (in RTTY la finestra «Impostazioni RTTY»). */
  onThreshold(): void;
  /** L'orologio UTC nella barra e «TX lampeggiante», dal menu del tasto destro. */
  showUtc?: boolean;
  onToggleUtc?(): void;
  onToggleFlashTx?(): void;
}

/** L'ora UTC (HH:MM:SS), aggiornata ogni secondo. */
function UtcClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(id);
  }, []);
  return <span className="digi-st digi-st--info">{now.toISOString().slice(11, 19)} UTC</span>;
}

/** La barra di stato: RX/TX, Sq, AFC, Lock, Snap, frequenza audio, dati del modo, misure del segnale (S/N, IMD, WPM) e modo. Data e ora tolte (scelta del proprietario 2026-11-11:
 * l'ora c'e' gia' nella barra in alto di Aurora). */
export function DigiStatusBar(p: Props) {
  const t = useT();
  const [draft, setDraft] = useState<string | null>(null);
  // il menu del tasto destro (su qualunque casella della barra)
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);
  const item = (label: string, checked: boolean | null, run: () => void) => (
    <button
      type="button"
      role={checked === null ? 'menuitem' : 'menuitemcheckbox'}
      aria-checked={checked === null ? undefined : checked}
      onClick={() => {
        setMenu(null);
        run();
      }}
    >
      <span className="digi-st-menu-check">{checked ? '✓' : ''}</span>
      {label}
    </button>
  );
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
        setMenu({ x: e.clientX, y: e.clientY });
      }}
    >
      <PopupMenu at={menu} onClose={closeMenu} className="call-menu digi-st-menu" width={230} height={230} label={t('digi.st.menu')}>
        {item(t('digi.mi.squelch'), p.squelchOn, p.onToggleSquelch)}
        {item(t('digi.mi.afc'), p.afc, p.onToggleAfc)}
        {item('Lock', p.lock, p.onToggleLock)}
        {item(t('digi.mi.snap'), p.snap, p.onToggleSnap)}
        {item(t('digi.st.threshold'), null, p.onThreshold)}
        <hr className="digi-st-menu-sep" />
        {item('UTC', p.showUtc === true, () => p.onToggleUtc?.())}
        {item(t('digi.mi.flashTx'), p.flashTx === true, () => p.onToggleFlashTx?.())}
      </PopupMenu>
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
      {p.rtty ? (
        <>
          <label className="digi-st digi-st--select" title={t('digi.rtty.shift')}>
            <select aria-label={t('digi.rtty.shift')} value={p.rtty.shiftHz} onChange={(e) => p.rtty?.onShift(Number(e.target.value))}>
              {withCurrent(RTTY_SHIFTS, p.rtty.shiftHz).map((v) => (
                <option key={v} value={v}>
                  {v} Hz
                </option>
              ))}
            </select>
          </label>
          <label className="digi-st digi-st--select" title={t('digi.rtty.baud')}>
            <select aria-label={t('digi.rtty.baud')} value={roundBaud(p.rtty.baud)} onChange={(e) => p.rtty?.onBaud(Number(e.target.value))}>
              {withCurrent(RTTY_BAUDS, roundBaud(p.rtty.baud)).map((v) => (
                <option key={v} value={v}>
                  {v} bd
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className={`digi-st${p.rtty.reverse ? ' on' : ''}`}
            aria-pressed={p.rtty.reverse}
            title={p.rtty.radioLsb ? `${t('digi.rtty.reverseTip')} ${t('digi.rtty.lsbTip')}` : t('digi.rtty.reverseTip')}
            onClick={p.rtty.onReverse}
          >
            Rev{p.rtty.radioLsb ? ' (LSB)' : ''}
          </button>
        </>
      ) : null}
      {p.cw ? (
        <label className="digi-st digi-st--select" title={t('digi.cw.bandwidth')}>
          <select aria-label={t('digi.cw.bandwidth')} value={p.cw.bandwidthHz} onChange={(e) => p.cw?.onBandwidth(Number(e.target.value))}>
            {withCurrent(CW_BANDWIDTHS, p.cw.bandwidthHz).map((v) => (
              <option key={v} value={v}>
                {v >= CW_WIDE_HZ ? `${v} Hz, ${t('digi.cw.wide')}` : `${v} Hz`}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      {p.cw?.onTxAtRx ? (
        <button type="button" className="digi-st" disabled={p.cw.rxWpm == null} title={t('digi.mi.txAtRxTip')} onClick={p.cw.onTxAtRx}>
          {t('digi.st.txAtRx')}
        </button>
      ) : null}
      {p.rtty ? null : p.modeData !== '' ? (
        <button type="button" className="digi-st" title={t('digi.st.modeData')} onClick={p.onModeSettings}>
          {p.modeData}
        </button>
      ) : null}
      {p.measures ? (
        <span className="digi-st digi-st--info" title={t('digi.st.measures')}>
          {p.measures}
        </span>
      ) : null}
      {p.showUtc ? <UtcClock /> : null}
      {p.onModeClick ? (
        <button type="button" className="digi-st" title={t('digi.rtty.title')} onClick={p.onModeClick}>
          {p.modeName}
        </button>
      ) : (
        <span className="digi-st digi-st--info">{p.modeName}</span>
      )}
    </div>
  );
}
