import { useEffect, useRef, useState } from 'react';
import { useT } from '../../i18n';
import type { Key } from '../../i18n';
import { ALERT_SOUNDS, type AlertKind, type SatAlertSettings, type SoundChoice } from '../../lib/satAlerts';
import { armAudio } from '../../lib/satAlertSound';
import { Icon } from '../Icon';

interface Props {
  settings: SatAlertSettings;
  blocked: boolean;
  update(patch: Partial<SatAlertSettings>): void;
  test(kind: AlertKind): void;
}

const EVENTS: { kind: AlertKind; field: 'soundPre' | 'soundAos' | 'soundLos'; label: Key }[] = [
  { kind: 'pre', field: 'soundPre', label: 'sat.alert.ev.pre' },
  { kind: 'aos', field: 'soundAos', label: 'sat.alert.ev.aos' },
  { kind: 'los', field: 'soundLos', label: 'sat.alert.ev.los' },
];

/** Il pulsante «Avvisi» (campanella) della pagina Satelliti, con il riquadro delle scelte e il pulsante «Prova» per ogni suono. */
export function SatAlertsPanel({ settings, blocked, update, test }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('pointerdown', away);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('pointerdown', away);
      document.removeEventListener('keydown', esc);
    };
  }, [open]);

  return (
    <div className="sat-alerts" ref={box}>
      <button
        type="button"
        className={`glass sat-alerts-btn${settings.enabled ? ' on' : ''}`}
        aria-label={t('sat.alert.title')}
        aria-expanded={open}
        aria-haspopup="dialog"
        title={blocked && settings.enabled ? t('sat.alert.unlock') : t('sat.alert.tip')}
        onClick={() => {
          armAudio();
          setOpen((v) => !v);
        }}
      >
        <Icon name="bell" size={21} />
        {blocked && settings.enabled ? <Icon name="speaker-off" size={11} className="sat-alerts-badge" /> : null}
      </button>
      {open ? (
        <div className="sat-alerts-pop" role="dialog" aria-label={t('sat.alert.title')}>
          <label className="sat-alerts-row sat-alerts-row--check">
            <input type="checkbox" checked={settings.enabled} onChange={(e) => update({ enabled: e.target.checked })} />
            <span>{t('sat.alert.on')}</span>
          </label>
          <label className="sat-alerts-row">
            <span>
              {t('sat.alert.volume')}: <b>{settings.volume}</b>
            </span>
            <input type="range" min={0} max={100} step={1} value={settings.volume} onChange={(e) => update({ volume: Number(e.target.value) })} />
          </label>
          <label className="sat-alerts-row sat-alerts-row--inline">
            <span>{t('sat.alert.lead')}</span>
            <input type="number" min={0} max={30} value={settings.leadMin} onChange={(e) => update({ leadMin: Math.max(0, Math.min(30, Math.round(Number(e.target.value) || 0))) })} />
          </label>
          <label className="sat-alerts-row sat-alerts-row--check">
            <input type="checkbox" checked={settings.onlySelected} onChange={(e) => update({ onlySelected: e.target.checked })} />
            <span>{t('sat.alert.only')}</span>
          </label>
          {EVENTS.map((ev) => (
            <div className="sat-alerts-row sat-alerts-row--inline" key={ev.kind}>
              <span>{t(ev.label)}</span>
              <select aria-label={t(ev.label)} value={settings[ev.field]} onChange={(e) => update({ [ev.field]: e.target.value as SoundChoice })}>
                <option value="off">{t('sat.alert.snd.off')}</option>
                {ALERT_SOUNDS.map((s) => (
                  <option key={s} value={s}>
                    {t(`sat.alert.snd.${s}` as Key)}
                  </option>
                ))}
              </select>
              <button type="button" className="glass sstv-btn" disabled={settings[ev.field] === 'off'} onClick={() => test(ev.kind)}>
                {t('sat.alert.test')}
              </button>
            </div>
          ))}
          <p className="sat-muted sat-small">{t('sat.alert.hint')}</p>
        </div>
      ) : null}
    </div>
  );
}
