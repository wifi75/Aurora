import type { KeyboardEvent } from 'react';
import '../../styles/sat-label.css';
import { useT } from '../../i18n';
import { getSat, putSat } from '../../api/client';
import { mmss } from '../../lib/satGeo';
import { pickSatellite } from '../../lib/satSelection';
import { satLabelStore, useSatLabel } from '../../lib/satLabel';

const activate = (fn: () => void) => (e: KeyboardEvent) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    fn();
  }
};

/**
 * L'etichetta «satellite in arrivo» della barra in alto: nome, conto alla rovescia, ✕ per chiuderla. Un clic sceglie quel satellite e porta
 * alla pagina Satelliti. Sta dentro la barra, tra Rotore e la spia di salute.
 */
export function SatAlertLabel() {
  const t = useT();
  const { label } = useSatLabel();
  if (!label) return null;
  // il nome si accorcia con i puntini se e' lungo (RS-44 & BREEZE-KM R/B), il conto alla rovescia resta sempre intero
  const SAT = '';
  const text = t(label.phase === 'soon' ? 'sat.label.soon' : 'sat.label.now', { sat: SAT, time: mmss(label.seconds) });
  const [before, after = ''] = text.split(SAT);
  const plain = text.replace(SAT, label.name);
  // il clic sceglie quel satellite nella pagina Satelliti (spuntato e selezionato) e la apre
  const open = () => {
    void getSat()
      .then((snap) => putSat({ ...snap.settings, ...pickSatellite(snap.settings, label.norad), transponder: 0, down_hz: null }))
      .catch(() => undefined)
      .finally(() => {
        location.hash = '#/satelliti';
      });
  };
  const close = () => satLabelStore.dismiss({ norad: label.norad, los: label.los });
  return (
    <span className="sat-label" role="status" data-phase={label.phase}>
      <span className="sat-label-main" role="link" tabIndex={0} title={t('sat.label.open')} onClick={open} onKeyDown={activate(open)}>
        <svg className="sat-label-icon" viewBox="0 0 24 24" width="1.1em" height="1.1em" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M9 3l3 3-3 3-3-3Z" />
          <path d="M15 9l3 3-3 3-3-3Z" />
          <path d="M10.5 10.5l3 3" />
          <path d="M4 20a8 8 0 0 0 8-8" />
          <path d="M4 15a3 3 0 0 0 3-3" />
        </svg>
        <span className="sat-label-text" title={plain}>
          {before}
          <span className="sat-label-name">{label.name}</span>
          {after}
        </span>
      </span>
      <span className="sat-label-close" role="button" tabIndex={0} title={t('sat.label.close')} aria-label={t('sat.label.close')} onClick={close} onKeyDown={activate(close)}>
        ✕
      </span>
    </span>
  );
}
