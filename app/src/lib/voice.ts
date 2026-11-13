import { useEffect, useState } from 'react';
import { useLang } from '../i18n';
import type { SoundEvent } from './soundAssign';
import { playSound, previewSound } from './soundBank';
import { browserStorage } from './theme';

/**
 * Suoni che non sono generati da Aurora: la voce (frasi registrate con la voce Ava, una per funzione e lingua, e il nominativo
 * con l'alfabeto fonetico) e i file dell'utente (WAV/MP3, tenuti nel database del browser).
 */
export const VOICE_ID = 'voice';
export const FILE_PREFIX = 'file:';

// ---- lingua della voce: quella di Aurora oppure sempre l'inglese ----

const LANG_KEY = 'aurora-voice-lang';
const LANG_EVENT = 'aurora-voice-lang';
export type VoiceLang = 'ui' | 'en';

export function readVoiceLang(): VoiceLang {
  try {
    return browserStorage()?.getItem(LANG_KEY) === 'en' ? 'en' : 'ui';
  } catch {
    return 'ui';
  }
}

export function setVoiceLang(v: VoiceLang): void {
  try {
    browserStorage()?.setItem(LANG_KEY, v);
  } catch {
    /* vale solo per questa volta */
  }
  window.dispatchEvent(new Event(LANG_EVENT));
}

export function useVoiceLang(): VoiceLang {
  const [v, setV] = useState<VoiceLang>(readVoiceLang);
  useEffect(() => {
    const update = () => setV(readVoiceLang());
    window.addEventListener(LANG_EVENT, update);
    window.addEventListener('storage', update);
    return () => {
      window.removeEventListener(LANG_EVENT, update);
      window.removeEventListener('storage', update);
    };
  }, []);
  return v;
}

// ---- riproduzione in fila di piu' pezzi ----

let queue: HTMLAudioElement[] = [];

function stopQueue(): void {
  for (const a of queue) a.pause();
  queue = [];
}

function playChain(urls: string[], volume: number): void {
  stopQueue();
  const chain = urls.map((u) => {
    const a = new Audio(u);
    a.volume = Math.max(0, Math.min(1, volume));
    a.preload = 'auto';
    return a;
  });
  queue = chain;
  chain.forEach((a, i) => {
    a.onended = () => {
      const next = chain[i + 1];
      if (next && queue === chain) void next.play().catch(() => undefined);
    };
  });
  void chain[0]?.play().catch(() => undefined);
}

const base = (): string => import.meta.env.BASE_URL ?? '/';

/** I pezzi da dire: la frase della funzione e il nominativo lettera per lettera (alfabeto fonetico, uguale in tutte le lingue). */
export function voiceParts(event: SoundEvent, call: string | undefined, lang: string): string[] {
  const parts = [`${base()}voice/${lang}/${event}.ogg`];
  for (const ch of (call ?? '').toUpperCase()) {
    if (/[A-Z0-9]/.test(ch)) parts.push(`${base()}voice/nato/${ch}.ogg`);
    else if (ch === '/') parts.push(`${base()}voice/nato/slash.ogg`);
  }
  return parts;
}

/** Le lingue che hanno le frasi registrate (cartelle in `public/voice/`): le altre (olandese) parlano inglese. */
const VOICE_LANGS: readonly string[] = ['it', 'en', 'de', 'fr', 'es', 'pt'];

/** La lingua della voce: l'inglese se scelto, altrimenti quella dell'interfaccia se ha le frasi registrate, se no l'inglese. */
export function voiceLangOf(uiLang: string, voiceChoice: string): string {
  if (voiceChoice === 'en') return 'en';
  return VOICE_LANGS.includes(uiLang) ? uiLang : 'en';
}

function speak(event: SoundEvent, call: string | undefined, volume: number): void {
  const lang = voiceLangOf(useLang.getState().lang, readVoiceLang());
  playChain(voiceParts(event, call, lang), volume);
}

// ---- file dell'utente, nel database del browser (condiviso con la finestra delle Impostazioni) ----

const DB = 'aurora-sounds';
const STORE = 'files';

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function tx<T>(mode: IDBTransactionMode, run: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const req = run(db.transaction(STORE, mode).objectStore(STORE));
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

const FILES_EVENT = 'aurora-sound-files';
const urls = new Map<string, string>();

/** Aggiunge un file (al massimo 5 MB); il nome del suono e' il nome del file. Restituisce l'identificativo da assegnare. */
export async function addSoundFile(file: File): Promise<string> {
  if (file.size > 5 * 1024 * 1024) throw new Error('too_big');
  const name = file.name.replace(/[^\w .()-]/g, '_').slice(0, 60) || 'suono';
  await tx('readwrite', (s) => s.put(file, name));
  // un file con lo stesso nome sostituisce il vecchio: l'indirizzo del vecchio si libera (prima restava in memoria fino alla chiusura)
  const old = urls.get(name);
  if (old) URL.revokeObjectURL(old);
  urls.delete(name);
  window.dispatchEvent(new Event(FILES_EVENT));
  return FILE_PREFIX + name;
}

export async function removeSoundFile(name: string): Promise<void> {
  await tx('readwrite', (s) => s.delete(name));
  const u = urls.get(name);
  if (u) URL.revokeObjectURL(u);
  urls.delete(name);
  window.dispatchEvent(new Event(FILES_EVENT));
}

export async function listSoundFiles(): Promise<string[]> {
  try {
    return ((await tx('readonly', (s) => s.getAllKeys())) as IDBValidKey[]).map(String).sort();
  } catch {
    return [];
  }
}

export function useSoundFiles(): string[] {
  const [names, setNames] = useState<string[]>([]);
  useEffect(() => {
    const update = () => void listSoundFiles().then(setNames);
    update();
    window.addEventListener(FILES_EVENT, update);
    window.addEventListener('focus', update);
    return () => {
      window.removeEventListener(FILES_EVENT, update);
      window.removeEventListener('focus', update);
    };
  }, []);
  return names;
}

async function fileUrl(name: string): Promise<string | null> {
  const cached = urls.get(name);
  if (cached) return cached;
  try {
    const blob = (await tx('readonly', (s) => s.get(name))) as Blob | undefined;
    if (!blob) return null;
    const u = URL.createObjectURL(blob);
    urls.set(name, u);
    return u;
  } catch {
    return null;
  }
}

// ---- un solo punto per suonare qualunque suono assegnato ----

/** Suona il suono assegnato a una funzione: un suono di Aurora, la voce o un file dell'utente. */
export function playAlert(id: string, volume: number, info: { event?: SoundEvent; call?: string } = {}, preview = false): void {
  if (!(volume > 0)) return;
  if (id === VOICE_ID) {
    speak(info.event ?? 'new_dxcc', info.call, volume);
    return;
  }
  if (id.startsWith(FILE_PREFIX)) {
    void fileUrl(id.slice(FILE_PREFIX.length)).then((u) => u && playChain([u], volume));
    return;
  }
  if (preview) previewSound(id, volume);
  else playSound(id, volume);
}

/** Il nome da mostrare per un suono che non e' di Aurora, o null. */
export function extraSoundName(id: string, voiceLabel: string): string | null {
  if (id === VOICE_ID) return voiceLabel;
  if (id.startsWith(FILE_PREFIX)) return id.slice(FILE_PREFIX.length);
  return null;
}
