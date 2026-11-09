import { getEngineSettings, putEngineSettings } from '../api/client';

/** Le evidenziazioni del motore (Impostazioni → Segnalazioni) che servono perche' una novita' venga calcolata. */
const NEEDS: Record<string, string[]> = {
  new_dxcc: ['newDXCC'],
  new_dxcc_band: ['newDXCC', 'newDXCCBand'],
  new_zone: ['newCQZ'],
  new_zone_band: ['newCQZ', 'newCQZBand'],
  new_itu: ['newITUZ'],
  new_itu_band: ['newITUZ', 'newITUZBand'],
  new_grid: ['newGrid'],
  new_grid_band: ['newGrid', 'newGridBand'],
  new_px: ['newPx'],
  new_px_band: ['newPx', 'newPxBand'],
  new_call: ['newCall'],
  new_call_band: ['newCall', 'newCallBand'],
};

/**
 * Scegliendo una novita' nel filtro o nelle Automazioni si accende da sola la sua evidenziazione, se era spenta: senza, il motore non la
 * calcola e il filtro restava vuoto (ON7KEC, issue #7: «mostra solo le mie trasmissioni»). Gli errori si ignorano: e' un aiuto.
 */
export async function ensureHighlightFor(tag: string): Promise<void> {
  const keys = NEEDS[tag];
  if (!keys) return;
  try {
    const doc = await getEngineSettings();
    const changes: Record<string, string> = {};
    for (const k of keys) if (k in doc.values && doc.values[k] !== 'true') changes[k] = 'true';
    if (Object.keys(changes).length > 0) await putEngineSettings(changes);
  } catch {
    // senza motore o senza permesso resta come prima
  }
}
