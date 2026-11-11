import { useState } from 'react';
import type { DigiSettings } from '../../api/client';
import { useT } from '../../i18n';

/** Gli spostamenti e le velocita' RTTY piu' usati. */
const RTTY_SHIFTS = [23, 85, 160, 170, 182, 200, 240, 350, 425, 450, 850];
const RTTY_BAUDS = [45, 45.45, 50, 56, 75, 100, 110, 150, 200, 300];

export interface Personal {
  myname: string;
  myqth: string;
}

interface Props {
  settings: DigiSettings;
  personal: Personal;
  /** Il livello del segnale adesso (dB), per tarare la soglia dello squelch. */
  levelDb: number;
  onSave(patch: Partial<DigiSettings>): void;
  onPersonal(patch: Partial<Personal>): void;
  onClose(): void;
}

const withCurrent = (list: number[], value: number) => (list.includes(value) ? list : [...list, value].sort((a, b) => a - b));

/** «Mode settings»: frequenze, spostamento e velocita' RTTY, AFC, Lock, Snap, squelch con la sua soglia e i dati personali. */
export function ModeSettings({ settings: s, personal, levelDb, onSave, onPersonal, onClose }: Props) {
  const t = useT();
  const [rx, setRx] = useState(s.rx_freq_hz);
  const [tx, setTx] = useState(s.tx_freq_hz);
  const [db, setDb] = useState(s.squelch_db);

  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.ms.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)}>
      <div className="digi-dialog">
        <header>
          <strong>{t('digi.ms.title')}</strong>
        </header>
        <div className="digi-ms-grid">
          <fieldset>
            <legend>{t('digi.ms.receive')}</legend>
            <label>
              <span>{t('digi.freq.rx')}</span>
              <input type="number" min={200} max={3500} value={rx} onChange={(e) => setRx(Number(e.target.value))} onBlur={() => (rx >= 200 && rx <= 3500 && rx !== s.rx_freq_hz ? onSave({ rx_freq_hz: rx }) : setRx(s.rx_freq_hz))} />
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
            <label>
              <span>{t('digi.ms.squelchDb', { db: Math.round(db) })}</span>
              <input type="range" min={-90} max={0} step={1} value={db} onChange={(e) => setDb(Number(e.target.value))} onPointerUp={() => onSave({ squelch_db: db })} onKeyUp={() => onSave({ squelch_db: db })} />
            </label>
            <p className="sstv-hint">{t('digi.ms.level', { db: Math.round(levelDb) })} — {t('digi.ms.squelchHint')}</p>
          </fieldset>

          <fieldset>
            <legend>{t('digi.tx.title')}</legend>
            <label className="sstv-check">
              <input type="checkbox" checked={!s.tx_follow_rx} onChange={(e) => onSave({ tx_follow_rx: !e.target.checked, tx_freq_hz: e.target.checked ? s.rx_freq_hz : s.tx_freq_hz })} />
              {t('digi.mi.lock')}
            </label>
            <label>
              <span>{t('digi.freq.tx')}</span>
              <input type="number" min={200} max={3500} value={tx} disabled={s.tx_follow_rx} onChange={(e) => setTx(Number(e.target.value))} onBlur={() => (tx >= 200 && tx <= 3500 && tx !== s.tx_freq_hz ? onSave({ tx_freq_hz: tx }) : setTx(s.tx_freq_hz))} />
            </label>
            {s.mode === 'cw' ? (
              <label>
                <span>{t('digi.cw.wpm', { wpm: Math.round(s.cw_wpm) })}</span>
                <input type="range" min={5} max={60} step={1} value={s.cw_wpm} onChange={(e) => onSave({ cw_wpm: Number(e.target.value) })} />
              </label>
            ) : null}
            {s.mode === 'rtty' ? (
              <>
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
                  <select value={s.rtty_baud} onChange={(e) => onSave({ rtty_baud: Number(e.target.value) })}>
                    {withCurrent(RTTY_BAUDS, s.rtty_baud).map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </select>
                </label>
              </>
            ) : null}
            <label className="sstv-check">
              <input type="checkbox" checked={s.auto_receive} onChange={(e) => onSave({ auto_receive: e.target.checked })} />
              {t('digi.autoReceive')}
            </label>
          </fieldset>

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
