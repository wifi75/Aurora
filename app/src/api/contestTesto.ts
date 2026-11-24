/** Le chiamate dei contest dei modi testuali (`/api/v1/contest-testo`, vedi `core/crates/remote-api/src/contest_testo_api.rs`). */
import { sinceParam } from '../lib/contestSince';
import { ContestError } from './contest';

const BASE = '/api/v1/contest-testo';

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(BASE + path, { credentials: 'include', ...init });
  if (!r.ok) {
    let code = 'error';
    let detail: string | undefined;
    try {
      const b = (await r.json()) as { error?: string; message?: string };
      code = b.error ?? code;
      detail = b.message;
    } catch {
      // corpo assente: resta il codice generico
    }
    throw new ContestError(r.status, code, detail);
  }
  return (await r.json()) as T;
}

const json = (method: string, body: unknown): RequestInit => ({ method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });

export interface BandMult {
  band: string;
  on: boolean;
  weight: number;
}

/** Un campo dello scambio dichiarato da un contest in cartella (`[[scambio.ricevuto]]`, `[[scambio.inviato]]`). */
export interface ExchangeField {
  nome: string;
  etichetta_it?: string;
  etichetta_en?: string;
  /** `rst`, `numero`, `testo`, `zona_cq` (zona CQ proposta dal nominativo) o `zona_itu` (zona ITU solo suggerita). */
  tipo: string;
  quando?: string;
  elenco?: string;
  progressivo?: boolean;
  motore?: boolean;
}

/** La parte della dichiarazione di un contest in cartella (`contest.toml`) che serve alle impostazioni (`dichiarazione` di `/voce/{id}`). */
export interface ContestDeclaration {
  /** Le bande ammesse come scritte nel file (`28 MHz`); vuoto = tutte. */
  bande: string[];
  modi: { ammessi: string[]; limite: { modo: string; frequenza_massima_khz?: number | null; frequenza_minima_khz?: number | null; nota?: string }[] };
  categorie: Record<string, unknown>;
  /** `modo`, `banda`, `banda+modo`, `sempre` (vuoto = banda e modo). */
  doppi: { per: string; nota?: string };
  /** Le date dell'edizione calcolate (`aaaammgghhmmss`) e i limiti d'orario; `null` senza `[edizione]`. */
  edizione: { mese: number; durata_ore: number; ore_massime: number | null; pausa_minima_min: number | null; invio_log_giorni: number | null; date: { anno: number; inizio: string; fine: string; confermata: boolean } | null } | null;
  cabrillo: Record<string, unknown>;
}

/** Un contest dei modi testuali, come nel file `.contest` (`docs/contest-file.md`). */
export interface TextContest {
  /** `cartella` (contest incluso o dell'utente in una cartella, regole in sola lettura) o `file` (vecchio `.contest`). */
  origine?: 'cartella' | 'file';
  /** Lo scambio dichiarato (vuoto per i vecchi file). */
  scambio?: { ricevuto: ExchangeField[]; inviato: ExchangeField[] };
  /** Le modifiche personali gia' applicate a un contest in cartella (date, ore, numero, macro; `null` o assente = valori della cartella). */
  sovrascritta?: { start?: string; end?: string; work_time?: number; nrs?: string; auto_inc?: boolean; macros?: string } | null;
  /** La dichiarazione del contest in cartella (solo da `/voce/{id}`; `null` per i file). */
  dichiarazione?: ContestDeclaration | null;
  /** Il regolamento in markdown semplice (solo da `/voce/{id}`; `null` per i file). */
  regolamento?: string | null;
  id: string;
  name: string;
  notes: string;
  /** Le regole del punteggio, righe della sezione `[Score]` (vuoto = le vecchie chiavi). */
  score?: string;
  /** Lo script dei casi speciali, testo Rhai della sezione `[Script]` (vuoto = nessuno). */
  script?: string;
  /** `aaaammgghhmmss` UTC, vuoto = nessun limite. */
  start: string;
  end: string;
  work_time: number;
  nrs: string;
  auto_inc: boolean;
  valid_contacts: number;
  valid_cw: boolean;
  valid_phone: boolean;
  valid_digital: boolean;
  points_my_dxcc: number;
  points_my_cont: number;
  points_dx: number;
  multipliers: number;
  multipliers_option: number;
  band_mult: boolean;
  bands: BandMult[];
  macros: string;
  merge_macros: boolean;
  grab_number: boolean;
  grab_number_type: number;
  grab_time: boolean;
  grab_state: boolean;
  domain_file: string;
  grab_domain_to: number;
  suggest_cq_zone: boolean;
  suggest_itu_zone: boolean;
  suggest_domain: boolean;
  suggest_dxcc_pfx: boolean;
  dupe_no_band: boolean;
  dupe_no_mode: boolean;
  /** Importato dall'altro programma e non ancora verificato sul regolamento ufficiale: sta nella scheda «Beta contest». */
  beta?: boolean;
  log_bar_mode: number;
  /** QTC del WAE: 0 nessuno, 1 dalle stazioni non europee alle europee (CW, SSB), 2 in entrambe le direzioni fra continenti diversi (RTTY). */
  qtc: number;
}

export interface TextContestRef {
  id: string;
  name: string;
  notes: string;
  start: string;
  end: string;
  beta?: boolean;
}

export interface TextContestList {
  attivo: string | null;
  contest: TextContestRef[];
  macros: string[];
  domini: string[];
}

export interface ContestMacro {
  name: string;
  label: string;
  text: string;
  color: number | null;
}

export interface DomainEntry {
  key: string;
  value: string;
  section: string;
}

export interface Domain {
  title: string;
  country: string;
  entries: DomainEntry[];
}

/** Il contest attivo con le sue macro e il suo elenco dei domini. */
export interface ActiveTextContest {
  contest: TextContest;
  macros: ContestMacro[];
  domain: Domain | null;
  start_ms: number | null;
  end_ms: number | null;
}

export const getTextContests = () => call<TextContestList>('');
export const getActiveTextContest = () => call<ActiveTextContest | null>('/attivo');
export const getTextContest = (id: string) => call<TextContest>(`/voce/${encodeURIComponent(id)}`);
/** Salva un contest. Per un contest in cartella il core tiene solo le voci personali (date, ore, numero, macro) e risponde con il contest aggiornato. */
export const saveTextContest = (c: TextContest) => (c.id ? call<{ ok?: boolean; id: string }>(`/voce/${encodeURIComponent(c.id)}`, json('PUT', c)) : call<{ ok?: boolean; id: string }>('/voce', json('POST', c)));
/** «Ripristina i valori del contest»: toglie le modifiche personali di un contest in cartella e risponde con il contest com'e' nella cartella. */
export const resetTextContest = (id: string) => call<TextContest>(`/voce/${encodeURIComponent(id)}/sovrascrittura`, { method: 'DELETE' });
/** «Copia nella mia cartella»: copia un contest incluso nei dati dell'utente, dove si modifica (e sostituisce quello incluso). */
export const copyTextContest = (id: string) => call<{ ok: boolean; id: string }>(`/voce/${encodeURIComponent(id)}/copia`, { method: 'POST' });
export const deleteTextContest = (id: string) => call<{ ok: boolean }>(`/voce/${encodeURIComponent(id)}`, { method: 'DELETE' });
/** Importa un contest: pacchetto `.zip` (`.contest` con `macros/` e `dom/`) o `.contest` da solo; il core lo controlla e lo aggiunge. */
export const importTextContest = async (file: File) =>
  call<{ ok: boolean; id: string }>(`/importa?nome=${encodeURIComponent(file.name)}`, { method: 'POST', headers: { 'content-type': 'application/octet-stream' }, body: await file.arrayBuffer() });
/** Il pacchetto `.zip` di un contest con le sue macro e il suo elenco dei domini. */
export async function exportTextContest(id: string): Promise<Blob> {
  const r = await fetch(`${BASE}/voce/${encodeURIComponent(id)}/pacchetto`, { credentials: 'include' });
  if (!r.ok) {
    let code = 'error';
    let detail: string | undefined;
    try {
      const b = (await r.json()) as { error?: string; message?: string };
      code = b.error ?? code;
      detail = b.message;
    } catch {
      // corpo assente: resta il codice generico
    }
    throw new ContestError(r.status, code, detail);
  }
  return r.blob();
}
/** Una riga della statistica per banda del contest attivo. */
export interface TextContestBand {
  band: string;
  qso: number;
  doppi: number;
  punti: number;
  molt: number;
  cw: number;
  phone: number;
  digital: number;
}

/** Il punteggio del contest attivo calcolato dal core sui QSO del log (doppi esclusi). */
export interface TextContestScore {
  qso: number;
  doppi: number;
  punti: number;
  molt: number;
  punteggio: number;
  /** QTC mandati e ricevuti (contest WAE), contati con i punti. */
  qtc: number;
  tipi: string[];
  bande: TextContestBand[];
  /** QSO non ammessi dalle regole (banda o paese fuori dal contest). */
  esclusi: number;
  /** I moltiplicatori lavorati: tipo (`dxcc`, `cq`, `itu`, `wpx`, `cont`, `area`, `dom`), valore, bande (vuoto = una volta sola). */
  griglia: { tipo: string; valore: string; bande: string[] }[];
  /** I moltiplicatori che mancano dei tipi con un elenco chiuso. */
  mancanti: { tipo: string; per_banda: boolean; totali: number; valori: string[] }[];
  /** Gli ultimi QSO che hanno portato moltiplicatori nuovi (`dxcc:DL`). */
  recenti: { call: string; band: string; time_ms: number; nuovi: string[] }[];
  /** QSO all'ora sugli ultimi 10 minuti, QSO nell'ultima ora, media all'ora. */
  rate10: number;
  rate60: number;
  media_ora: number;
  /** L'errore dello script della sezione `[Script]` (il punteggio e' quello della sola `[Score]`), null se non c'e'. */
  errore_script?: string | null;
}

/** Il valore di un nominativo nel contest attivo mentre lo si scrive. */
export interface TextContestCheck {
  ammesso: boolean;
  motivo: 'band' | 'valid' | null;
  doppio: boolean;
  punti: number;
  nuovi: string[];
  /** Lo scambio di questo nominativo ha un terzo elemento dopo il numero (sezione, club, stato)? Calcolato dal core con le regole del punteggio (assente con un core vecchio). */
  exch_atteso?: boolean;
  /** L'ultimo QSO del contest con lo stesso nominativo (assente con un core vecchio). */
  precedente?: { time_ms: number; band: string; mode: string; exch_rcvd: string; exch_sent: string } | null;
  /** Lo scambio mandato nell'ultimo QSO del log del contest (`<PREVNRS>`). */
  ultimo_mandato?: string | null;
  /** Nominativo e rapporto mandato dell'ultimo QSO del log del contest (`<PREVCALL>`, `<PREVRSTS>`). */
  ultimo?: { call: string; rst_sent: string } | null;
}
export const getTextContestCheck = (call_: string, band: string, mode: string, exch = '') =>
  call<TextContestCheck>(`/ammesso?${new URLSearchParams({ call: call_, band, mode, exch }).toString()}${sinceParam(false)}`);

export const getTextContestScore = () => call<TextContestScore>(`/punteggio${sinceParam(true)}`);
/** Un punto da controllare nel Cabrillo: `empty`, `exch_sent`, `exch_rcvd`, `outside`, `dupes`, con quanti QSO riguarda. */
export interface CabrilloWarning {
  codice: 'empty' | 'exch_sent' | 'exch_rcvd' | 'outside' | 'dupes';
  quanti: number;
}
export const getTextContestCabrillo = () => call<{ testo: string; nome: string; qso: number; avvisi?: CabrilloWarning[] }>(`/cabrillo${sinceParam(true)}`);
/** Un QTC mandato o ricevuto (vedi `core/crates/contest/src/qtc.rs`). */
export interface Qtc {
  inviato: boolean;
  corrispondente: string;
  serie: number;
  quanti: number;
  time_ms: number;
  band: string;
  mode: string;
  freq_hz: number;
  ora: string;
  call: string;
  nr: string;
  qso_ms: number;
}

/** Perche' i QTC non si possono scambiare con una stazione. */
export type QtcBlock = 'stesso_continente' | 'direzione_sbagliata' | 'esauriti';

export interface QtcProposal {
  ora: string;
  call: string;
  nr: string;
  qso_ms: number;
}

/** Lo stato dei QTC con una stazione. */
export interface QtcStation {
  call: string;
  continente: string;
  mio_continente: string;
  scambiati: number;
  residuo: number;
  serie: number;
  invio: QtcBlock | null;
  ricezione: QtcBlock | null;
  proposte: QtcProposal[];
}

export interface QtcState {
  regola: 'classica' | 'rtty' | null;
  qtc: Qtc[];
  stazione: QtcStation | null;
}

export const getQtc = (station: string) => call<QtcState>(`/qtc?call=${encodeURIComponent(station)}${sinceParam(false)}`);
export const saveQtc = (qtc: Qtc[]) => call<{ ok: boolean; totale: number }>('/qtc', json('POST', { qtc }));
export const selectTextContest =(id: string | null) => call<{ ok: boolean; attivo: string | null }>('/seleziona', json('POST', { id }));

/** Un contest nuovo con i valori di base (digitali validi, incremento automatico, macro unite). */
export function newTextContest(): TextContest {
  return {
    id: '', name: '', notes: '', start: '', end: '', work_time: 0, nrs: '', auto_inc: true, valid_contacts: 0, valid_cw: false, valid_phone: false, valid_digital: true,
    points_my_dxcc: 0, points_my_cont: 0, points_dx: 0, multipliers: 0, multipliers_option: 0, band_mult: false,
    bands: ['160', '80', '40', '20', '15', '10'].map((band) => ({ band, on: false, weight: 0 })),
    macros: '', merge_macros: true, grab_number: true, grab_number_type: -1, grab_time: false, grab_state: false, domain_file: '', grab_domain_to: 0,
    suggest_cq_zone: false, suggest_itu_zone: false, suggest_domain: false, suggest_dxcc_pfx: false, dupe_no_band: false, dupe_no_mode: false, log_bar_mode: 0, qtc: 0,
  };
}
