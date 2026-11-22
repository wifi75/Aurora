import { useCallback, useEffect, useState } from 'react';
import { useT } from '../../i18n';
import { PopupMenu } from '../PopupMenu';
import { CW_BANDWIDTHS, CW_WIDE_HZ, RTTY_BAUDS, RTTY_SHIFTS, roundBaud, withCurrent } from './ModeSettings';
import { OLIVIA_BANDWIDTHS, OLIVIA_TONES } from '../../lib/olivia';
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
  /** Interruttore «TX = RX»: acceso, la velocita' di trasmissione segue quella ricevuta (campo `cw_wpm_follow_rx`). */
  followRx?: boolean;
  onFollowRx?(on: boolean): void;
  /** Velocita' di trasmissione CW (WPM) e minimo/massimo: i pulsanti − e + accanto ai dati del modo la cambiano di 1 (con Maiuscolo di 5). */
  txWpm?: number;
  onTxWpm?(wpm: number): void;
}

/** In Olivia e Contestia la barra ha le tendine dei toni e della banda, che salvano subito come il menu Modo. */
export interface OliviaStatus {
  tones: number;
  bandwidthHz: number;
  onTones(n: number): void;
  onBandwidth(hz: number): void;
}

export const CW_WPM_MIN = 5;
export const CW_WPM_MAX = 60;

/** La nuova velocita' CW dopo un clic su − (dir -1) o + (dir +1): passo 1, con Maiuscolo 5, entro minimo e massimo. */
export function stepCwWpm(current: number, dir: -1 | 1, shift: boolean): number {
  return Math.min(CW_WPM_MAX, Math.max(CW_WPM_MIN, Math.round(current) + dir * (shift ? 5 : 1)));
}

/** Cio' che l'interruttore «TX = RX» salva: acceso, la velocita' di trasmissione diventa subito quella misurata (se c'e'). */
export function followRxPatch(on: boolean, rxWpm: number | null | undefined): { cw_wpm_follow_rx: boolean; cw_wpm?: number } {
  return on && rxWpm != null ? { cw_wpm_follow_rx: true, cw_wpm: clampCwWpm(rxWpm) } : { cw_wpm_follow_rx: on };
}

/** Interruttore acceso: la velocita' di trasmissione da salvare perche' segua la misura, o null se non c'e' nulla da fare. */
export function followRxTarget(follow: boolean | undefined, rxWpm: number | null | undefined, txWpm: number): number | null {
  if (!follow || rxWpm == null || !Number.isFinite(rxWpm)) return null;
  const next = clampCwWpm(rxWpm);
  return next === txWpm ? null : next;
}

/** Cio' che salvano i tasti − e +: la nuova velocita' e l'interruttore «TX = RX» spento. */
export function wpmStepPatch(wpm: number): { cw_wpm: number; cw_wpm_follow_rx: false } {
  return { cw_wpm: wpm, cw_wpm_follow_rx: false };
}

function clampCwWpm(w: number): number {
  return Math.min(CW_WPM_MAX, Math.max(CW_WPM_MIN, Math.round(w)));
}

interface Props {
  transmitting: boolean;
  /** «Flashing TX»: l'indicatore TX lampeggia durante la trasmissione. */
  flashTx?: boolean;
  squelchOn: boolean;
  /** Il segnale e' sopra la soglia: nella barra si legge «Sq *». */
  squelchOpen: boolean;
  afc: boolean;
  /** Passabanda «a coltello» intorno alla frequenza di ricezione. */
  passband?: boolean;
  onTogglePassband?(): void;
  /** Quanti notch ci sono: il pulsante «Togli i notch» li toglie tutti. */
  notchCount?: number;
  onClearNotches?(): void;
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
  /** Solo in Olivia e Contestia: al posto dei dati del modo, le tendine dei toni e della banda. */
  olivia?: OliviaStatus;
  modeName: string;
  /** Clic sul nome del modo (in RTTY: la finestra «Impostazioni RTTY»); senza, il nome non si clicca. */
  onModeClick?(): void;
  /** I modi della pagina: un clic sul nome del modo in fondo alla barra apre la tendina per cambiarlo. */
  modes?: { id: string; name: string }[];
  modeId?: string;
  onPickMode?(id: string): void;
  /** Il contest dei modi testuali attivo (menu Contest): il nome nella barra, un clic apre l'elenco dei contest. */
  contestName?: string | null;
  onContestClick?(): void;
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
  // la tendina dei modi (clic sul nome del modo): i modi, poi le impostazioni del modo
  const [modeMenu, setModeMenu] = useState<{ x: number; y: number } | null>(null);
  const closeModeMenu = useCallback(() => setModeMenu(null), []);
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
      {p.onTogglePassband ? (
        <button type="button" className={`digi-st${p.passband ? ' on' : ''}`} aria-pressed={p.passband === true} title={t('digi.st.passband')} onClick={p.onTogglePassband}>
          BPF
        </button>
      ) : null}
      {/* «Togli i notch» compare solo quando c'e' almeno un notch */}
      {p.onClearNotches && (p.notchCount ?? 0) > 0 ? (
          <button type="button" className="digi-st digi-st--notch" title={t('digi.notch.clearTip')} onClick={p.onClearNotches}>
            {t('digi.notch.clear')}
            {` (${p.notchCount ?? 0})`}
          </button>
      ) : null}
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
      {p.cw?.onFollowRx ? (
        <button type="button" className={`digi-st${p.cw.followRx ? ' on' : ''}`} aria-pressed={p.cw.followRx === true} title={t('digi.st.txAtRxSwitchTip')} onClick={() => p.cw?.onFollowRx?.(p.cw?.followRx !== true)}>
          {t('digi.st.txAtRx')}
        </button>
      ) : null}
      {p.olivia ? (
        <>
          <label className="digi-st digi-st--select" title={t('digi.olivia.tonesTip')}>
            <select aria-label={t('digi.olivia.tones')} value={p.olivia.tones} onChange={(e) => p.olivia?.onTones(Number(e.target.value))}>
              {OLIVIA_TONES.map((v) => (
                <option key={v} value={v}>
                  {t('digi.olivia.tonesN', { n: v })}
                </option>
              ))}
            </select>
          </label>
          <label className="digi-st digi-st--select" title={t('digi.olivia.bandwidthTip')}>
            <select aria-label={t('digi.olivia.bandwidth')} value={p.olivia.bandwidthHz} onChange={(e) => p.olivia?.onBandwidth(Number(e.target.value))}>
              {OLIVIA_BANDWIDTHS.map((v) => (
                <option key={v} value={v}>
                  {v} Hz
                </option>
              ))}
            </select>
          </label>
        </>
      ) : null}
      {p.rtty || p.olivia ? null : p.modeData !== '' ? (
        <button type="button" className="digi-st" title={t('digi.st.modeData')} onClick={p.onModeSettings}>
          {p.modeData}
        </button>
      ) : null}
      {p.cw?.onTxWpm && p.cw.txWpm != null ? (
        <button type="button" className="digi-st digi-st--step" disabled={p.cw.txWpm <= CW_WPM_MIN} aria-label={t('digi.st.wpmDown')} title={t('digi.st.wpmDown')} onClick={(e) => p.cw?.onTxWpm?.(stepCwWpm(p.cw.txWpm ?? CW_WPM_MIN, -1, e.shiftKey))}>
          −
        </button>
      ) : null}
      {p.cw?.onTxWpm && p.cw.txWpm != null ? (
        <button type="button" className="digi-st digi-st--step" disabled={p.cw.txWpm >= CW_WPM_MAX} aria-label={t('digi.st.wpmUp')} title={t('digi.st.wpmUp')} onClick={(e) => p.cw?.onTxWpm?.(stepCwWpm(p.cw.txWpm ?? CW_WPM_MIN, 1, e.shiftKey))}>
          +
        </button>
      ) : null}
      {p.measures ? (
        <span className="digi-st digi-st--info digi-st--measures" title={t('digi.st.measures')}>
          {p.measures}
        </span>
      ) : null}
      {p.showUtc ? <UtcClock /> : null}
      {p.contestName ? (
        <button type="button" className="digi-st digi-status-contest" title={t('ctest.statusTip')} onClick={p.onContestClick}>
          {t('ctest.status', { name: p.contestName })}
        </button>
      ) : null}
      {p.modes && p.onPickMode ? (
        <>
          <button
            type="button"
            className="digi-st"
            title={t('digi.menu.mode')}
            aria-haspopup="menu"
            aria-expanded={modeMenu !== null}
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              setModeMenu(modeMenu ? null : { x: r.left, y: r.top - Math.min(400, 32 * (p.modes?.length ?? 0) + 60) });
            }}
          >
            {p.modeName}
          </button>
          <PopupMenu at={modeMenu} onClose={closeModeMenu} className="call-menu digi-st-menu" width={230} height={400} label={t('digi.menu.mode')}>
            {p.modes.map((m) => (
              <button
                key={m.id}
                type="button"
                role="menuitemradio"
                aria-checked={m.id === p.modeId}
                onClick={() => {
                  setModeMenu(null);
                  if (m.id !== p.modeId) p.onPickMode?.(m.id);
                }}
              >
                <span className="digi-st-menu-check">{m.id === p.modeId ? '✓' : ''}</span>
                {m.name}
              </button>
            ))}
            {/* SSTV ha la sua pagina: la voce la apre */}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={false}
              onClick={() => {
                setModeMenu(null);
                window.location.hash = '#/sstv';
              }}
            >
              <span className="digi-st-menu-check" />
              {t('nav.sstv')}
            </button>
            {/* anche WEFAX (cartine meteo) ha la sua pagina */}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={false}
              onClick={() => {
                setModeMenu(null);
                window.location.hash = '#/wefax';
              }}
            >
              <span className="digi-st-menu-check" />
              {t('nav.wefax')}
            </button>
            <hr className="digi-st-menu-sep" />
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setModeMenu(null);
                if (p.onModeClick) p.onModeClick();
                else p.onModeSettings();
              }}
            >
              <span className="digi-st-menu-check" />
              {t('digi.st.modeSettings')}
            </button>
          </PopupMenu>
        </>
      ) : p.onModeClick ? (
        <button type="button" className="digi-st" title={t('digi.rtty.title')} onClick={p.onModeClick}>
          {p.modeName}
        </button>
      ) : (
        <span className="digi-st digi-st--info">{p.modeName}</span>
      )}
    </div>
  );
}
