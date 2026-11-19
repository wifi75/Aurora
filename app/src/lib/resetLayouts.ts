/**
 * «Riporta tutte le finestre alla disposizione iniziale»: toglie dal browser tutte le disposizioni salvate (finestre della pagina FT8, cascata
 * e riquadri in basso, larghezze e altezze, colonne delle tabelle, finestre staccate, riquadri della pagina Satelliti), cosi' l'interfaccia
 * riparte com'era subito dopo la prima installazione. Le scelte che non sono di disposizione (tema, lingua, nome, filtri, suoni) non si toccano.
 * Togliere la voce (e non scrivere quella di fabbrica) fa si' che anche il server delle preferenze la dimentichi (vedi `syncSettings`).
 */

/** Le voci `aurora-...` che sono disposizione di finestre. */
const DASH_KEYS = [
  'aurora-board',
  'aurora-board-staccate',
  'aurora-bottom',
  'aurora-finestre',
  'aurora-finestra-rx',
  'aurora-layout',
  'aurora-dxcc-card',
  'aurora-rotor-card',
  'aurora-newdx-pos',
  'aurora-display-scala',
] as const;

/** Le voci `aurora.` che sono disposizione (larghezze, altezze, griglie). */
const DOT_KEYS = [
  'cluster.splitdx',
  'digi.bandh',
  'digi.log.grid',
  'digi.logcols2',
  'digi.logw',
  'digi.logw3',
  'digi.macrorows',
  'digi.macrow',
  'digi.radarh',
  'digi.radarw',
  'digi.rxfrac',
  'digi.rxfrac.cw',
  'digi.ssbcols',
  'digi.wfh2',
  'digi.wfh2.cw',
  'digi.wfw',
  'digi.winw',
  'digi.winw.cw',
  'sat.grid.all',
  'sat.grid.passes',
  'sat.layout',
  'sw.macroRows',
] as const;

/** `true` se la voce del browser descrive dove sta una finestra o quanto e' grande. */
export function isLayoutKey(key: string): boolean {
  if ((DASH_KEYS as readonly string[]).includes(key)) return true;
  if (!key.startsWith('aurora.')) return false;
  const rest = key.slice('aurora.'.length);
  // tutte le colonne delle tabelle (`aurora.band.cols`, `aurora.digi.radar.cols`...) e le finestre in piu' della ricezione
  return rest.endsWith('.cols') || rest.startsWith('xrx.') || (DOT_KEYS as readonly string[]).includes(rest);
}

type Store = Pick<Storage, 'key' | 'length' | 'removeItem'>;

/** Toglie dal browser tutte le disposizioni salvate; restituisce quante voci ha tolto. */
export function clearLayouts(storage: Store): number {
  const gone: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    if (key !== null && isLayoutKey(key)) gone.push(key);
  }
  for (const key of gone) storage.removeItem(key);
  return gone.length;
}
