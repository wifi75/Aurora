import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { RotorCard, useRotorCardVisible } from '../components/rotor/RotorCard';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { useRotorDx } from '../lib/rotorDx';
import { useStatusPick } from '../state/station';
import '../styles/rotator.css';

/**
 * Il DX per il rotore fuori dalle pagine che lo conoscono (finestra a parte, pagina Rotore): quello scritto dall'ultima pagina aperta nella
 * finestra principale (Stazione o PSK/RTTY/CW), altrimenti quello del QSO della Stazione.
 */
function useCurrentDx(): { call: string; grid: string } {
  const published = useRotorDx();
  const station = useStatusPick(['dx_call', 'dx_grid']);
  return published ?? { call: station?.dx_call ?? '', grid: station?.dx_grid ?? '' };
}

/** La pagina Rotore: la stessa scheda del rotore delle altre pagine, grande; da qui si mette nella Stazione o in una finestra a parte. */
export function RotatorPage({ solo = false }: { solo?: boolean } = {}) {
  const groups = useGroups();
  const elsewhere = groups.some((g) => g.open && g.items.includes('rotator'));
  if (elsewhere && !solo) return <DetachedNote item="rotator" />;
  return solo ? <RotatorSolo /> : <RotatorBody />;
}

/** Nella finestra a parte (secondo schermo): solo la scheda, che riempie la finestra. */
function RotatorSolo() {
  const dx = useCurrentDx();
  return (
    <div className="rotor-solo">
      <RotorCard call={dx.call} grid={dx.grid} place="window" />
    </div>
  );
}

function RotatorBody() {
  const t = useT();
  const dx = useCurrentDx();
  const [onStation, setOnStation] = useRotorCardVisible('station');
  return (
    <div className="rot-page">
      <header className="rot-head">
        <h1>{t('rot.title')}</h1>
        <span className="rot-head-sp" />
        <button type="button" className="rot-btn" aria-pressed={onStation} title={t('tip.rot.onStation')} onClick={() => setOnStation(!onStation)}>
          {t(onStation ? 'rot.onStation.on' : 'rot.onStation.off')}
        </button>
        <DetachButton item="rotator" />
      </header>
      <RotorCard call={dx.call} grid={dx.grid} place="page" />
      <p className="muted rot-hint">{t('rot.clickHint')}</p>
    </div>
  );
}
