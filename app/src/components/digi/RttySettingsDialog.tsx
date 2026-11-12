import { useState } from 'react';
import { useT } from '../../i18n';
import type { DigiSettings, RttyCharset, RttyProfile } from '../../api/client';
import { RTTY_BAUDS, RTTY_SHIFTS, withCurrent } from './ModeSettings';

const BASE_PROFILE: RttyProfile = { shift_hz: 170, baud: 45.45, reverse: false, receiver: 'auto', charset: 'ita2', afc: true, snap: true, squelch: false, uos: true, squelch_snr_db: 0 };

/** I quattro profili di fabbrica (gli stessi del motore): 170/45,45; 22/45,45 con il discriminatore; 30/10; 170/45,45. */
export const RTTY_FACTORY_PROFILES: RttyProfile[] = [BASE_PROFILE, { ...BASE_PROFILE, shift_hz: 22, receiver: 'discriminator' }, { ...BASE_PROFILE, shift_hz: 30, baud: 10 }, BASE_PROFILE];

const CHARSETS: RttyCharset[] = ['ita2', 'lowercase', 'cyrillic', 'swedish'];
/** Gli spostamenti e le velocita' delle tendine, con quelli dei profili stretti e lenti. */
const SHIFTS = [...new Set([22, 30, ...RTTY_SHIFTS])].sort((a, b) => a - b);
const BAUDS = [...new Set([10, ...RTTY_BAUDS])].sort((a, b) => a - b);
const CHARSET_LABEL = { ita2: 'digi.rtty.cs.ita2', lowercase: 'digi.rtty.cs.lower', cyrillic: 'digi.rtty.cs.cyrillic', swedish: 'digi.rtty.cs.swedish' } as const;

/** Il profilo che corrisponde alle impostazioni attuali (quello scelto, aggiornato con quello che si usa adesso). */
function currentProfile(s: DigiSettings, base: RttyProfile): RttyProfile {
  return {
    ...base,
    shift_hz: s.rtty_shift_hz,
    baud: s.rtty_baud,
    reverse: s.rtty_reverse,
    receiver: s.rtty_receiver,
    charset: s.rtty_charset,
    afc: s.afc_on,
    snap: s.snap_on,
    squelch: s.squelch_on,
    uos: s.rtty_uos,
    squelch_snr_db: s.rtty_squelch_snr_db,
  };
}

/** La finestra «Impostazioni RTTY»: tutto in un posto, con quattro profili (Set 1-4) che si scelgono con i pallini. OK salva il profilo
 * scelto e lo mette in uso; Annulla lascia tutto com'era. */
export function RttySettingsDialog({ settings: s, onApply, onClose }: { settings: DigiSettings; onApply(patch: Partial<DigiSettings>): void; onClose(): void }) {
  const t = useT();
  const [profiles, setProfiles] = useState<RttyProfile[]>(() => {
    const saved = s.rtty_profiles.length === 4 ? s.rtty_profiles : RTTY_FACTORY_PROFILES;
    return saved.map((p, i) => (i === s.rtty_profile ? currentProfile(s, p) : p));
  });
  const [active, setActive] = useState(Math.min(3, Math.max(0, s.rtty_profile)));
  const [rxHz, setRxHz] = useState(String(Math.round(s.rx_freq_hz)));
  const [txHz, setTxHz] = useState(String(Math.round(s.tx_freq_hz)));
  const p = profiles[active] ?? BASE_PROFILE;
  const set = (patch: Partial<RttyProfile>) => setProfiles((all) => all.map((x, i) => (i === active ? { ...x, ...patch } : x)));
  const hz = (v: string, fallback: number) => {
    const n = Number(v.replace(',', '.'));
    return Number.isFinite(n) && n >= 200 && n <= 3500 ? n : fallback;
  };
  const ok = () => {
    const rx = hz(rxHz, s.rx_freq_hz);
    const tx = hz(txHz, s.tx_freq_hz);
    onApply({
      rtty_profiles: profiles,
      rtty_profile: active,
      rtty_shift_hz: p.shift_hz,
      rtty_baud: p.baud,
      rtty_reverse: p.reverse,
      rtty_receiver: p.receiver,
      rtty_charset: p.charset,
      afc_on: p.afc,
      snap_on: p.snap,
      squelch_on: p.squelch,
      rtty_uos: p.uos,
      rtty_squelch_snr_db: p.squelch_snr_db,
      rx_freq_hz: rx,
      tx_freq_hz: tx,
      tx_follow_rx: Math.abs(tx - rx) < 0.5 ? s.tx_follow_rx : false,
    });
    onClose();
  };
  const check = (key: 'reverse' | 'afc' | 'snap' | 'squelch' | 'uos', label: string) => (
    <label className="rtty-check">
      <input type="checkbox" checked={p[key]} onChange={(e) => set({ [key]: e.target.checked })} />
      <span>{label}</span>
    </label>
  );
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.rtty.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : e.key === 'Enter' && e.target instanceof HTMLInputElement ? ok() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="digi-dialog rtty-dialog">
        <header className="cop-head">
          <strong>{t('digi.rtty.title')}</strong>
          <div className="rtty-sets" role="radiogroup" aria-label={t('digi.rtty.profiles')}>
            {profiles.map((_, i) => (
              <label key={i} className="rtty-set">
                <input type="radio" name="rtty-set" checked={i === active} onChange={() => setActive(i)} />
                <span>{t('digi.rtty.set', { n: i + 1 })}</span>
              </label>
            ))}
          </div>
        </header>
        <div className="rtty-body">
          <div className="rtty-col">
            {check('reverse', t('digi.rtty.reverse'))}
            {check('afc', t('digi.mi.afc'))}
            {check('snap', t('digi.mi.snap'))}
            {check('squelch', t('digi.mi.squelch'))}
            {check('uos', t('digi.rtty.uos'))}
          </div>
          <label className="rtty-sq" title={t('digi.rtty.squelchSnrHint')}>
            <input type="range" className="rtty-vertical" min={0} max={40} step={1} value={p.squelch_snr_db} onChange={(e) => set({ squelch_snr_db: Number(e.target.value) })} aria-label={t('digi.rtty.squelchSnr', { db: p.squelch_snr_db })} />
            <span>{t('digi.rtty.squelchSnr', { db: p.squelch_snr_db })}</span>
          </label>
          <div className="rtty-col">
            <label>
              <span>{t('digi.rtty.txHz')}</span>
              <input type="text" inputMode="decimal" value={txHz} onChange={(e) => setTxHz(e.target.value)} />
            </label>
            <label>
              <span>{t('digi.rtty.rxHz')}</span>
              <input type="text" inputMode="decimal" value={rxHz} onChange={(e) => setRxHz(e.target.value)} />
            </label>
            <label title={t('digi.rtty.receiverTip')}>
              <span>{t('digi.rtty.receiver')}</span>
              <select value={p.receiver} onChange={(e) => set({ receiver: e.target.value as RttyProfile['receiver'] })}>
                <option value="auto">{t('digi.rtty.receiverAuto')}</option>
                <option value="filters">{t('digi.rtty.receiverFilters')}</option>
                <option value="discriminator">{t('digi.rtty.receiverDisc')}</option>
              </select>
            </label>
          </div>
          <div className="rtty-col">
            <label>
              <span>{t('digi.rtty.shift')}</span>
              <select value={p.shift_hz} onChange={(e) => set({ shift_hz: Number(e.target.value) })}>
                {withCurrent(SHIFTS, p.shift_hz).map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t('digi.rtty.baud')}</span>
              <select value={p.baud} onChange={(e) => set({ baud: Number(e.target.value) })}>
                {withCurrent(BAUDS, p.baud).map((v) => (
                  <option key={v} value={v}>
                    {String(v).replace('.', ',')}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t('digi.rtty.charset')}</span>
              <select value={p.charset} onChange={(e) => set({ charset: e.target.value as RttyCharset })}>
                {CHARSETS.map((c) => (
                  <option key={c} value={c}>
                    {t(CHARSET_LABEL[c])}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>
        <footer className="rtty-foot">
          <button type="button" className="glass sstv-btn" onClick={() => set(RTTY_FACTORY_PROFILES[active] ?? {})}>
            {t('digi.rtty.factory')}
          </button>
          <span className="rtty-spacer" />
          <button type="button" className="glass sstv-btn" onClick={ok}>
            {t('confirm.ok')}
          </button>
          <button type="button" className="glass sstv-btn" onClick={onClose}>
            {t('confirm.cancel')}
          </button>
        </footer>
      </div>
    </div>
  );
}
