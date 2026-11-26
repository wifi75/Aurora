/**
 * Le preferenze dell'interfaccia (chiavi `aurora-...` del browser) vengono copiate anche sul server di Aurora:
 * cosi' non si perdono cambiando browser ne' al riavvio. Le scrive solo chi le cambia davvero (dopo un clic o un tasto) e solo le voci
 * cambiate. Il PC principale (finestra del programma o browser su 127.0.0.1) tiene le impostazioni `desktop`; un client remoto
 * (telefono, tablet, portatile in rete) legge quelle del PC ma salva solo la sua disposizione, nelle `mobile`: non riscrive mai il PC.
 */

/** Le preferenze dell'interfaccia: `aurora-...` e `aurora....` (le disposizioni delle finestre: colonne, larghezze, altezze). */
const isAuroraKey = (k: string): boolean => k.startsWith('aurora-') || k.startsWith('aurora.');
const BASE = '/api/v1/ui-settings';
const SAVE_DELAY_MS = 400;
const LOAD_TIMEOUT_MS = 1500;

export type DeviceKind = 'desktop' | 'mobile';

/** Il tipo si decide da come e' collegata la pagina, non dalla larghezza: il PC principale e' `desktop`, ogni client remoto `mobile`. */
export function deviceKind(remote: boolean): DeviceKind {
  return remote ? 'mobile' : 'desktop';
}

type Store = Pick<Storage, 'getItem' | 'setItem' | 'key' | 'length'>;

/** Le preferenze proprie del telefono: la disposizione, diversa da quella del PC. Le altre sono quelle di Aurora. */
const PHONE_OWN = ['aurora-layout', 'aurora-phone-', 'aurora-last-page', 'aurora-known-addresses', 'aurora-fox-', 'aurora-settings-tab', 'aurora-update-dismissed', 'aurora-ui-scale'];

export function isPhoneOwn(key: string): boolean {
  return PHONE_OWN.some((p) => key === p || (p.endsWith('-') && key.startsWith(p)));
}

async function fetchKind(kind: DeviceKind, doFetch: Fetch): Promise<[string, string][] | null> {
  const response = await doFetch(`${BASE}/${kind}`, { credentials: 'include', signal: AbortSignal.timeout(LOAD_TIMEOUT_MS) });
  if (!response.ok) return null;
  const saved = (await response.json()) as Record<string, unknown>;
  return Object.entries(saved).filter((e): e is [string, string] => isAuroraKey(e[0]) && typeof e[1] === 'string');
}
type Fetch = typeof fetch;

/** Le preferenze che il browser tiene adesso. */
export function collect(storage: Store): Record<string, string> {
  const out: Record<string, string> = {};
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    const value = key != null && isAuroraKey(key) ? storage.getItem(key) : null;
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

/**
 * Da chiamare prima di disegnare l'interfaccia: se il server ha le preferenze le porta nel browser
 * (il server vince); se non ne ha ancora, il PC principale ci manda quelle del browser (un client remoto no). Non blocca oltre il tempo massimo.
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
      if (kind === 'desktop' && Object.keys(collect(storage)).length > 0) await put(kind, collect(storage), doFetch);
      return;
    }
    for (const [key, value] of entries) storage.setItem(key, value);
  } catch {
    // senza server (o non ancora associato) si usa quello che c'e' nel browser
  }
}

/** Le voci cambiate fuse con quelle che il server tiene gia' (il server sostituisce tutto il tipo a ogni PUT). */
async function putChanges(kind: DeviceKind, changes: Map<string, string | null>, doFetch: Fetch): Promise<boolean> {
  const current = await fetchKind(kind, doFetch);
  if (current === null) return false;
  const merged: Record<string, string> = Object.fromEntries(current);
  for (const [key, value] of changes) {
    if (value === null) delete merged[key];
    else merged[key] = value;
  }
  await put(kind, merged, doFetch);
  return true;
}

export type SyncOptions = { storage?: Storage; target?: EventTarget };

/**
 * Dopo ogni modifica di una preferenza manda al server solo le voci cambiate, senza martellarlo a ogni tasto. Si arma al primo gesto
 * dell'utente (clic o tasto): quello che l'interfaccia scrive da sola all'apertura non parte. Un client remoto salva solo la sua
 * disposizione. Restituisce la funzione che smette di seguire i cambi.
 */
export function keepSynced(kind: DeviceKind, doFetch: Fetch = fetch, options: SyncOptions = {}): () => void {
  let storage: Storage;
  let target: EventTarget;
  try {
    storage = options.storage ?? window.localStorage;
    target = options.target ?? window;
  } catch {
    return () => undefined;
  }
  let armed = false;
  const arm = () => {
    armed = true;
    target.removeEventListener('pointerdown', arm, true);
    target.removeEventListener('keydown', arm, true);
  };
  target.addEventListener('pointerdown', arm, true);
  target.addEventListener('keydown', arm, true);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const changes = new Map<string, string | null>();
  const flush = () => {
    const batch = new Map(changes);
    changes.clear();
    void putChanges(kind, batch, doFetch)
      .then((ok) => {
        // il server non risponde: le voci si rimandano al prossimo cambio (senza scavalcare quelle nel frattempo piu' nuove)
        if (!ok) for (const [k, v] of batch) if (!changes.has(k)) changes.set(k, v);
      })
      .catch(() => {
        for (const [k, v] of batch) if (!changes.has(k)) changes.set(k, v);
      });
  };
  const schedule = (key: string, value: string | null) => {
    if (!armed || !isAuroraKey(key)) return;
    if (kind === 'mobile' && !isPhoneOwn(key)) return;
    changes.set(key, value);
    clearTimeout(timer);
    timer = setTimeout(flush, SAVE_DELAY_MS);
  };
  const { setItem, removeItem } = Storage.prototype;
  Storage.prototype.setItem = function (this: Storage, key: string, value: string) {
    // una scrittura identica non cambia niente: non si rimanda tutto il gruppo al server
    const same = this === storage && this.getItem(key) === String(value);
    setItem.call(this, key, value);
    if (this === storage && !same) schedule(key, String(value));
  };
  Storage.prototype.removeItem = function (this: Storage, key: string) {
    removeItem.call(this, key);
    if (this === storage) schedule(key, null);
  };
  return () => {
    clearTimeout(timer);
    arm();
    Storage.prototype.setItem = setItem;
    Storage.prototype.removeItem = removeItem;
  };
}
