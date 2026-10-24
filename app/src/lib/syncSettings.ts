/**
 * Le preferenze dell'interfaccia (chiavi `aurora-...` del browser) vengono copiate anche sul server di Aurora:
 * cosi' non si perdono cambiando indirizzo (127.0.0.1 o l'IP del PC), browser o dispositivo, ne' al riavvio.
 * Il telefono ha le sue (disposizione diversa), il resto e' condiviso.
 */

const PREFIX = 'aurora-';
const BASE = '/api/v1/ui-settings';
const SAVE_DELAY_MS = 400;
const LOAD_TIMEOUT_MS = 1500;

export type DeviceKind = 'desktop' | 'mobile';

export function deviceKind(width: number): DeviceKind {
  return width < 768 ? 'mobile' : 'desktop';
}

type Store = Pick<Storage, 'getItem' | 'setItem' | 'key' | 'length'>;
type Fetch = typeof fetch;

/** Le preferenze che il browser tiene adesso. */
export function collect(storage: Store): Record<string, string> {
  const out: Record<string, string> = {};
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    const value = key?.startsWith(PREFIX) ? storage.getItem(key) : null;
    if (key && value !== null) out[key] = value;
  }
  return out;
}

async function push(kind: DeviceKind, storage: Store, doFetch: Fetch): Promise<void> {
  try {
    await doFetch(`${BASE}/${kind}`, {
      method: 'PUT',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(collect(storage)),
    });
  } catch {
    // il server non risponde: le preferenze restano nel browser e si riprova al prossimo cambio
  }
}

/**
 * Da chiamare prima di disegnare l'interfaccia: se il server ha le preferenze le porta nel browser
 * (il server vince); se non ne ha ancora, ci manda quelle del browser. Non blocca oltre il tempo massimo.
 */
export async function hydrate(storage: Store, kind: DeviceKind, doFetch: Fetch = fetch): Promise<void> {
  try {
    const response = await doFetch(`${BASE}/${kind}`, { credentials: 'include', signal: AbortSignal.timeout(LOAD_TIMEOUT_MS) });
    if (!response.ok) return;
    const saved = (await response.json()) as Record<string, unknown>;
    const entries = Object.entries(saved).filter((e): e is [string, string] => e[0].startsWith(PREFIX) && typeof e[1] === 'string');
    if (entries.length === 0) {
      if (Object.keys(collect(storage)).length > 0) await push(kind, storage, doFetch);
      return;
    }
    for (const [key, value] of entries) storage.setItem(key, value);
  } catch {
    // senza server (o non ancora associato) si usa quello che c'e' nel browser
  }
}

/** Dopo ogni modifica di una preferenza la manda al server, senza martellarlo a ogni tasto. */
export function keepSynced(kind: DeviceKind, doFetch: Fetch = fetch): void {
  let storage: Storage;
  try {
    storage = window.localStorage;
  } catch {
    return;
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  const schedule = (key: string) => {
    if (!key.startsWith(PREFIX)) return;
    clearTimeout(timer);
    timer = setTimeout(() => void push(kind, storage, doFetch), SAVE_DELAY_MS);
  };
  const { setItem, removeItem } = Storage.prototype;
  Storage.prototype.setItem = function (this: Storage, key: string, value: string) {
    setItem.call(this, key, value);
    if (this === storage) schedule(key);
  };
  Storage.prototype.removeItem = function (this: Storage, key: string) {
    removeItem.call(this, key);
    if (this === storage) schedule(key);
  };
}
