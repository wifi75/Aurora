import { pick, type Entry, type Lang } from '../i18n/core';

/*
 * Banco dei suoni: tutti sintetizzati con WebAudio, descritti con dati (nessun file audio, nessuna licenza).
 * Ogni suono e' un elenco di note (frequenza, inizio, durata, forma d'onda, inviluppo, glissando, vibrato) con un filtro facoltativo.
 * La normalizzazione del volume e' automatica: peso per forma d'onda e divisione per la radice delle note che suonano insieme.
 */

export type Wave = 'sine' | 'square' | 'triangle' | 'sawtooth';
/** flat = attacco, tenuta, rilascio; fade = attacco e calo lineare; pluck = attacco rapido e calo esponenziale (corde, campane); swell = attacco lento (fiati). */
export type Env = 'flat' | 'fade' | 'pluck' | 'swell';
export type SoundCategory = 'bells' | 'beeps' | 'melodies' | 'signals' | 'radio' | 'nature';

export interface Tone {
  /** Frequenza in Hz (all'inizio, se c'e' il glissando). */
  hz: number;
  /** Inizio in secondi dall'avvio del suono. */
  at: number;
  /** Durata in secondi. */
  dur: number;
  /** Frequenza finale di un glissando. */
  to?: number;
  /** Peso della nota (0-1, di base 1). */
  gain?: number;
  wave?: Wave;
  env?: Env;
  /** Vibrato: velocita' in Hz e profondita' in Hz. */
  vibrato?: { rate: number; depth: number };
}

export interface SoundDef {
  id: string;
  category: SoundCategory;
  /** [it, en, de, fr, es, pt] */
  name: Entry;
  wave: Wave;
  /** Correzione fine del livello percepito (di base 1). */
  level?: number;
  filter?: { type: 'lowpass' | 'highpass' | 'bandpass'; hz: number; q?: number };
  tones: Tone[];
}

export const SOUND_CATEGORIES: readonly { id: SoundCategory; name: Entry }[] = [
  { id: 'bells', name: ['Campane e campanelli', 'Bells and chimes', 'Glocken und Klingeln', 'Cloches et carillons', 'Campanas y campanillas', 'Sinos e campainhas'] },
  { id: 'beeps', name: ['Bip e toni', 'Beeps and tones', 'Pieptöne und Töne', 'Bips et tonalités', 'Pitidos y tonos', 'Bipes e tons'] },
  { id: 'melodies', name: ['Scale e melodie', 'Scales and melodies', 'Tonleitern und Melodien', 'Gammes et mélodies', 'Escalas y melodías', 'Escalas e melodias'] },
  { id: 'signals', name: ['Sirene e segnali', 'Sirens and signals', 'Sirenen und Signale', 'Sirènes et signaux', 'Sirenas y señales', 'Sirenes e sinais'] },
  { id: 'radio', name: ['Radio e telegrafo', 'Radio and telegraph', 'Funk und Telegrafie', 'Radio et télégraphe', 'Radio y telégrafo', 'Rádio e telégrafo'] },
  { id: 'nature', name: ['Gocce ed effetti', 'Drops and effects', 'Tropfen und Effekte', 'Gouttes et effets', 'Gotas y efectos', 'Gotas e efeitos'] },
];

const m = (midi: number): number => Math.round(440 * 2 ** ((midi - 69) / 12) * 100) / 100;

const seq = (hzs: readonly number[], step: number, dur: number, extra: Partial<Tone> = {}): Tone[] => hzs.map((hz, i) => ({ hz, at: Math.round(i * step * 1000) / 1000, dur, ...extra }));

type Partials = readonly (readonly [number, number])[];
const HARMONIC: Partials = [[1, 1], [2, 0.35], [3, 0.12]];
const METAL: Partials = [[1, 1], [2.76, 0.4], [5.4, 0.15]];
const GONG: Partials = [[1, 1], [2.4, 0.5], [3.7, 0.3], [5.1, 0.15]];

/** Una campana: la fondamentale e le armoniche, piu' corte quanto piu' acute. */
function bell(hz: number, at: number, dur: number, parts: Partials = HARMONIC, gain = 1): Tone[] {
  return parts.map(([ratio, g], i) => ({ hz: Math.round(hz * ratio * 100) / 100, at, dur: Math.max(0.05, dur / (1 + i * 0.7)), env: 'pluck' as const, gain: g * gain }));
}

const MORSE: Record<string, string> = { D: '-..', X: '-..-', '7': '--...', '3': '...--', C: '-.-.', Q: '--.-', A: '.-', R: '.-.' };

/** Telegrafo: il testo in Morse con l'unita' di tempo data (punto = 1, linea = 3, spazio tra segni = 1, tra lettere = 3). */
function morse(text: string, unit: number, hz: number): Tone[] {
  const tones: Tone[] = [];
  let t = 0;
  [...text].forEach((ch, li) => {
    const code = MORSE[ch] ?? '';
    [...code].forEach((e, j) => {
      const dur = (e === '.' ? 1 : 3) * unit;
      tones.push({ hz, at: Math.round(t * 1000) / 1000, dur: Math.round(dur * 1000) / 1000 });
      t += dur;
      if (j < code.length - 1) t += unit;
    });
    if (li < text.length - 1) t += 3 * unit;
  });
  return tones;
}

/** Due frequenze che si alternano in fretta (telefono, modem). */
const alternate = (a: number, b: number, count: number, step: number, from = 0, extra: Partial<Tone> = {}): Tone[] =>
  Array.from({ length: count }, (_, i) => ({ hz: i % 2 === 0 ? a : b, at: Math.round((from + i * step) * 1000) / 1000, dur: step, ...extra }));

const C_MAJOR = [m(72), m(74), m(76), m(77), m(79), m(81), m(83), m(84)];
const PENTA = [m(72), m(74), m(76), m(79), m(81), m(84), m(86), m(88), m(91), m(93)];
const FT8 = [3, 1, 4, 0, 6, 2, 7, 5].map((k) => 800 + k * 90);
const FT4 = [2, 0, 3, 1].map((k) => 900 + k * 220);

export const SOUNDS: readonly SoundDef[] = [
  // ---------------------------------------------------------------- Campane e campanelli
  { id: 'chime', category: 'bells', name: ['Campanelle', 'Chime', 'Glockenspiel', 'Carillon', 'Campanillas', 'Sinos'], wave: 'sine', tones: seq([988, 1319, 1568], 0.16, 0.22, { env: 'fade' }) },
  { id: 'bell', category: 'bells', name: ['Campana', 'Bell', 'Glocke', 'Cloche', 'Campana', 'Sino'], wave: 'sine', tones: seq([880, 880, 660], 0.3, 0.5, { env: 'fade' }) },
  { id: 'dingdong', category: 'bells', name: ['Ding-dong', 'Ding-dong', 'Ding-Dong', 'Ding-dong', 'Ding-dong', 'Ding-dong'], wave: 'sine', tones: [...bell(m(76), 0, 0.7), ...bell(m(72), 0.45, 0.9)] },
  { id: 'gong', category: 'bells', name: ['Gong', 'Gong', 'Gong', 'Gong', 'Gong', 'Gongo'], wave: 'sine', tones: bell(147, 0, 1.4, GONG) },
  { id: 'carillon', category: 'bells', name: ['Carillon a scendere', 'Descending carillon', 'Absteigendes Glockenspiel', 'Carillon descendant', 'Carillón descendente', 'Carrilhão descendente'], wave: 'sine', tones: seq([1568, 1319, 1047, 784], 0.22, 0.6, { env: 'pluck' }) },
  { id: 'doorbell', category: 'bells', name: ['Campanello', 'Little bell', 'Klingel', 'Sonnette', 'Timbre', 'Campainha'], wave: 'triangle', tones: alternate(2093, 2349, 11, 0.045, 0, { env: 'flat' }) },
  { id: 'shipbell', category: 'bells', name: ['Campana di bordo', 'Ship’s bell', 'Schiffsglocke', 'Cloche de bord', 'Campana de a bordo', 'Sino de bordo'], wave: 'sine', tones: [0, 0.2, 0.7, 0.9].flatMap((at) => bell(m(79), at, 0.5, METAL, 0.8)) },

  // ---------------------------------------------------------------- Bip e toni
  { id: 'pip', category: 'beeps', name: ['Pip pip', 'Pip pip', 'Piep piep', 'Bip bip', 'Pip pip', 'Pip pip'], wave: 'sine', tones: seq([1800, 1800], 0.12, 0.09, { env: 'fade' }) },
  { id: 'beep2', category: 'beeps', name: ['Doppio bip', 'Double beep', 'Doppelpiep', 'Double bip', 'Doble pitido', 'Duplo bipe'], wave: 'triangle', tones: seq([880, 1320], 0.14, 0.1) },
  { id: 'beep3', category: 'beeps', name: ['Triplo bip', 'Triple beep', 'Dreifachpiep', 'Triple bip', 'Triple pitido', 'Triplo bipe'], wave: 'triangle', tones: seq([1200, 1200, 1200], 0.12, 0.07) },
  { id: 'beeplong', category: 'beeps', name: ['Bip lungo', 'Long beep', 'Langer Piep', 'Bip long', 'Pitido largo', 'Bipe longo'], wave: 'sine', tones: [{ hz: 1000, at: 0, dur: 0.45 }] },
  { id: 'glideup', category: 'beeps', name: ['Bip ascendente', 'Rising beep', 'Steigender Piep', 'Bip montant', 'Pitido ascendente', 'Bipe ascendente'], wave: 'sine', tones: [{ hz: 500, to: 1500, at: 0, dur: 0.3 }] },
  { id: 'glidedown', category: 'beeps', name: ['Bip discendente', 'Falling beep', 'Fallender Piep', 'Bip descendant', 'Pitido descendente', 'Bipe descendente'], wave: 'sine', tones: [{ hz: 1500, to: 500, at: 0, dur: 0.3 }] },
  { id: 'click2', category: 'beeps', name: ['Clic doppio', 'Double click', 'Doppelklick', 'Double clic', 'Doble clic', 'Duplo clique'], wave: 'square', level: 0.8, filter: { type: 'bandpass', hz: 2200, q: 1 }, tones: seq([2200, 2200], 0.18, 0.04) },
  { id: 'tictoc', category: 'beeps', name: ['Tic-tac', 'Tick-tock', 'Tick-Tack', 'Tic-tac', 'Tic-tac', 'Tique-taque'], wave: 'square', level: 0.8, filter: { type: 'bandpass', hz: 1500, q: 2 }, tones: [{ hz: 1800, at: 0, dur: 0.035 }, { hz: 1200, at: 0.25, dur: 0.045 }] },
  { id: 'single', category: 'beeps', name: ['Nota singola', 'Single note', 'Einzelton', 'Note unique', 'Nota única', 'Nota única'], wave: 'sine', tones: [{ hz: 880, at: 0, dur: 0.2, env: 'fade' }] },
  { id: 'trill4', category: 'beeps', name: ['Trillo insistente', 'Insistent trill', 'Hartnäckiger Triller', 'Trille insistant', 'Trino insistente', 'Trinado insistente'], wave: 'sine', tones: seq([1047, 1319, 1047, 1319], 0.14, 0.12, { env: 'fade' }) },

  // ---------------------------------------------------------------- Scale e melodie
  { id: 'rise3', category: 'melodies', name: ['Tre note in salita', 'Three rising notes', 'Drei steigende Töne', 'Trois notes montantes', 'Tres notas ascendentes', 'Três notas a subir'], wave: 'sine', tones: seq([660, 880, 1320], 0.14, 0.12, { env: 'fade' }) },
  { id: 'rise4', category: 'melodies', name: ['Quattro note in salita', 'Four rising notes', 'Vier steigende Töne', 'Quatre notes montantes', 'Cuatro notas ascendentes', 'Quatro notas a subir'], wave: 'sine', tones: seq([784, 1175, 1568, 1976], 0.14, 0.12, { env: 'fade' }) },
  { id: 'scaleup', category: 'melodies', name: ['Scala ascendente', 'Rising scale', 'Aufsteigende Tonleiter', 'Gamme montante', 'Escala ascendente', 'Escala ascendente'], wave: 'triangle', tones: seq(C_MAJOR, 0.08, 0.11) },
  { id: 'scaledown', category: 'melodies', name: ['Scala discendente', 'Falling scale', 'Absteigende Tonleiter', 'Gamme descendante', 'Escala descendente', 'Escala descendente'], wave: 'triangle', tones: seq([...C_MAJOR].reverse(), 0.08, 0.11) },
  { id: 'arp', category: 'melodies', name: ['Arpa', 'Harp', 'Harfe', 'Harpe', 'Arpa', 'Harpa'], wave: 'sine', tones: seq([m(72), m(76), m(79), m(84), m(88), m(91)], 0.09, 0.5, { env: 'pluck' }) },
  { id: 'gliss', category: 'melodies', name: ['Glissando d’arpa', 'Harp glissando', 'Harfenglissando', 'Glissando de harpe', 'Glissando de arpa', 'Glissando de harpa'], wave: 'sine', tones: seq(PENTA, 0.045, 0.35, { env: 'pluck' }) },
  { id: 'xylo', category: 'melodies', name: ['Xilofono', 'Xylophone', 'Xylophon', 'Xylophone', 'Xilófono', 'Xilofone'], wave: 'triangle', tones: seq([m(84), m(88), m(91), m(88), m(84)], 0.11, 0.18, { env: 'pluck' }).flatMap((t) => [t, { ...t, hz: Math.round(t.hz * 3.9 * 100) / 100, dur: t.dur * 0.4, gain: 0.25 }]) },
  { id: 'pizz', category: 'melodies', name: ['Pizzicato', 'Pizzicato', 'Pizzicato', 'Pizzicato', 'Pizzicato', 'Pizzicato'], wave: 'triangle', tones: seq([m(67), m(72), m(76), m(79)], 0.1, 0.16, { env: 'pluck' }) },
  { id: 'fanfare', category: 'melodies', name: ['Fanfara', 'Fanfare', 'Fanfare', 'Fanfare', 'Fanfarria', 'Fanfarra'], wave: 'sawtooth', filter: { type: 'lowpass', hz: 2400, q: 0.7 }, tones: [{ hz: m(67), at: 0, dur: 0.14 }, { hz: m(72), at: 0.16, dur: 0.14 }, { hz: m(76), at: 0.32, dur: 0.14 }, { hz: m(79), at: 0.48, dur: 0.5, env: 'swell', vibrato: { rate: 5.5, depth: 6 } }] },
  { id: 'trumpet', category: 'melodies', name: ['Squillo di tromba', 'Trumpet call', 'Trompetenstoß', 'Sonnerie de trompette', 'Toque de trompeta', 'Toque de trompete'], wave: 'sawtooth', filter: { type: 'lowpass', hz: 1800, q: 0.7 }, tones: [{ hz: m(70), at: 0, dur: 0.7, env: 'swell', vibrato: { rate: 5, depth: 5 } }] },
  { id: 'ft8', category: 'melodies', name: ['Otto toni a gradini (FT8)', 'Eight stepped tones (FT8)', 'Acht Stufentöne (FT8)', 'Huit tons étagés (FT8)', 'Ocho tonos escalonados (FT8)', 'Oito tons em degraus (FT8)'], wave: 'sine', tones: seq(FT8, 0.12, 0.12) },
  { id: 'ft4', category: 'melodies', name: ['Quattro toni (FT4)', 'Four tones (FT4)', 'Vier Töne (FT4)', 'Quatre tons (FT4)', 'Cuatro tonos (FT4)', 'Quatro tons (FT4)'], wave: 'sine', tones: seq(FT4, 0.1, 0.1) },
  { id: 'chord', category: 'melodies', name: ['Accordo maggiore', 'Major chord', 'Durakkord', 'Accord majeur', 'Acorde mayor', 'Acorde maior'], wave: 'triangle', tones: [m(72), m(76), m(79)].map((hz) => ({ hz, at: 0, dur: 0.8, env: 'swell' as const })) },
  { id: 'flute', category: 'melodies', name: ['Flauto', 'Flute', 'Flöte', 'Flûte', 'Flauta', 'Flauta'], wave: 'sine', tones: [{ hz: m(79), at: 0, dur: 0.3, env: 'swell', vibrato: { rate: 5.5, depth: 7 } }, { hz: m(84), at: 0.3, dur: 0.5, env: 'swell', vibrato: { rate: 5.5, depth: 9 } }] },

  // ---------------------------------------------------------------- Sirene e segnali
  { id: 'siren', category: 'signals', name: ['Sirena breve', 'Short siren', 'Kurze Sirene', 'Sirène brève', 'Sirena corta', 'Sirene curta'], wave: 'triangle', tones: [{ hz: 600, to: 1100, at: 0, dur: 0.5, env: 'swell' }, { hz: 1100, to: 600, at: 0.5, dur: 0.5, env: 'swell' }] },
  { id: 'shiphorn', category: 'signals', name: ['Chiamata di nave', 'Ship’s horn', 'Schiffshorn', 'Sirène de navire', 'Sirena de barco', 'Buzina de navio'], wave: 'sawtooth', filter: { type: 'lowpass', hz: 700, q: 0.8 }, tones: [{ hz: 117, at: 0, dur: 1.2, env: 'swell' }, { hz: 175, at: 0, dur: 1.2, env: 'swell' }] },
  { id: 'twotone', category: 'signals', name: ['Sirena a due toni', 'Two-tone siren', 'Zweiklang-Sirene', 'Sirène à deux tons', 'Sirena de dos tonos', 'Sirene de dois tons'], wave: 'sawtooth', filter: { type: 'lowpass', hz: 1800, q: 0.7 }, tones: alternate(587, 440, 4, 0.3) },
  { id: 'sonar', category: 'signals', name: ['Sonar', 'Sonar', 'Sonar', 'Sonar', 'Sonar', 'Sonar'], wave: 'sine', tones: [{ hz: 1100, at: 0, dur: 1, env: 'pluck' }, { hz: 1100, at: 0.6, dur: 0.8, env: 'pluck', gain: 0.35 }] },
  { id: 'radar', category: 'signals', name: ['Radar', 'Radar', 'Radar', 'Radar', 'Radar', 'Radar'], wave: 'sine', tones: [{ hz: 900, to: 2600, at: 0, dur: 0.12 }, { hz: 2600, at: 0.12, dur: 0.3, env: 'pluck' }, { hz: 2600, at: 0.6, dur: 0.3, env: 'pluck', gain: 0.3 }] },
  { id: 'alarm', category: 'signals', name: ['Allarme rapido', 'Fast alarm', 'Schneller Alarm', 'Alarme rapide', 'Alarma rápida', 'Alarme rápido'], wave: 'square', level: 0.8, tones: seq([1760, 1760, 1760, 1760], 0.12, 0.07) },
  { id: 'warble', category: 'signals', name: ['Trillo', 'Warble', 'Tremolo', 'Gazouillis', 'Gorjeo', 'Gorjeio'], wave: 'sine', tones: [{ hz: 1500, at: 0, dur: 0.6, env: 'swell', vibrato: { rate: 14, depth: 250 } }] },
  { id: 'cuckoo', category: 'signals', name: ['Cucù', 'Cuckoo', 'Kuckuck', 'Coucou', 'Cucú', 'Cuco'], wave: 'triangle', tones: [{ hz: 784, at: 0, dur: 0.22 }, { hz: 659, at: 0.26, dur: 0.3 }, { hz: 784, at: 0.7, dur: 0.22 }, { hz: 659, at: 0.96, dur: 0.3 }] },
  { id: 'phone', category: 'signals', name: ['Telefono antico', 'Old telephone', 'Altes Telefon', 'Vieux téléphone', 'Teléfono antiguo', 'Telefone antigo'], wave: 'triangle', tones: [...alternate(1100, 1400, 12, 0.03), ...alternate(1100, 1400, 12, 0.03, 0.55)] },
  { id: 'horn', category: 'signals', name: ['Tromba', 'Horn', 'Horn', 'Klaxon', 'Bocina', 'Buzina'], wave: 'sine', tones: seq([330, 330, 392], 0.25, 0.22, { env: 'fade' }) },

  // ---------------------------------------------------------------- Radio e telegrafo
  { id: 'cwdx', category: 'radio', name: ['Telegrafo «DX»', 'Telegraph «DX»', 'Telegrafie «DX»', 'Télégraphe «DX»', 'Telégrafo «DX»', 'Telégrafo «DX»'], wave: 'sine', tones: morse('DX', 0.05, 700) },
  { id: 'cw73', category: 'radio', name: ['Telegrafo «73»', 'Telegraph «73»', 'Telegrafie «73»', 'Télégraphe «73»', 'Telégrafo «73»', 'Telégrafo «73»'], wave: 'sine', tones: morse('73', 0.04, 800) },
  { id: 'cwcq', category: 'radio', name: ['Telegrafo «CQ»', 'Telegraph «CQ»', 'Telegrafie «CQ»', 'Télégraphe «CQ»', 'Telégrafo «CQ»', 'Telégrafo «CQ»'], wave: 'sine', tones: morse('CQ', 0.045, 600) },
  { id: 'cwar', category: 'radio', name: ['Telegrafo «AR» (fine messaggio)', 'Telegraph «AR» (end of message)', 'Telegrafie «AR» (Ende)', 'Télégraphe «AR» (fin de message)', 'Telégrafo «AR» (fin de mensaje)', 'Telégrafo «AR» (fim de mensagem)'], wave: 'sine', tones: morse('AR', 0.06, 750) },
  { id: 'packet', category: 'radio', name: ['Packet radio', 'Packet radio', 'Packet-Radio', 'Packet radio', 'Packet radio', 'Packet radio'], wave: 'sine', tones: alternate(1200, 2200, 14, 0.03) },
  { id: 'timesig', category: 'radio', name: ['Segnale orario', 'Time signal', 'Zeitzeichen', 'Signal horaire', 'Señal horaria', 'Sinal horário'], wave: 'sine', tones: [0, 0.2, 0.4].map((at) => ({ hz: 1000, at, dur: 0.1 })).concat([{ hz: 1000, at: 0.6, dur: 0.5 }]) },
  { id: 'roger', category: 'radio', name: ['Roger bip', 'Roger beep', 'Roger-Piep', 'Bip de fin', 'Pitido roger', 'Bipe roger'], wave: 'triangle', tones: [{ hz: 1400, at: 0, dur: 0.08 }, { hz: 1900, at: 0.12, dur: 0.14 }] },

  // ---------------------------------------------------------------- Gocce ed effetti
  { id: 'drops', category: 'nature', name: ['Gocce', 'Drops', 'Tropfen', 'Gouttes', 'Gotas', 'Gotas'], wave: 'sine', tones: [{ hz: 1200, to: 400, at: 0, dur: 0.12 }, { hz: 900, to: 350, at: 0.25, dur: 0.12, gain: 0.8 }, { hz: 1050, to: 380, at: 0.5, dur: 0.12, gain: 0.9 }] },
  { id: 'bubbles', category: 'nature', name: ['Bolle', 'Bubbles', 'Blasen', 'Bulles', 'Burbujas', 'Bolhas'], wave: 'sine', tones: [{ hz: 300, to: 900, at: 0, dur: 0.1 }, { hz: 400, to: 1100, at: 0.14, dur: 0.1 }, { hz: 250, to: 800, at: 0.3, dur: 0.12 }] },
  { id: 'spaceship', category: 'nature', name: ['Astronave', 'Spaceship', 'Raumschiff', 'Vaisseau spatial', 'Nave espacial', 'Nave espacial'], wave: 'sine', tones: [{ hz: 400, to: 1600, at: 0, dur: 0.8, env: 'swell', vibrato: { rate: 8, depth: 40 } }] },
  { id: 'chirp', category: 'nature', name: ['Cinguettio', 'Chirp', 'Zwitschern', 'Gazouillis d’oiseau', 'Trino de pájaro', 'Chilrear'], wave: 'sine', tones: [{ hz: 1800, to: 2600, at: 0, dur: 0.08 }, { hz: 2000, to: 2800, at: 0.12, dur: 0.08 }, { hz: 1600, to: 3000, at: 0.24, dur: 0.1 }] },
  { id: 'tinkle', category: 'nature', name: ['Tintinnio', 'Tinkle', 'Klimpern', 'Tintement', 'Tintineo', 'Tilintar'], wave: 'sine', tones: seq([3136, 2637, 3520, 2794, 3951], 0.07, 0.25, { env: 'pluck' }) },
];

const BY_ID = new Map(SOUNDS.map((s) => [s.id, s]));

export function getSound(id: string): SoundDef | undefined {
  return BY_ID.get(id);
}

export function hasSound(id: string | null | undefined): id is string {
  return typeof id === 'string' && BY_ID.has(id);
}

export function soundName(id: string, lang: Lang): string {
  const s = BY_ID.get(id);
  return s ? pick(s.name, lang) : id;
}

export function categoryName(id: SoundCategory, lang: Lang): string {
  const c = SOUND_CATEGORIES.find((x) => x.id === id);
  return c ? pick(c.name, lang) : id;
}

/** Durata totale di un suono, in secondi. */
export function soundDuration(def: SoundDef): number {
  return def.tones.reduce((end, t) => Math.max(end, t.at + t.dur), 0);
}

/** Quante note suonano insieme al massimo: serve a non far salire il volume con gli accordi e le campane. */
export function polyphony(def: SoundDef): number {
  let max = 1;
  for (const a of def.tones) {
    const n = def.tones.filter((b) => b.at <= a.at + 1e-9 && b.at + b.dur > a.at + 1e-6).length;
    if (n > max) max = n;
  }
  return max;
}

const WAVE_WEIGHT: Record<Wave, number> = { sine: 1, triangle: 0.85, square: 0.4, sawtooth: 0.45 };
/** Il volume 1 del suono, prima del volume scelto: meta' della scala come gli avvisi di prima. */
const MASTER = 0.5;

/** Il peso di una nota: forma d'onda, correzione del suono, note simultanee e peso proprio. */
export function toneGain(def: SoundDef, tone: Tone, volume: number): number {
  const v = Math.min(1, Math.max(0, volume));
  const wave = tone.wave ?? def.wave;
  return v * MASTER * WAVE_WEIGHT[wave] * (def.level ?? 1) * (tone.gain ?? 1) / Math.sqrt(polyphony(def));
}

export interface EnvPoint {
  /** Secondi dall'inizio della nota. */
  t: number;
  /** Da 0 a 1. */
  g: number;
  /** Rampa esponenziale fino a questo punto (altrimenti lineare). */
  exp?: boolean;
}

/** L'inviluppo di una nota: parte sempre da zero e finisce sempre a zero, cosi' non ci sono scatti. */
export function envelope(tone: Tone): EnvPoint[] {
  const { dur } = tone;
  switch (tone.env ?? 'flat') {
    case 'fade': {
      const a = Math.min(0.01, dur * 0.3);
      return [{ t: 0, g: 0 }, { t: a, g: 1 }, { t: dur, g: 0 }];
    }
    case 'pluck': {
      const a = Math.min(0.006, dur * 0.2);
      const r = Math.min(0.012, dur * 0.2);
      return [{ t: 0, g: 0 }, { t: a, g: 1 }, { t: dur - r, g: 0.0008, exp: true }, { t: dur, g: 0 }];
    }
    case 'swell': {
      const a = Math.min(0.1, dur * 0.35);
      const r = Math.min(0.12, dur * 0.4);
      return [{ t: 0, g: 0 }, { t: a, g: 1 }, { t: dur - r, g: 0.85 }, { t: dur, g: 0 }];
    }
    default: {
      const a = Math.min(0.012, dur * 0.3);
      const r = Math.min(0.04, dur * 0.4);
      return [{ t: 0, g: 0 }, { t: a, g: 1 }, { t: dur - r, g: 1 }, { t: dur, g: 0 }];
    }
  }
}

interface Playing {
  stop(): void;
}
const playing = new Set<Playing>();

/** Programma un suono su un contesto audio; restituisce quante note ha messo in coda. */
export function scheduleSound(ctx: AudioContext, def: SoundDef, volume: number, startAt: number = ctx.currentTime + 0.02): number {
  let count = 0;
  for (const tone of def.tones) {
    const peak = toneGain(def, tone, volume);
    if (peak <= 0) continue;
    const at = startAt + tone.at;
    const osc = ctx.createOscillator();
    const amp = ctx.createGain();
    osc.type = tone.wave ?? def.wave;
    osc.frequency.setValueAtTime(tone.hz, at);
    if (tone.to !== undefined) osc.frequency.exponentialRampToValueAtTime(tone.to, at + tone.dur);
    const extra: AudioNode[] = [];
    if (tone.vibrato) {
      const lfo = ctx.createOscillator();
      const depth = ctx.createGain();
      lfo.frequency.value = tone.vibrato.rate;
      depth.gain.value = tone.vibrato.depth;
      lfo.connect(depth).connect(osc.frequency);
      lfo.start(at);
      lfo.stop(at + tone.dur + 0.05);
      extra.push(lfo, depth);
    }
    for (const p of envelope(tone)) {
      const value = Math.max(p.g * peak, 0);
      if (p.t === 0) amp.gain.setValueAtTime(value, at);
      else if (p.exp) amp.gain.exponentialRampToValueAtTime(Math.max(value, 0.0001), at + p.t);
      else amp.gain.linearRampToValueAtTime(value, at + p.t);
    }
    let out: AudioNode = osc.connect(amp);
    if (def.filter) {
      const f = ctx.createBiquadFilter();
      f.type = def.filter.type;
      f.frequency.value = def.filter.hz;
      f.Q.value = def.filter.q ?? 0.7;
      out = out.connect(f);
    }
    out.connect(ctx.destination);
    osc.start(at);
    osc.stop(at + tone.dur + 0.03);
    const handle: Playing = {
      stop() {
        try {
          osc.stop();
        } catch {
          // gia' fermo
        }
        try {
          amp.disconnect();
        } catch {
          // gia' staccato
        }
      },
    };
    playing.add(handle);
    osc.onended = () => {
      playing.delete(handle);
      for (const n of extra) {
        try {
          n.disconnect();
        } catch {
          // gia' staccato
        }
      }
    };
    count++;
  }
  return count;
}

let shared: AudioContext | null = null;
let armed = false;

function resumeOnGesture(): void {
  if (typeof window === 'undefined' || armed) return;
  armed = true;
  const resume = () => {
    if (shared && shared.state !== 'running') void shared.resume().catch(() => undefined);
  };
  for (const ev of ['pointerdown', 'keydown', 'touchstart'] as const) window.addEventListener(ev, resume, { passive: true });
}

/** L'unico contesto audio dell'applicazione, creato alla prima richiesta; `null` se il browser non ha WebAudio. */
export function audioContext(): AudioContext | null {
  if (shared) return shared;
  const Ctor: typeof AudioContext | undefined =
    typeof window === 'undefined' ? undefined : (window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext);
  if (!Ctor) return null;
  try {
    shared = new Ctor();
  } catch {
    return null;
  }
  resumeOnGesture();
  return shared;
}

/** Prepara l'audio (da chiamare in un gesto dell'utente): crea il contesto e lo riprende se il browser lo ha sospeso. */
export function armAudio(): void {
  const ctx = audioContext();
  if (ctx && ctx.state !== 'running') void ctx.resume().catch(() => undefined);
}

/**
 * Suona il suono `id` al volume dato (0-1). Non suona se il suono non esiste, il volume e' zero, il browser non ha WebAudio
 * o il contesto e' ancora sospeso (le note non restano in coda per partire tutte insieme al primo clic).
 * `ctx` serve alle prove.
 */
export function playSound(id: string, volume: number, ctx: AudioContext | null = audioContext()): boolean {
  const def = BY_ID.get(id);
  if (!def || !ctx || !(volume > 0)) return false;
  if (ctx.state === 'suspended') {
    void ctx.resume().catch(() => undefined);
    return false;
  }
  return scheduleSound(ctx, def, volume) > 0;
}

/** Come `playSound`, ma per un gesto dell'utente (il pulsante «Prova»): se il contesto e' sospeso lo riprende e poi suona. */
export function previewSound(id: string, volume: number, ctx: AudioContext | null = audioContext()): void {
  const def = BY_ID.get(id);
  if (!def || !ctx || !(volume > 0)) return;
  stopAll();
  if (ctx.state === 'suspended') void ctx.resume().then(() => scheduleSound(ctx, def, volume)).catch(() => undefined);
  else scheduleSound(ctx, def, volume);
}

/** Ferma tutti i suoni in corso. */
export function stopAll(): void {
  for (const p of [...playing]) p.stop();
  playing.clear();
}
