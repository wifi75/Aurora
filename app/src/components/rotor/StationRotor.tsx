import { useStationView } from '../../lib/phone';
import { usePublishRotorDx } from '../../lib/rotorDx';
import { useStatusPick } from '../../state/station';
import { RotorCard } from './RotorCard';

/**
 * Il rotore nella Stazione, in tutti i modi del motore (FT8, FT4, JT65, JT9, JT4, Q65, MSK144, FST4, ...) e nelle pagine Fox e Contest: la
 * stessa scheda di PSK/RTTY/CW sopra la pagina, puntata sul DX del QSO in corso. Il DX si scrive anche per il rotore staccato in una
 * finestra a parte. Sul telefono e sul tablet la scheda non c'e' (si usa la pagina Rotore).
 */
export function StationRotor() {
  const station = useStatusPick(['dx_call', 'dx_grid']);
  const call = station?.dx_call ?? '';
  const grid = station?.dx_grid ?? '';
  usePublishRotorDx(call, grid);
  const view = useStationView();
  if (view !== 'desktop') return null;
  return <RotorCard call={call} grid={grid} place="station" />;
}
