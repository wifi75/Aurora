/**
 * Il telefono si ricorda dove trovare Aurora: gli indirizzi con cui si e' collegato (gli IP del PC e `aurora.local`).
 * Se il collegamento cade, si provano gli altri e si passa a quello che risponde.
 *
 * Attenzione: la sessione (cookie) vale per un solo indirizzo. Passando da un IP a un altro va rifatta l'associazione;
 * con `aurora.local` l'indirizzo resta sempre lo stesso e il problema non c'e'.
 */

const KEY = 'aurora-known-addresses';
const MAX = 6;
/** Quanto si aspetta un indirizzo prima di considerarlo spento. */
const PROBE_MS = 2500;

type Store = Pick<Storage, 'getItem' | 'setItem'> | null;

/** `http://192.168.1.20:8787`: solo schema, host e porta, niente altro. */
function clean(url: string): string | null {
  try {
    const u = new URL(url);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    return u.origin;
  } catch {
    return null;
  }
}

export function knownAddresses(storage: Store): string[] {
  try {
    const raw = storage?.getItem(KEY);
    const list: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(list) ? list.filter((x): x is string => typeof x === 'string' && clean(x) === x) : [];
  } catch {
    return [];
  }
}

/** Aggiunge gli indirizzi buoni in testa (i nomi `.local` prima degli IP: non cambiano mai). */
export function rememberAddresses(storage: Store, urls: string[]): void {
  const fresh = urls.map(clean).filter((x): x is string => x !== null);
  const all = [...fresh, ...knownAddresses(storage)].filter((x, i, a) => a.indexOf(x) === i);
  all.sort((a, b) => Number(new URL(b).hostname.endsWith('.local')) - Number(new URL(a).hostname.endsWith('.local')));
  try {
    storage?.setItem(KEY, JSON.stringify(all.slice(0, MAX)));
  } catch {
    // senza archivio il telefono non ricorda gli indirizzi: resta il QR
  }
}

/** L'indirizzo fisso dal nome del QR (`aurora.local`) con la stessa porta e lo stesso schema della pagina. */
export function addressFromName(name: string, here: Location): string | null {
  if (!/^[a-zA-Z0-9.-]+$/.test(name)) return null;
  return clean(`${here.protocol}//${name}${here.port ? `:${here.port}` : ''}`);
}

/** Vero se l'indirizzo risponde (la risposta non si legge: basta che arrivi). */
async function answers(origin: string): Promise<boolean> {
  const ctrl = new AbortController();
  const timer = window.setTimeout(() => ctrl.abort(), PROBE_MS);
  try {
    await fetch(`${origin}/api/v1/health`, { mode: 'no-cors', cache: 'no-store', signal: ctrl.signal });
    return true;
  } catch {
    return false;
  } finally {
    window.clearTimeout(timer);
  }
}

/** Il primo degli altri indirizzi noti che risponde, oppure null. */
export async function findOtherAddress(storage: Store, current: string): Promise<string | null> {
  for (const origin of knownAddresses(storage)) {
    if (origin === current) continue;
    if (await answers(origin)) return origin;
  }
  return null;
}
