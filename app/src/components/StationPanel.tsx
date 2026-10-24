import type { StationStatus } from '../api/schemas';
import { useT, type T } from '../i18n';
import { bandOf, formatMhz } from '../lib/format';

interface Props {
  status: StationStatus | null;
}

function txLabel(s: StationStatus, t: T): { text: string; hot: boolean } {
  if (s.transmitting) return { text: t('st.txOn'), hot: true };
  if (s.tx_enabled) return { text: t('st.txArmed'), hot: false };
  return { text: t('st.txOff'), hot: false };
}

/** Scheda "Stazione": i dati che JTDX comunica, in forma leggibile. */
export function StationPanel({ status }: Props) {
  const t = useT();
  if (!status) {
    return <p className="muted station-empty">{t('st.waitingJtdx')}</p>;
  }
  const tx = txLabel(status, t);
  return (
    <dl className="facts">
      <div title={t('tip.status.band')}>
        <dt>{t('st.band')}</dt>
        <dd className="data">{bandOf(status.dial_hz) ?? t('st.unknownBand')}</dd>
      </div>
      <div title={t('tip.status.freq')}>
        <dt>{t('st.frequency')}</dt>
        <dd className="data">{status.dial_hz > 0 ? formatMhz(status.dial_hz) : '—'}</dd>
      </div>
      <div title={t('tip.status.mode')}>
        <dt>{t('st.mode')}</dt>
        <dd className="data">{status.mode}</dd>
      </div>
      <div title={t('tip.status.me')}>
        <dt>{t('st.station')}</dt>
        <dd className="data">{`${status.de_call} ${status.de_grid}`.trim()}</dd>
      </div>
      <div>
        <dt>{t('st.reception')}</dt>
        <dd>{status.decoding ? t('st.receiving') : t('st.waitingCycle')}</dd>
      </div>
      <div title={t('tip.onair')}>
        <dt>{t('st.transmission')}</dt>
        <dd className={tx.hot ? 'tx-hot' : undefined}>{tx.text}</dd>
      </div>
    </dl>
  );
}
