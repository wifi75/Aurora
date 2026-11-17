/**
 * Quali voci della finestra delle impostazioni di un contest valgono per quel contest (`ContestEditDialog`).
 *
 * La finestra e' una sola, con tutte le voci; per ogni contest quelle che non gli servono si inibiscono (grigie, non modificabili,
 * «Non previsto da questo contest»). Una voce e' attiva solo se il contest la dichiara o le sue regole la usano:
 * - contest in cartella (`origine = 'cartella'`): la dichiarazione di `contest.toml` (`dichiarazione`, `scambio`) piu' le regole `[Score]`;
 * - vecchio `.contest`: le regole `[Score]` e, senza, le chiavi storiche (`multipliers`, `points_*`, `valid_*`...).
 * Un contest nuovo (senza `id`) non ha ancora un modo di essere: tutte le voci sono attive.
 */
import type { TextContest } from '../api/contestTesto';
import { hasExchMultiplier } from './contestSection';

/** Le voci della finestra, una per controllo (le bande hanno una voce per banda: `band:<banda>`). */
export const VOCI = [
  'name', 'notes', 'start', 'end', 'work_time',
  'nrs', 'auto_inc', 'mySect',
  'valid_cw', 'valid_phone', 'valid_digital', 'valid_contacts',
  'points_my_dxcc', 'points_my_cont', 'points_dx',
  'multipliers', 'multipliers_option', 'band_mult', 'qtc',
  'macros', 'merge_macros',
  'grab_number', 'grab_number_type', 'grab_time', 'grab_state', 'domain_file', 'grab_domain_to',
  'suggest_cq_zone', 'suggest_itu_zone', 'suggest_domain', 'suggest_dxcc_pfx',
  'dupe_no_band', 'dupe_no_mode', 'log_bar_mode',
] as const;
export type VoceFissa = (typeof VOCI)[number];
export type Voce = VoceFissa | `band:${string}`;

/** Le bande della finestra (le stesse di `newTextContest`). */
export const BANDE = ['160', '80', '40', '20', '15', '10'] as const;

export type CategoriaModo = 'cw' | 'phone' | 'digital';

/** Un contest e' in cartella (regole e contatori in sola lettura)? */
export const inCartella = (c: Pick<TextContest, 'origine'>): boolean => c.origine === 'cartella';

// ---------------------------------------------------------------- le regole `[Score]`

/** Cio' che serve delle regole `[Score]`: le bande ammesse, i moltiplicatori (tipo e se per banda) e se ci sono i modi. */
export interface RegoleScore {
  presenti: boolean;
  /** Le bande di `bands=` (`null` = nessun limite). */
  bande: string[] | null;
  mult: { tipo: string; perBanda: boolean }[];
  /** Le categorie di modo nominate da `mode:` nelle condizioni (per i vecchi file senza `modi` dichiarati). */
  modi: CategoriaModo[];
}

/** La categoria di un modo (la stessa del core: CW, fonia, il resto digitale). */
export function categoriaModo(m: string): CategoriaModo {
  const x = m.trim().toLowerCase();
  if (x === 'cw') return 'cw';
  if (['ssb', 'usb', 'lsb', 'am', 'fm', 'phone', 'fonia'].includes(x)) return 'phone';
  return 'digital';
}

/** Legge le righe `chiave=valore` della sezione `[Score]` (le righe che iniziano per `;` sono commenti). */
export function leggiScore(score: string | undefined): RegoleScore {
  const righe = new Map<string, string>();
  for (const r of (score ?? '').split(/\r?\n/)) {
    if (/^\s*;/.test(r)) continue;
    const m = /^\s*([A-Za-z_][\w]*)\s*=(.*)$/.exec(r);
    if (m && m[1]) righe.set(m[1].toLowerCase(), (m[2] ?? '').trim());
  }
  const bande = righe.get('bands');
  const mult: RegoleScore['mult'] = [];
  for (const pezzo of (righe.get('mult') ?? '').split(';')) {
    const p = pezzo.trim();
    if (p === '') continue;
    const [testa = '', ...opzioni] = p.split('/');
    mult.push({ tipo: testa.split(':')[0]?.trim().toLowerCase() ?? '', perBanda: opzioni.some((o) => o.trim().toLowerCase() === 'band') });
  }
  const modi = new Set<CategoriaModo>();
  for (const m of (score ?? '').matchAll(/(?:^|[=;&\s/])mode:([A-Za-z0-9+,-]+)/gim)) {
    for (const x of (m[1] ?? '').split(',')) if (x) modi.add(categoriaModo(x));
  }
  return {
    presenti: righe.size > 0,
    bande: bande ? bande.split(',').map((b) => b.trim()).filter(Boolean) : null,
    mult,
    modi: [...modi],
  };
}

/** `28 MHz`, `3.5 MHz`, `10m`, `10` -> la banda come nella finestra (`10`); sconosciuta -> `null`. */
export function bandaDi(testo: string): string | null {
  const t = testo.trim().toLowerCase().replace(',', '.');
  const mhz = /^([\d.]+)\s*mhz$/.exec(t);
  if (mhz) {
    const tab: Record<string, string> = { '1.8': '160', '3.5': '80', '5.3': '60', '7': '40', '10': '30', '14': '20', '18': '17', '21': '15', '24': '12', '28': '10', '50': '6' };
    const f = String(Number(mhz[1]));
    return tab[f] ?? null;
  }
  const m = /^(\d+)\s*m?$/.exec(t);
  return m?.[1] ?? null;
}

/** Le bande che il contest ammette (`null` = tutte): `[bande] ammesse` della cartella, altrimenti `bands=` delle regole. */
export function bandeAmmesse(c: TextContest): string[] | null {
  const dich = c.dichiarazione?.bande ?? [];
  if (inCartella(c) && dich.length > 0) return dich.map(bandaDi).filter((b): b is string => b !== null);
  return leggiScore(c.score).bande;
}

/** Le categorie di modo ammesse. Cartella: `[modi] ammessi` (nessuno scritto = tutte). Vecchio file: quelle accese nel file (nessuna = tutte) piu' quelle nominate dalle regole. */
export function modiAmmessi(c: TextContest): Set<CategoriaModo> {
  const tutte = new Set<CategoriaModo>(['cw', 'phone', 'digital']);
  if (inCartella(c) && c.dichiarazione) {
    const a = c.dichiarazione.modi.ammessi;
    return a.length === 0 ? tutte : new Set(a.map(categoriaModo));
  }
  const accese = new Set<CategoriaModo>();
  if (c.valid_cw) accese.add('cw');
  if (c.valid_phone) accese.add('phone');
  if (c.valid_digital) accese.add('digital');
  for (const m of leggiScore(c.score).modi) accese.add(m);
  return accese.size === 0 ? tutte : accese;
}

/** I tipi di moltiplicatore del contest: dalle regole `[Score]` o, senza, dalle chiavi storiche (1 DXCC, 2 WPX, 3 zona CQ, 4 zona ITU, 5 dominio). */
export function tipiMoltiplicatore(c: TextContest): Set<string> {
  const r = leggiScore(c.score);
  if (r.presenti) return new Set(r.mult.map((m) => m.tipo));
  const nome: Record<number, string> = { 1: 'dxcc', 2: 'wpx', 3: 'cq', 4: 'itu', 5: 'exch' };
  return new Set([c.multipliers, c.multipliers_option].map((n) => nome[n]).filter((x): x is string => x !== undefined));
}

/** I moltiplicatori hanno una banda ciascuno? Regole `[Score]`: un pezzo di `mult` con `band`; senza: la chiave storica con un moltiplicatore. */
export function moltiplicatoriPerBanda(c: TextContest): boolean {
  const r = leggiScore(c.score);
  if (r.presenti) return r.mult.some((m) => m.perBanda);
  return c.multipliers > 0 || c.multipliers_option > 0;
}

const haCampo = (campi: { tipo: string; progressivo?: boolean; elenco?: string }[] | undefined, f: (x: { tipo: string; progressivo?: boolean; elenco?: string }) => boolean) => (campi ?? []).some(f);
const haDominio = (c: TextContest): boolean => c.domain_file !== '' || haCampo(c.scambio?.ricevuto, (x) => (x.elenco ?? '') !== '');

/** La propria sezione / stato / club: il contest ha un moltiplicatore sullo scambio (`exch`) o la cartella dichiara un campo di testo da mandare. */
export function haSezionePropria(c: TextContest): boolean {
  return hasExchMultiplier(c.score) || (inCartella(c) && haCampo(c.scambio?.inviato, (x) => x.tipo === 'testo'));
}

// ---------------------------------------------------------------- l'elenco delle voci attive

/** La voce vale per questo contest? Se no la finestra la mostra grigia, non modificabile, con «Non previsto da questo contest». */
export function voceAttiva(c: TextContest, voce: Voce): boolean {
  if (voce === 'mySect') return c.id !== '' && haSezionePropria(c);
  if (c.id === '') return true; // un contest nuovo: ancora niente da escludere
  const cartella = inCartella(c);
  const regole = leggiScore(c.score);
  const dich = c.dichiarazione ?? null;
  if (voce.startsWith('band:')) {
    // la casella e il peso di una banda servono ai moltiplicatori per banda, e solo per le bande che il contest ammette
    const b = voce.slice(5);
    const ammesse = bandeAmmesse(c);
    return moltiplicatoriPerBanda(c) && (ammesse === null || ammesse.includes(b));
  }
  switch (voce as VoceFissa) {
    case 'name':
    case 'notes':
      return true;
    case 'start':
    case 'end':
      return cartella ? dich?.edizione != null : true;
    case 'work_time':
      return cartella ? (dich?.edizione?.ore_massime ?? null) !== null : true;
    case 'nrs':
    case 'auto_inc':
      return cartella ? haCampo(c.scambio?.inviato, (x) => x.progressivo === true || x.tipo === 'numero') : true;
    case 'valid_cw':
      return modiAmmessi(c).has('cw');
    case 'valid_phone':
      return modiAmmessi(c).has('phone');
    case 'valid_digital':
      return modiAmmessi(c).has('digital');
    case 'valid_contacts':
    case 'grab_time':
    case 'log_bar_mode':
      return !cartella;
    // le vecchie chiavi dei punti e dei moltiplicatori valgono solo senza regole `[Score]`
    case 'points_my_dxcc':
    case 'points_my_cont':
    case 'points_dx':
    case 'multipliers':
    case 'multipliers_option':
      return !cartella && !regole.presenti;
    case 'band_mult':
      return moltiplicatoriPerBanda(c);
    case 'qtc':
      return c.qtc > 0;
    case 'macros':
    case 'merge_macros':
      return cartella ? c.macros !== '' : true;
    case 'grab_number':
      return cartella ? haCampo(c.scambio?.ricevuto, (x) => x.tipo === 'numero') : true;
    case 'grab_number_type':
      return (cartella ? haCampo(c.scambio?.ricevuto, (x) => x.tipo === 'numero') : true) && c.grab_number;
    case 'grab_state':
    case 'domain_file':
    case 'grab_domain_to':
    case 'suggest_domain':
      return cartella ? haDominio(c) : true;
    case 'suggest_cq_zone':
      return tipiMoltiplicatore(c).has('cq');
    case 'suggest_itu_zone':
      return tipiMoltiplicatore(c).has('itu');
    case 'suggest_dxcc_pfx': {
      const t = tipiMoltiplicatore(c);
      return t.has('dxcc') || t.has('wpx');
    }
    case 'dupe_no_band': {
      // con una sola banda ammessa il doppio non dipende dalla banda
      const b = bandeAmmesse(c);
      return b === null || b.length !== 1;
    }
    case 'dupe_no_mode':
      return true;
    default:
      return true;
  }
}

/** La voce si puo' cambiare? In un contest in cartella le regole e i contatori sono in sola lettura (resta la propria sezione, che e' una preferenza di questo computer). */
export function voceModificabile(c: TextContest, voce: Voce): boolean {
  return inCartella(c) ? voce === 'mySect' : true;
}

// ---------------------------------------------------------------- il regolamento

/** Un blocco del regolamento (markdown semplice, reso senza HTML libero). */
export type BloccoRegolamento = { tipo: 'titolo'; livello: number; testo: string } | { tipo: 'paragrafo'; testo: string } | { tipo: 'elenco'; voci: string[] };

/** Divide un markdown semplice in titoli (`#`), elenchi (`-`, `*`, `1.`) e paragrafi (righe vicine unite); tutto il resto resta testo. */
export function blocchiRegolamento(md: string | null | undefined): BloccoRegolamento[] {
  const out: BloccoRegolamento[] = [];
  let par: string[] = [];
  let elenco: string[] | null = null;
  const chiudi = () => {
    if (par.length > 0) out.push({ tipo: 'paragrafo', testo: par.join(' ') });
    if (elenco) out.push({ tipo: 'elenco', voci: elenco });
    par = [];
    elenco = null;
  };
  for (const riga of (md ?? '').split(/\r?\n/)) {
    const r = riga.trim();
    const t = /^(#{1,6})\s+(.*)$/.exec(r);
    const v = /^(?:[-*+]|\d+[.)])\s+(.*)$/.exec(r);
    if (r === '') chiudi();
    else if (t) {
      chiudi();
      out.push({ tipo: 'titolo', livello: (t[1] ?? '#').length, testo: (t[2] ?? '').replace(/#+\s*$/, '').trim() });
    } else if (v) {
      if (par.length > 0) chiudi();
      (elenco ??= []).push(v[1] ?? '');
    } else {
      if (elenco) chiudi();
      par.push(r);
    }
  }
  chiudi();
  return out;
}

/** Un pezzo di testo di una riga del regolamento. */
export type PezzoInline = { tipo: 'testo' | 'grassetto' | 'codice'; testo: string } | { tipo: 'link'; testo: string; url: string };

const SOLO_WEB = /^https?:\/\/[^\s<>"']+$/i;

/** Divide una riga di markdown semplice in testo, **grassetto**, `codice` e link (`[testo](url)` e `<url>`); i link valgono solo se sono http o https. */
export function pezziInline(riga: string): PezzoInline[] {
  const out: PezzoInline[] = [];
  const re = /\*\*(.+?)\*\*|`([^`]+)`|\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)|<(https?:\/\/[^\s>]+)>/g;
  let da = 0;
  for (let m = re.exec(riga); m !== null; m = re.exec(riga)) {
    if (m.index > da) out.push({ tipo: 'testo', testo: riga.slice(da, m.index) });
    if (m[1] !== undefined) out.push({ tipo: 'grassetto', testo: m[1] });
    else if (m[2] !== undefined) out.push({ tipo: 'codice', testo: m[2] });
    else if (m[3] !== undefined && m[4] !== undefined && SOLO_WEB.test(m[4])) out.push({ tipo: 'link', testo: m[3], url: m[4] });
    else if (m[5] !== undefined && SOLO_WEB.test(m[5])) out.push({ tipo: 'link', testo: m[5], url: m[5] });
    else out.push({ tipo: 'testo', testo: m[0] });
    da = m.index + m[0].length;
  }
  if (da < riga.length) out.push({ tipo: 'testo', testo: riga.slice(da) });
  return out;
}
