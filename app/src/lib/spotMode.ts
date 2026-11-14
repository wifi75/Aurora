import type { ClusterSpot } from '../api/client';
import { bandPlanRows, type TextMode } from './bandplan';

/** I modi dei testuali che hanno una pagina (l'SSTV ha la sua: qui no). */
export type SpotTextMode = 'psk31' | 'psk63' | 'rtty' | 'cw';

/** Distanza (Hz) da una frequenza di chiamata del piano di banda entro cui lo spot e' di quel modo. */
const PLAN_NEAR_HZ = 2_500;

const PLAN_MODES: Array<[TextMode, SpotTextMode]> = [
  ['RTTY', 'rtty'],
  ['CW', 'cw'],
  ['PSK31', 'psk31'],
];

/** Il modo scritto dal cluster nello spot, se e' uno dei modi testuali. */
function clusterMode(spot: ClusterSpot): SpotTextMode | null {
  const m = (spot.mode ?? '').toUpperCase();
  if (m === 'RTTY') return 'rtty';
  if (m === 'CW') return 'cw';
  if (m === 'PSK63') return 'psk63';
  if (m.startsWith('PSK') || m === 'BPSK') return 'psk31';
  return null;
}

/**
 * Il modo testuale di uno spot, con quest'ordine: prima il piano di banda (lo spot sta vicino alla frequenza di chiamata di un modo solo),
 * poi il modo scritto dal cluster. `null`: lo spot non e' di un modo testuale (FT8, SSB...) o non si sa.
 */
export function textModeOfSpot(spot: ClusterSpot): SpotTextMode | null {
  const hz = spot.freq_khz * 1000;
  const fromPlan = new Set<SpotTextMode>();
  for (const [plan, mode] of PLAN_MODES) {
    if (bandPlanRows(plan).some((r) => Math.abs(r.hz - hz) <= PLAN_NEAR_HZ)) fromPlan.add(mode);
  }
  const fromCluster = clusterMode(spot);
  // il piano dice un modo solo: vale quello (ma PSK63 resta PSK63 se lo dice il cluster: il piano li ha insieme)
  if (fromPlan.size === 1) {
    const only = [...fromPlan][0] ?? null;
    return only === 'psk31' && fromCluster === 'psk63' ? 'psk63' : only;
  }
  // piu' modi alla stessa frequenza di chiamata (40 m: PSK e RTTY a 7040) o nessuno: decide il cluster
  return fromCluster;
}

let pending: ClusterSpot | null = null;

/** Uno spot da sintonizzare appena si apre la pagina dei modi testuali (clic su uno spot RTTY, CW o PSK da un'altra pagina). */
export function setPendingSpot(spot: ClusterSpot): void {
  pending = spot;
}

/** Lo spot in attesa, una volta sola. */
export function takePendingSpot(): ClusterSpot | null {
  const p = pending;
  pending = null;
  return p;
}
