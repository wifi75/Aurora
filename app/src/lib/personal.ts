import { useEffect, useState } from 'react';
import { browserStorage } from './theme';

/**
 * I dati personali dell'operatore che non sono voci del motore: nome, cognome, QTH (citta') ed email facoltativa. Stanno dove la pagina
 * PSK·RTTY·CW teneva gia' «Il mio nome» e «Il mio QTH» (`aurora.digi.campi`, insieme ai rapporti RST dell'ultimo QSO), cosi' le macro
 * <MYNAME> e <MYQTH> e la pagina Stazione › Dati personali leggono e scrivono gli stessi valori: un posto solo.
 */
export interface PersonalFields {
  myname: string;
  mysurname: string;
  myqth: string;
  myemail: string;
  /** Rapporto mandato e ricevuto della pagina PSK·RTTY·CW (non sono dati personali, ma vivono nello stesso oggetto da sempre). */
  rsts: string;
  rstr: string;
}

export const PERSONAL_KEY = 'aurora.digi.campi';
const EVENT = 'aurora-personal';
const DEFAULTS: PersonalFields = { myname: '', mysurname: '', myqth: '', myemail: '', rsts: '599', rstr: '599' };

const text = (v: unknown, fallback: string): string => (typeof v === 'string' ? v : fallback);

export function readPersonal(): PersonalFields {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(PERSONAL_KEY) ?? 'null') as Partial<Record<keyof PersonalFields, unknown>> | null;
    if (!raw || typeof raw !== 'object') return { ...DEFAULTS };
    return {
      myname: text(raw.myname, ''),
      mysurname: text(raw.mysurname, ''),
      myqth: text(raw.myqth, ''),
      myemail: text(raw.myemail, ''),
      rsts: text(raw.rsts, '599'),
      rstr: text(raw.rstr, '599'),
    };
  } catch {
    return { ...DEFAULTS };
  }
}

/** Cambia alcuni campi e avvisa chi li mostra (questa finestra con l'evento, le altre con `storage`). */
export function savePersonal(patch: Partial<PersonalFields>): PersonalFields {
  const next = { ...readPersonal(), ...patch };
  try {
    browserStorage()?.setItem(PERSONAL_KEY, JSON.stringify(next));
  } catch {
    // senza archivio i valori valgono finche' la pagina e' aperta
  }
  try {
    window.dispatchEvent(new Event(EVENT));
  } catch {
    // fuori dal browser (prove) non c'e' nessuno da avvisare
  }
  return next;
}

/** I dati di adesso, aggiornati quando cambiano (anche dalle Impostazioni in una finestra a parte). */
export function usePersonal(): [PersonalFields, (patch: Partial<PersonalFields>) => void] {
  const [value, setValue] = useState<PersonalFields>(readPersonal);
  useEffect(() => {
    const sync = () => setValue(readPersonal());
    const onStorage = (e: StorageEvent) => {
      if (e.key === PERSONAL_KEY) sync();
    };
    window.addEventListener(EVENT, sync);
    window.addEventListener('storage', onStorage);
    return () => {
      window.removeEventListener(EVENT, sync);
      window.removeEventListener('storage', onStorage);
    };
  }, []);
  return [value, (patch) => setValue(savePersonal(patch))];
}
