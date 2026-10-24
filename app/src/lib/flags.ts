import { FLAG_TABLE } from './flagTable';

/** Codice della bandiera (classe `fi-<codice>` di flag-icons) per il nome di un'entita' DXCC; `null` se non ce n'e'. */
export function flagCode(country: string | null | undefined): string | null {
  if (!country) return null;
  return FLAG_TABLE[country] ?? null;
}
