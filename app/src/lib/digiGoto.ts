import { getDigi, putDigi, type DigiSettings } from '../api/client';

/** Passa a un modo testuale (PSK31, PSK63, RTTY): lo sceglie e apre la pagina dei modi testuali. */
export async function gotoTextMode(mode: DigiSettings['mode']): Promise<void> {
  try {
    const snap = await getDigi();
    if (snap.settings.mode !== mode) await putDigi({ ...snap.settings, mode });
  } catch {
    // la pagina si apre lo stesso: il modo si sceglie da Modo
  }
  window.location.hash = '#/testo';
}
