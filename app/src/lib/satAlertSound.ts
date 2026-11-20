import type { Entry } from '../i18n/core';
import { armAudio, audioContext, scheduleSound, type SoundDef, type Tone } from './soundBank';
import type { AlertKind, AlertSound, SoundChoice } from './satAlerts';

/*
 * I suoni degli avvisi dei satelliti, sintetizzati con WebAudio (nessun file audio) sul contesto unico di soundBank.
 * Campanella: due note che salgono. Radar: tre bip crescenti. Arpeggio: quattro note.
 * Il preavviso suona solo le prime due note e a volume piu' basso; l'AOS suona intero; la LOS ha due note che scendono.
 */

interface Note {
  hz: number;
  at: number;
  dur: number;
  gain?: number;
}

const UP: Record<AlertSound, readonly Note[]> = {
  bell: [{ hz: 659.25, at: 0, dur: 0.6 }, { hz: 987.77, at: 0.2, dur: 0.8 }],
  radar: [{ hz: 880, at: 0, dur: 0.1, gain: 0.55 }, { hz: 1108.73, at: 0.16, dur: 0.1, gain: 0.78 }, { hz: 1318.51, at: 0.32, dur: 0.14, gain: 1 }],
  arpeggio: [{ hz: 523.25, at: 0, dur: 0.24 }, { hz: 659.25, at: 0.11, dur: 0.24 }, { hz: 783.99, at: 0.22, dur: 0.24 }, { hz: 1046.5, at: 0.33, dur: 0.4 }],
};

const DOWN: Record<AlertSound, readonly Note[]> = {
  bell: [{ hz: 987.77, at: 0, dur: 0.6 }, { hz: 659.25, at: 0.2, dur: 0.8 }],
  radar: [{ hz: 1318.51, at: 0, dur: 0.12 }, { hz: 880, at: 0.18, dur: 0.16 }],
  arpeggio: [{ hz: 783.99, at: 0, dur: 0.22 }, { hz: 523.25, at: 0.13, dur: 0.4 }],
};

const NAMES: Record<AlertSound, Entry> = {
  bell: ['Campanella', 'Chime', 'Glöckchen', 'Clochette', 'Campanilla', 'Sineta'],
  radar: ['Radar', 'Radar', 'Radar', 'Radar', 'Radar', 'Radar'],
  arpeggio: ['Arpeggio', 'Arpeggio', 'Arpeggio', 'Arpège', 'Arpegio', 'Arpejo'],
};

/** Il volume del preavviso rispetto all'AOS. */
export const PRE_VOLUME = 0.4;

/** Il suono `sound` per l'evento `kind`, come descrizione di note per `scheduleSound`. */
export function alertSoundDef(sound: AlertSound, kind: AlertKind): SoundDef {
  const notes = kind === 'los' ? DOWN[sound] : kind === 'pre' || kind === 'rotor' ? UP[sound].slice(0, 2) : UP[sound];
  const tones: Tone[] = [];
  for (const n of notes) {
    if (sound === 'bell') {
      tones.push({ hz: n.hz, at: n.at, dur: n.dur, env: 'pluck', wave: 'sine' });
      tones.push({ hz: Math.round(n.hz * 2.76 * 100) / 100, at: n.at, dur: n.dur * 0.5, env: 'pluck', wave: 'sine', gain: 0.25 });
    } else if (sound === 'radar') {
      tones.push({ hz: n.hz, at: n.at, dur: n.dur, env: 'flat', wave: 'sine', gain: n.gain ?? 1 });
    } else {
      tones.push({ hz: n.hz, at: n.at, dur: n.dur, env: 'pluck', wave: 'triangle' });
    }
  }
  return { id: `sat-alert-${sound}-${kind}`, category: 'bells', name: NAMES[sound], wave: 'sine', tones };
}

const volumeOf = (kind: AlertKind, volume: number): number => (Math.max(0, Math.min(100, volume)) / 100) * (kind === 'pre' || kind === 'rotor' ? PRE_VOLUME : 1);

/** Suona l'avviso. Non suona se il suono e' spento, il volume e' zero, manca WebAudio o il contesto e' ancora bloccato (lo sblocca il primo gesto dell'utente). */
export function playAlert(kind: AlertKind, choice: SoundChoice, volume: number, ctx: AudioContext | null = audioContext()): boolean {
  if (choice === 'off' || !ctx) return false;
  const v = volumeOf(kind, volume);
  if (!(v > 0)) return false;
  if (ctx.state !== 'running') {
    void ctx.resume().catch(() => undefined);
    return false;
  }
  return scheduleSound(ctx, alertSoundDef(choice, kind), v) > 0;
}

/** Il pulsante «Prova»: e' un gesto dell'utente, quindi sblocca l'audio e poi suona. */
export function previewAlert(kind: AlertKind, choice: SoundChoice, volume: number, ctx: AudioContext | null = audioContext()): void {
  if (choice === 'off' || !ctx) return;
  const v = volumeOf(kind, volume);
  if (!(v > 0)) return;
  const def = alertSoundDef(choice, kind);
  if (ctx.state !== 'running') void ctx.resume().then(() => scheduleSound(ctx, def, v)).catch(() => undefined);
  else scheduleSound(ctx, def, v);
}

/** Vero se il browser ha WebAudio ma l'audio e' ancora bloccato (politica di autoplay): serve un gesto dell'utente. */
export function audioBlocked(): boolean {
  const ctx = audioContext();
  return !!ctx && ctx.state !== 'running';
}

export { armAudio };
