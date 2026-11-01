import { FLAG_TABLE } from './flagTable';

/**
 * Il nome di un paese ridotto all'essenziale, per riconoscere le grafie di fonti diverse: NG3K scrive «Gambia», «Solomon Is», «eSwatini»
 * dove `cty.dat` ha «The Gambia», «Solomon Islands», «Kingdom of Eswatini».
 */
export function countryKey(name: string): string {
  return name
    .toLowerCase()
    .replace(/&/g, ' and ')
    .replace(/\bst\.?\s/g, 'saint ')
    .replace(/\bis\.?$/, 'islands')
    .replace(/\bi\.?$/, 'island')
    .replace(/\b(the|kingdom of|republic of|rep\. of|fed\. rep\. of|federal republic of)\b/g, ' ')
    .replace(/[^a-z0-9]/g, '');
}

let byKey: Map<string, string> | null = null;

/** Codice della bandiera (classe `fi-<codice>` di flag-icons) per il nome di un'entita' DXCC; `null` se non ce n'e'. */
export function flagCode(country: string | null | undefined): string | null {
  if (!country) return null;
  const exact = FLAG_TABLE[country];
  if (exact) return exact;
  if (!byKey) byKey = new Map(Object.entries(FLAG_TABLE).map(([name, code]) => [countryKey(name), code]));
  return byKey.get(countryKey(country)) ?? null;
}
