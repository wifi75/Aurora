import type { AudioDevices, CatSnapshot, OutputLevelState } from '../api/client';
import type { DotState } from './txChain';

/** I modi della radio che non portano una trasmissione digitale (vedi `chainDots`). */
const NOT_FOR_DIGITAL = /^(CW|CWR|FM|WFM|AM|RTTY|RTTYR)$/i;

export type CheckKey = 'cat' | 'mode' | 'ptt' | 'audio' | 'level';

export interface CheckItem {
  key: CheckKey;
  state: DotState;
  /** Chiave del testo da mostrare e valore da inserirvi (modo letto, nome del dispositivo, dB...). */
  text: string;
  value: string;
}

export interface CheckInput {
  cat: CatSnapshot | null;
  audio: AudioDevices | null;
  level: OutputLevelState | null;
}

/**
 * «Prova senza trasmettere»: guarda la catena (CAT, modo radio, PTT, uscita audio, livello) con quello che la radio e Aurora gia' dicono,
 * senza dare il PTT e senza mandare audio (con il VOX l'audio farebbe trasmettere). Ogni voce e' a posto, da guardare o non leggibile.
 */
export function evaluateTxCheck({ cat, audio, level }: CheckInput): CheckItem[] {
  const out: CheckItem[] = [];
  const rig = cat?.status.rig ?? null;
  const backend = cat?.config.backend ?? 'none';
  const connected = rig?.state === 'connected';
  const method = cat?.config.ptt ?? 'vox';

  if (backend === 'none') out.push({ key: 'cat', state: 'unknown', text: 'txcheck.cat.none', value: '' });
  else out.push({ key: 'cat', state: connected ? 'ok' : 'bad', text: connected ? 'txcheck.cat.ok' : 'txcheck.cat.bad', value: rig?.message ?? cat?.status.message ?? '' });

  const mode = rig?.mode?.trim() ?? '';
  out.push({ key: 'mode', state: mode === '' ? 'unknown' : NOT_FOR_DIGITAL.test(mode) ? 'bad' : 'ok', text: mode === '' ? 'txcheck.mode.unknown' : NOT_FOR_DIGITAL.test(mode) ? 'txcheck.mode.bad' : 'txcheck.mode.ok', value: mode });

  if (method === 'vox') out.push({ key: 'ptt', state: 'unknown', text: 'txcheck.ptt.vox', value: '' });
  else if (rig?.ptt_blocked === true) out.push({ key: 'ptt', state: 'bad', text: 'txcheck.ptt.blocked', value: '' });
  else if (!connected) out.push({ key: 'ptt', state: 'unknown', text: 'txcheck.ptt.nocat', value: method });
  else out.push({ key: 'ptt', state: rig?.ptt === true ? 'bad' : 'ok', text: rig?.ptt === true ? 'txcheck.ptt.on' : 'txcheck.ptt.ok', value: method });

  const device = audio?.output_in_use ?? audio?.output_pending ?? null;
  if (audio?.output_error) out.push({ key: 'audio', state: 'bad', text: 'txcheck.audio.error', value: audio.output_error });
  else if (device === null || device === '') out.push({ key: 'audio', state: 'bad', text: 'txcheck.audio.none', value: '' });
  else out.push({ key: 'audio', state: 'ok', text: 'txcheck.audio.ok', value: device });

  if (level) {
    // l'attenuazione e' 0 dB a livello pieno e sale fino al massimo (cursore Pwr in fondo): lì il segnale e' praticamente muto
    const muted = level.attenuation_db >= level.max_db - 1;
    out.push({ key: 'level', state: muted ? 'bad' : 'ok', text: muted ? 'txcheck.level.bad' : 'txcheck.level.ok', value: `${level.attenuation_db} dB` });
  } else out.push({ key: 'level', state: 'unknown', text: 'txcheck.level.unknown', value: '' });
  return out;
}
