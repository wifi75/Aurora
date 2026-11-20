/*
 * Un solo suono per avviso. La logica degli avvisi gira alla radice dell'app (motore invisibile) e dentro la pagina
 * Satelliti: i due motori vedono gli stessi avvisi, ma ognuno ha il suo registro. Prima di suonare, ognuno «prenota» qui
 * l'avviso (tipo, satellite, fine del passaggio): vince il primo, l'altro tace. Il registro e' di questa finestra.
 */

import { isDone, type AlertRecord } from './satAlerts';

let claimed: AlertRecord[] = [];
const KEEP_S = 3600;

/** Vero se questo avviso non e' ancora stato suonato da nessun motore (e lo segna come suonato). */
export function claimSound(ev: AlertRecord, nowUnix: number): boolean {
  claimed = claimed.filter((r) => r.los > nowUnix - KEEP_S);
  if (isDone(claimed, ev.kind, ev.norad, ev.los)) return false;
  claimed.push({ kind: ev.kind, norad: ev.norad, los: ev.los });
  return true;
}

export function resetClaims(): void {
  claimed = [];
}
