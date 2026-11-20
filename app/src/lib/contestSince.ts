/**
 * L'attivazione del contest scelto (ms UTC, la tiene la pagina dei modi testuali in `aurora.ssb.contest.since.<id>`): il core la riceve
 * come `da` per contare, con le stesse regole del log mostrato, i QSO dall'attivazione quando la finestra di date del contest non vale
 * adesso (prima dell'inizio, scaduta, senza date). Cosi' punteggio, doppi e Cabrillo coincidono con cio' che si vede nel log.
 */
let since: number | null = null;

export function setContestSince(ms: number | null): void {
  since = ms !== null && Number.isFinite(ms) && ms > 0 ? ms : null;
}

export function contestSinceMs(): number | null {
  return since;
}

/** `da=<ms>` da aggiungere a una richiesta (una stringa vuota se non c'e'). */
export function sinceParam(first: boolean): string {
  return since === null ? '' : `${first ? '?' : '&'}da=${Math.trunc(since)}`;
}
