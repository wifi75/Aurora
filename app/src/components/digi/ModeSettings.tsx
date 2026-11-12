import { Fragment, useState, type ReactNode } from 'react';
import type { DigiSettings } from '../../api/client';
import { useT } from '../../i18n';

/** Gli spostamenti e le velocita' RTTY piu' usati (gli stessi elenchi di fldigi, piu' 450 Hz). */
export const RTTY_SHIFTS = [23, 85, 160, 170, 182, 200, 240, 350, 425, 450, 850];
export const RTTY_BAUDS = [45, 45.45, 50, 56, 75, 100, 110, 150, 200, 300];
/** Le bande del filtro CW (le stesse del motore, 150 di base come fldigi; 1000 Hz e' la banda larga per la panoramica). */
export const CW_BANDWIDTHS = [50, 100, 150, 250, 500, 1000];
/** Oltre questa banda il filtro CW e' «largo», per la panoramica. */
export const CW_WIDE_HZ = 1000;

export interface Personal {
  myname: string;
  myqth: string;
}

interface Props {
  settings: DigiSettings;
  personal: Personal;
  /** Il livello del segnale adesso (dB). */
  levelDb: number;
  /** RTTY e CW: il rapporto segnale/rumore adesso (dB), per tarare la soglia dello squelch. */
  snrDb?: number | null;
  /** PSK: la qualita' della fase adesso (0-100), per tarare la soglia dello squelch. */
  quality?: number | null;
  onSave(patch: Partial<DigiSettings>): void;
  onPersonal(patch: Partial<Personal>): void;
  onClose(): void;
}

/** La velocita' RTTY arriva dal motore in virgola mobile corta (45,45 → 45,4500007…): si arrotonda a due decimali per mostrarla e confrontarla. */
export const roundBaud = (v: number) => Math.round(v * 100) / 100;

export const withCurrent = (list: number[], value: number) => (list.includes(value) ? list : [...list, value].sort((a, b) => a - b));

type Section = (title: string, children: ReactNode) => ReactNode;

/** Nel dialogo della pagina ogni parte e' un riquadro con legenda. */
const fieldset: Section = (title, children) => (
  <fieldset key={title}>
    <legend>{title}</legend>
    {children}
  </fieldset>
);

/**
 * Ricezione e trasmissione di PSK, RTTY e CW: frequenze, spostamento e velocita' RTTY, banda e SOM del CW, AFC, Lock, Snap, squelch con
 * la soglia del modo (qualita' della fase in PSK, rapporto segnale/rumore in RTTY e CW: mai il livello, che dipende dal volume).
 * Nel dialogo della pagina si vedono solo le voci del modo in uso; nelle Impostazioni (`allModes`) quelle di tutti i modi, in un riquadro
 * «Per modo». `section` disegna ogni parte (riquadro del dialogo o riquadro delle Impostazioni).
 */
export function ModeSettingsFields({
  settings: s,
  snrDb = null,
  quality = null,
  onSave,
  allModes = false,
  section = fieldset,
}: {
  settings: DigiSettings;
  levelDb: number | null;
  snrDb?: number | null;
  quality?: number | null;
  onSave(patch: Partial<DigiSettings>): void;
  allModes?: boolean;
  section?: Section;
}) {
  const t = useT();
  // numeri interi: il valore del motore e' un numero a 32 bit e mostrava code come 2037,800048828125 (niente cifre dopo la virgola,
  // richiesta del proprietario)
  const hz1 = (v: number) => Math.round(v);
  const [rx, setRx] = useState(hz1(s.rx_freq_hz));
  const [tx, setTx] = useState(hz1(s.tx_freq_hz));
  const [snr, setSnr] = useState(s.rtty_squelch_snr_db);
  const [pskSq, setPskSq] = useState(s.psk_squelch);
  const [cwSnr, setCwSnr] = useState(s.cw_squelch_snr_db);
  const isRtty = !allModes && s.mode === 'rtty';
  const isCw = !allModes && s.mode === 'cw';
  // PSK in ricezione: la soglia dello squelch sulla qualita' della fase (0-100, come fldigi), che non dipende dal volume
  const pskRx = (
    <Fragment key="pskRx">
      <label>
        <span>{t('digi.psk.squelch', { q: Math.round(pskSq) })}</span>
        <input type="range" min={0} max={100} step={1} value={pskSq} onChange={(e) => setPskSq(Number(e.target.value))} onPointerUp={() => onSave({ psk_squelch: pskSq })} onKeyUp={() => onSave({ psk_squelch: pskSq })} />
      </label>
      <label className="sstv-check" title={t('digi.band.miPskTip')}>
        <input type="checkbox" checked={s.band_psk !== false} onChange={(e) => onSave({ band_psk: e.target.checked })} />
        {t('digi.band.miPsk')}
      </label>
      <p className="sstv-hint">
        {quality === null ? null : <>{t('digi.psk.qualityNow', { q: Math.round(quality) })} — </>}
        {t('digi.psk.squelchHint')}
      </p>
    </Fragment>
  );
  // CW in ricezione: banda del filtro, decodifica SOM e soglia dello squelch sul rapporto segnale/rumore
  const cwRx = (
    <Fragment key="cwRx">
      <label title={t('digi.cw.bandwidthTip')}>
        <span>{t('digi.cw.bandwidth')}</span>
        <select value={s.cw_bandwidth_hz} onChange={(e) => onSave({ cw_bandwidth_hz: Number(e.target.value) })}>
          {CW_BANDWIDTHS.map((v) => (
            <option key={v} value={v}>
              {v >= CW_WIDE_HZ ? `${v} Hz, ${t('digi.cw.wide')}` : `${v} Hz`}
            </option>
          ))}
        </select>
      </label>
      <label className="sstv-check" title={t('digi.cw.somTip')}>
        <input type="checkbox" checked={s.cw_som} onChange={(e) => onSave({ cw_som: e.target.checked })} />
        {t('digi.cw.som')}
      </label>
      <label className="sstv-check" title={t('digi.band.miTip')}>
        <input type="checkbox" checked={s.band_cw !== false} onChange={(e) => onSave({ band_cw: e.target.checked })} />
        {t('digi.band.mi')}
      </label>
      <label className="sstv-check" title={t('digi.band.spotsTip')}>
        <input type="checkbox" checked={s.band_spots !== false} onChange={(e) => onSave({ band_spots: e.target.checked })} />
        {t('digi.band.spots')}
      </label>
      <label className="sstv-check">
        <input type="checkbox" checked={s.band_spots_cq_only === true} disabled={s.band_spots === false} onChange={(e) => onSave({ band_spots_cq_only: e.target.checked })} />
        {t('digi.band.spotsCqOnly')}
      </label>
      <label title={t('digi.band.validationTip')}>
        <span>{t('digi.band.validation')}</span>
        <select value={s.band_validation ?? 'normal'} onChange={(e) => onSave({ band_validation: e.target.value })}>
          <option value="minimal">{t('digi.band.valMinimal')}</option>
          <option value="normal">{t('digi.band.valNormal')}</option>
          <option value="strict">{t('digi.band.valStrict')}</option>
          <option value="maximum">{t('digi.band.valMaximum')}</option>
        </select>
      </label>
      <label>
        <span>{t('digi.cw.squelchSnr', { db: Math.round(cwSnr) })}</span>
        <input type="range" min={-30} max={20} step={1} value={cwSnr} onChange={(e) => setCwSnr(Number(e.target.value))} onPointerUp={() => onSave({ cw_squelch_snr_db: cwSnr })} onKeyUp={() => onSave({ cw_squelch_snr_db: cwSnr })} />
      </label>
      <p className="sstv-hint">
        {snrDb === null ? null : <>{t('digi.rtty.snrNow', { db: Math.round(snrDb) })} — </>}
        {t('digi.cw.squelchSnrHint')}
      </p>
    </Fragment>
  );
  // RTTY in ricezione: Rev, UOS, il ricevitore e la soglia dello squelch sul rapporto segnale/rumore (l'RTTY non usa la soglia in dB)
  const rttyRx = (
    <Fragment key="rttyRx">
      <label className="sstv-check" title={t('digi.rtty.reverseTip')}>
        <input type="checkbox" checked={s.rtty_reverse} onChange={(e) => onSave({ rtty_reverse: e.target.checked })} />
        {t('digi.rtty.reverse')}
      </label>
      <label className="sstv-check" title={t('digi.rtty.uosTip')}>
        <input type="checkbox" checked={s.rtty_uos} onChange={(e) => onSave({ rtty_uos: e.target.checked })} />
        {t('digi.rtty.uos')}
      </label>
      <label title={t('digi.rtty.receiverTip')}>
        <span>{t('digi.rtty.receiver')}</span>
        <select value={s.rtty_receiver} onChange={(e) => onSave({ rtty_receiver: e.target.value as DigiSettings['rtty_receiver'] })}>
          <option value="auto">{t('digi.rtty.receiverAuto')}</option>
          <option value="filters">{t('digi.rtty.receiverFilters')}</option>
          <option value="discriminator">{t('digi.rtty.receiverDisc')}</option>
        </select>
      </label>
      <label>
        <span>{t('digi.rtty.squelchSnr', { db: Math.round(snr) })}</span>
        <input type="range" min={0} max={40} step={1} value={snr} onChange={(e) => setSnr(Number(e.target.value))} onPointerUp={() => onSave({ rtty_squelch_snr_db: snr })} onKeyUp={() => onSave({ rtty_squelch_snr_db: snr })} />
      </label>
      <p className="sstv-hint">
        {snrDb === null ? null : <>{t('digi.rtty.snrNow', { db: Math.round(snrDb) })} — </>}
        {t('digi.rtty.squelchSnrHint')}
      </p>
    </Fragment>
  );
  const cw = (
    <label key="cw">
      <span>{t('digi.cw.wpm', { wpm: Math.round(s.cw_wpm) })}</span>
      <input type="range" min={5} max={60} step={1} value={s.cw_wpm} onChange={(e) => onSave({ cw_wpm: Number(e.target.value) })} />
    </label>
  );
  const rtty = (
    <Fragment key="rtty">
      <label>
        <span>{t('digi.rtty.shift')}</span>
        <select value={s.rtty_shift_hz} onChange={(e) => onSave({ rtty_shift_hz: Number(e.target.value) })}>
          {withCurrent(RTTY_SHIFTS, s.rtty_shift_hz).map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{t('digi.rtty.baud')}</span>
        <select value={roundBaud(s.rtty_baud)} onChange={(e) => onSave({ rtty_baud: Number(e.target.value) })}>
          {withCurrent(RTTY_BAUDS, roundBaud(s.rtty_baud)).map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      </label>
    </Fragment>
  );

  return (
    <>
      {section(
        t('digi.ms.receive'),
        <>
          <label>
            <span>{t('digi.freq.rx')}</span>
            <input type="number" min={200} max={3500} step={1} value={rx} onChange={(e) => setRx(Number(e.target.value))} onBlur={() => (rx >= 200 && rx <= 3500 && rx !== hz1(s.rx_freq_hz) ? onSave({ rx_freq_hz: rx }) : setRx(hz1(s.rx_freq_hz)))} />
          </label>
          <label className="sstv-check">
            <input type="checkbox" checked={s.afc_on} onChange={(e) => onSave({ afc_on: e.target.checked })} />
            {t('digi.mi.afc')}
          </label>
          <label className="sstv-check">
            <input type="checkbox" checked={s.snap_on} onChange={(e) => onSave({ snap_on: e.target.checked })} />
            {t('digi.mi.snap')}
          </label>
          <label className="sstv-check">
            <input type="checkbox" checked={s.squelch_on} onChange={(e) => onSave({ squelch_on: e.target.checked })} />
            {t('digi.mi.squelch')}
          </label>
          {allModes ? null : isRtty ? rttyRx : isCw ? cwRx : pskRx}
        </>,
      )}
      {section(
        t('digi.tx.title'),
        <>
          <label className="sstv-check">
            <input type="checkbox" checked={!s.tx_follow_rx} onChange={(e) => onSave({ tx_follow_rx: !e.target.checked, tx_freq_hz: e.target.checked ? s.rx_freq_hz : s.tx_freq_hz })} />
            {t('digi.mi.lock')}
          </label>
          <label>
            <span>{t('digi.freq.tx')}</span>
            <input type="number" min={200} max={3500} step={1} value={tx} disabled={s.tx_follow_rx} onChange={(e) => setTx(Number(e.target.value))} onBlur={() => (tx >= 200 && tx <= 3500 && tx !== hz1(s.tx_freq_hz) ? onSave({ tx_freq_hz: tx }) : setTx(hz1(s.tx_freq_hz)))} />
          </label>
          {!allModes && s.mode === 'cw' ? cw : null}
          {!allModes && s.mode === 'rtty' ? rtty : null}
          <label className="sstv-check">
            <input type="checkbox" checked={s.auto_receive} onChange={(e) => onSave({ auto_receive: e.target.checked })} />
            {t('digi.autoReceive')}
          </label>
        </>,
      )}
      {allModes
        ? section(
            t('sw.digi.perMode'),
            <>
              <b className="sw-digi-mode">PSK31 · PSK63</b>
              {pskRx}
              <b className="sw-digi-mode">RTTY</b>
              {rtty}
              {rttyRx}
              <b className="sw-digi-mode">CW</b>
              {cw}
              {cwRx}
            </>,
          )
        : null}
    </>
  );
}

/** Le impostazioni del modo: le voci del modo in uso e i dati personali (gli stessi di Impostazioni › Stazione › Dati personali). */
export function ModeSettings({ settings: s, personal, levelDb, snrDb = null, quality = null, onSave, onPersonal, onClose }: Props) {
  const t = useT();
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.ms.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)}>
      <div className="digi-dialog">
        <header>
          <strong>{t('digi.ms.title')}</strong>
        </header>
        <div className="digi-ms-grid">
          <ModeSettingsFields settings={s} levelDb={levelDb} snrDb={snrDb} quality={quality} onSave={onSave} />
          <fieldset>
            <legend>{t('digi.ms.personal')}</legend>
            <label>
              <span>{t('digi.f.myname')}</span>
              <input value={personal.myname} maxLength={30} onChange={(e) => onPersonal({ myname: e.target.value })} />
            </label>
            <label>
              <span>{t('digi.f.myqth')}</span>
              <input value={personal.myqth} maxLength={40} onChange={(e) => onPersonal({ myqth: e.target.value })} />
            </label>
          </fieldset>
        </div>
        <footer>
          <span style={{ flex: 1 }} />
          <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={onClose}>
            {t('digi.ms.close')}
          </button>
        </footer>
      </div>
    </div>
  );
}
