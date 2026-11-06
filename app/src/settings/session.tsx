import { createContext, useContext } from 'react';

/**
 * La bozza unica della finestra delle Impostazioni: le schede del motore scrivono qui e non salvano da sole; i pulsanti in basso
 * (Applica, Salva, Annulla) mandano o buttano tutto insieme. Fuori dalla finestra non c'e' e ogni scheda ha la sua bozza.
 */
export interface SharedDraft {
  changes: Record<string, string>;
  set(key: string, value: string, current: string | undefined): void;
  error?: { key: string | null; text: string } | null;
  setError?(e: { key: string | null; text: string } | null): void;
  clear(): void;
  /** Bozze di schede che non sono voci del motore (per esempio le regole delle Automazioni), per chiave; partono con Applica o Salva. */
  drafts: Record<string, unknown>;
  setDraftOf(key: string, value: unknown): void;
}

export const SessionDraft = createContext<SharedDraft | null>(null);

export function useSharedDraft(): SharedDraft | null {
  return useContext(SessionDraft);
}
