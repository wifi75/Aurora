/** I dati con cui si riempiono le macro dei modi testuali. */
export interface MacroFields {
  mycall: string;
  call: string;
  rst: string;
  name: string;
  qth: string;
}

/**
 * Sostituisce le parole fra `<` e `>` (`<MYCALL>`, `<CALL>`, `<RST>`, `<MYNAME>`, `<MYQTH>`) con i dati della stazione e del QSO.
 * Una parola sconosciuta resta com'è; un dato vuoto toglie la parola e lo spazio doppio che lascerebbe.
 */
export function expandMacro(text: string, fields: MacroFields): string {
  const values: Record<string, string> = {
    MYCALL: fields.mycall,
    CALL: fields.call,
    RST: fields.rst,
    MYNAME: fields.name,
    MYQTH: fields.qth,
  };
  return text
    .replace(/<([A-Z]+)>/g, (whole, key: string) => (key in values ? (values[key] ?? '').trim().toUpperCase() : whole))
    .replace(/ {2,}/g, ' ')
    .trim();
}
