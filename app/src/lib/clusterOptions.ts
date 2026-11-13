import { useEffect, useState } from 'react';
import { browserStorage } from './theme';

/** Le colonne dello spot che si possono nascondere. */
export const CLUSTER_COLUMNS = ['de', 'freq', 'dx', 'country', 'mode', 'comment', 'time'] as const;
export type ClusterColumn = (typeof CLUSTER_COLUMNS)[number];

/**
 * Le opzioni del DX Cluster nelle pagine PSK, RTTY e CW (la rotella): come si sceglie uno spot, se il rotore gira verso la
 * stazione, dopo quanti minuti uno spot sparisce e quali colonne si vedono. Restano su questo dispositivo.
 */
export interface ClusterOptions {
  /** Come si sceglie uno spot: con il doppio clic (di base) o con un clic solo (SingleClick). */
  click: 'double' | 'single';
  /** Scegliendo uno spot il rotore gira da solo verso quella stazione. */
  rotor: boolean;
  /** Gli spot piu' vecchi di tanti minuti non si mostrano; 0 = restano finche' il cluster li tiene (ExpirationTime). */
  expireMin: number;
  /** Colonne nascoste. */
  hidden: ClusterColumn[];
}

export const DEFAULT_CLUSTER_OPTIONS: ClusterOptions = { click: 'double', rotor: false, expireMin: 0, hidden: [] };

/** Dopo quanti minuti uno spot sparisce: quelli offerti nella rotella. */
export const EXPIRE_CHOICES = [0, 5, 10, 15, 20, 30, 45, 60, 120] as const;

const KEY = 'aurora.cluster.opts';
const EVENT = 'aurora-cluster-opts';

export function readClusterOptions(): ClusterOptions {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(KEY) ?? 'null') as Partial<Record<keyof ClusterOptions, unknown>> | null;
    if (!raw || typeof raw !== 'object') return { ...DEFAULT_CLUSTER_OPTIONS };
    const hidden = Array.isArray(raw.hidden) ? CLUSTER_COLUMNS.filter((c) => (raw.hidden as unknown[]).includes(c)) : [];
    return {
      click: raw.click === 'single' ? 'single' : 'double',
      rotor: raw.rotor === true,
      expireMin: typeof raw.expireMin === 'number' && raw.expireMin >= 0 && raw.expireMin <= 720 ? Math.round(raw.expireMin) : 0,
      hidden,
    };
  } catch {
    return { ...DEFAULT_CLUSTER_OPTIONS };
  }
}

export function saveClusterOptions(o: ClusterOptions): void {
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(o));
  } catch {
    // senza archivio la scelta vale finche' la pagina e' aperta
  }
  window.dispatchEvent(new Event(EVENT));
}

/** Le opzioni di adesso, che si aggiornano quando cambiano (anche da un'altra finestra). */
export function useClusterOptions(): [ClusterOptions, (patch: Partial<ClusterOptions>) => void] {
  const [o, setO] = useState<ClusterOptions>(readClusterOptions);
  useEffect(() => {
    const sync = () => setO(readClusterOptions());
    window.addEventListener(EVENT, sync);
    window.addEventListener('storage', sync);
    return () => {
      window.removeEventListener(EVENT, sync);
      window.removeEventListener('storage', sync);
    };
  }, []);
  return [o, (patch) => saveClusterOptions({ ...readClusterOptions(), ...patch })];
}
