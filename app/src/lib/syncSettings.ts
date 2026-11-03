/**
 * Le preferenze dell'interfaccia (chiavi `aurora-...` del browser) vengono copiate anche sul server di Aurora:
 * cosi' non si perdono cambiando indirizzo (127.0.0.1 o l'IP del PC), browser o dispositivo, ne' al riavvio.
 * Il telefono ha solo la sua disposizione (riquadri, pagina, indirizzi); tutto il resto (bande, macro, filtri, colori,
 * lingua, tema...) lo prende dalle impostazioni di Aurora sul PC, cosi' rispetta le stesse scelte.
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

/** Le preferenze proprie del telefono: la disposizione, diversa da quella del PC. Le altre sono quelle di Aurora. */
const PHONE_OWN = ['aurora-layout', 'aurora-phone-', 'aurora-last-page', 'aurora-known-addresses', 'aurora-fox-', 'aurora-settings-tab', 'aurora-update-dismissed'];

export function isPhoneOwn(key: string): boolean {
  return PHONE_OWN.some((p) => key === p || (p.endsWith('-') && key.startsWith(p)));
}

async function fetchKind(kind: DeviceKind, doFetch: Fetch): Promise<[string, string][] | null> {
  const response = await doFetch(`${BASE}/${kind}`, { credentials: 'include', signal: AbortSignal.timeout(LOAD_TIMEOUT_MS) });
  if (!response.ok) return null;
  const saved = (await response.json()) as Record<string, unknown>;
  return Object.entries(saved).filter((e): e is [string, string] => e[0].startsWith(PREFIX) && typeof e[1] === 'string');
}
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

async function put(kind: DeviceKind, entries: Record<string, string>, doFetch: Fetch): Promise<void> {
  await doFetch(`${BASE}/${kind}`, {
    method: 'PUT',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(entries),
  });
}

async function push(kind: DeviceKind, storage: Store, doFetch: Fetch): Promise<void> {
  try {
    await put(kind, collect(storage), doFetch);
  } catch {
    // il server non risponde: le preferenze restano nel browser e si riprova al prossimo cambio
  }
}

/** Una scelta condivisa cambiata dal telefono vale anche per Aurora: si fonde con quelle del PC senza toccarne la disposizione. */
async function pushShared(storage: Store, doFetch: Fetch): Promise<void> {
  try {
    const desktop = await fetchKind('desktop', doFetch);
    if (desktop === null) return;
    const merged: Record<string, string> = Object.fromEntries(desktop);
    for (const [key, value] of Object.entries(collect(storage))) if (!isPhoneOwn(key)) merged[key] = value;
    await put('desktop', merged, doFetch);
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
    let entries = await fetchKind(kind, doFetch);
    if (entries === null) return;
    // il telefono: la sua disposizione, ma le scelte condivise sono quelle di Aurora sul PC (il PC vince)
    if (kind === 'mobile') {
      const desktop = await fetchKind('desktop', doFetch).catch(() => null);
      if (desktop && desktop.length > 0) {
        entries = [...entries.filter(([key]) => isPhoneOwn(key)), ...desktop.filter(([key]) => !isPhoneOwn(key))];
      }
    }
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
  let shared = false;
  const schedule = (key: string) => {
    if (!key.startsWith(PREFIX)) return;
    if (kind === 'mobile' && !isPhoneOwn(key)) shared = true;
    clearTimeout(timer);
    timer = setTimeout(() => {
      void push(kind, storage, doFetch);
      if (shared) void pushShared(storage, doFetch);
      shared = false;
    }, SAVE_DELAY_MS);
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
