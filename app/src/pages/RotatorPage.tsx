import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { RotorCard, useRotorCardVisible } from '../components/rotor/RotorCard';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { useRotorDx } from '../lib/rotorDx';
import { useStatusPick } from '../state/station';
import '../styles/rotator.css';

/**
 * Il DX per il rotore fuori dalle pagine che lo conoscono. Nella pagina Rotore della finestra principale nessun'altra pagina e' aperta, quindi
 * vale prima il DX vivo del QSO della Stazione e solo senza quello il DX scritto dall'ultima pagina (che puo' essere vecchio). Nella finestra a
 * parte (`solo`) vale il DX scritto dalla pagina aperta nella finestra principale (Stazione o PSK/RTTY/CW), ma se e' vuoto si usa quello vivo.
 */
function useCurrentDx(solo: boolean): { call: string; grid: string } {
  const published = useRotorDx();
  const station = useStatusPick(['dx_call', 'dx_grid']);
  const live = { call: station?.dx_call?.trim() ?? '', grid: station?.dx_grid?.trim() ?? '' };
  const liveSet = live.call !== '' || live.grid !== '';
  const publishedSet = published !== null && (published.call !== '' || published.grid !== '');
  if (solo) return publishedSet ? published : live;
  return liveSet || !published ? live : published;
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
  const dx = useCurrentDx(true);
  return (
    <div className="rotor-solo">
      <RotorCard call={dx.call} grid={dx.grid} place="window" />
    </div>
  );
}

function RotatorBody() {
  const t = useT();
  const dx = useCurrentDx(false);
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
