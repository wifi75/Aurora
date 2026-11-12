import { signalBandwidthHz, signalStartOffsetHz } from './markers';

/**
 * Dove sta il segnale trasmesso nell'audio: molte radio in USB-D/DATA passano bene fra circa 200 e 2700 Hz e fuori tagliano la potenza
 * (il filtro di trasmissione). Aurora non lo impedisce, come WSJT-X, ma lo dice: con la frequenza Tx troppo alta la radio esce con pochi watt
 * e sembra un guasto di Aurora.
 */
export const TX_AUDIO_LOW_HZ = 200;
export const TX_AUDIO_HIGH_HZ = 2700;

/** `high` se il segnale finisce sopra 2700 Hz, `low` se comincia sotto 200 Hz, altrimenti `null`. Tiene conto della larghezza del modo. */
export function txAudioWarning(txHz: number | null | undefined, mode: string | undefined): 'high' | 'low' | null {
  if (txHz == null || !Number.isFinite(txHz) || txHz <= 0) return null;
  const start = txHz + signalStartOffsetHz(mode);
  const end = start + signalBandwidthHz(mode);
  if (end > TX_AUDIO_HIGH_HZ) return 'high';
  if (start < TX_AUDIO_LOW_HZ) return 'low';
  return null;
}
