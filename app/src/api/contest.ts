/** Le chiamate della pagina Contest (`/api/v1/contest`, vedi `core/crates/remote-api/src/contest_api.rs`). */

const BASE = '/api/v1/contest';

export class ContestError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly detail?: string,
  ) {
    super(detail || `${status} ${code}`);
    this.name = 'ContestError';
  }
}

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

export interface ContestRef {
  id: string;
  nome: string;
  famiglia: string;
  edizione: number;
  /** L'edizione in corso o la prossima, calcolata dalla regola del calendario (assente per i contest «sempre»). */
  finestra?: { inizio_ms: number; fine_ms: number } | null;
  da_verificare?: boolean;
  url_regole?: string;
  ore_massime?: number | null;
  pausa_minima_min?: number | null;
}

export interface Categoria {
  id: string;
  nome: string;
  operatore: string;
  potenza: string;
  trasmettitori: string;
  bande: string;
  assistenza: string;
  /** Sigla corta per la barra (vuota = id). */
  sigla?: string;
  /** Spiegazione breve in inglese (vuota = nome). */
  en?: string;
  stazione?: string;
}

/** Le scelte lasciate dal regolamento oltre alla categoria. */
export interface OpzioniCategoria {
  potenze: string[];
  assistenza: boolean;
  classe_fd: boolean;
}

export interface Regole {
  contest: { id: string; nome: string; sponsor: string; edizione: number; url_regole: string; verificato_il: string; famiglia: string; da_verificare: string[] };
  sintesi: { it: string; en: string };
  calendario: { regola: string; inizio: string; fine: string; ore_massime?: number | null; pausa_minima_min?: number | null };
  bande: { ammesse: string[]; frequenze: Record<string, Record<string, number>> };
  modi: { ammessi: string[]; stesso_modo_per_doppi: boolean };
  messaggi: { attivita: string; parola_cq: string; chiusura: string; settantatre_finale: string };
  scambio: { inviato: string[]; ricevuto: string[]; predefinito: string; seriale: string };
  doppi: { chiave: string; log_doppi: string };
  punti: { tipo: string };
  moltiplicatori: { tipo: string; ambito: string }[];
  punteggio: { formula: string };
  categorie: Categoria[];
  opzioni_categoria?: OpzioniCategoria;
  restrizioni: { avvio_umano: boolean; fox_hound: string; multi_flusso: string; autospot: string };
  export: { formato: string; cabrillo_contest: string; invio_url: string; scadenza: string };
}

export interface Impostazioni {
  attivita: string;
  parola_cq: string;
  scambio: string;
  seriale_partenza: number;
  categoria: string;
  potenza_w: number;
  bonus: number;
  modo: string;
  hold_tx: boolean;
  rrr: boolean;
  grid6: string;
  /** Antenna per l'intestazione EDI (`SAnte`). */
  antenna?: string;
  operatori: string;
  club: string;
  nome: string;
  location: string;
  soapbox: string;
  assistenza: string;
  cabrillo_contest: string;
  inizio_ms: number | null;
  fine_ms: number | null;
  potenza?: string;
  trasmettitori_n?: number;
  call_contest?: string;
  indirizzo?: string;
  email?: string;
}

export interface TotaleBanda {
  banda: string;
  qso: number;
  punti: number;
  molt: number;
  ultimo_ms: number;
}

export interface Totali {
  qso: number;
  doppi: number;
  incerti: number;
  punti: number;
  molt: number;
  punteggio: number;
  km_max: number;
  per_banda: TotaleBanda[];
}

export interface ContestStato {
  attivo: string | null;
  contest: ContestRef[];
  errori_regole: string[];
  regole?: Regole;
  impostazioni?: Impostazioni;
  sequenza?: { attivita: string; parola_cq: string; scambio: string; seriale: number } | null;
  stazione?: { call: string; grid: string };
  scambio_inviato?: string;
  categoria?: string;
  categoria_nome?: string;
  finestra?: { inizio_ms: number; fine_ms: number } | null;
  fase?: 'prima' | 'in-corso' | 'finito' | 'sempre';
  ore_restanti?: number;
  ore_usate?: number;
  ore_massime?: number | null;
  totali?: Totali;
  rate10?: number;
  rate60?: number;
  per_ora?: number[];
  previsto?: number;
  fox_vietato?: boolean;
  non_contest?: string[];
  ora_ms?: number;
  banda?: string;
  modo?: string;
  banda_ok?: boolean;
  modo_ok?: boolean;
}

export interface Valore {
  call: string;
  doppio: boolean;
  punti: number;
  km: number;
  molt_nuovi: string[];
  bande_utili: string[];
  fuori_regola: boolean;
}

export interface LogQso {
  n: number;
  time_ms: number;
  call: string;
  band: string;
  mode: string;
  freq_hz: number;
  grid: string;
  rst_sent: string;
  rst_rcvd: string;
  exch_sent: string;
  exch_rcvd: string;
  stato: 'valido' | 'incerto' | 'errore';
  modificato: boolean;
  doppio: boolean;
  punti: number;
  km: number;
  molt_nuovi: string[];
  scambio_ok: boolean;
  /** Perche' lo scambio non va (fuori lista, formato...); vuota se va bene. */
  scambio_nota?: string;
}

export interface Esportazione {
  testo: string;
  nome: string;
  problemi: string[];
  invio_url: string;
}

export interface Prossimo {
  id: string;
  nome: string;
  inizio_ms: number;
  fine_ms: number;
  da_verificare: boolean;
}

export const getContest = () => call<ContestStato>('');
export const getRegole = (id: string) => call<{ regole: Regole; impostazioni: Impostazioni }>(`/regole/${encodeURIComponent(id)}`);
export const attivaContest = (id: string) => call<{ ok: boolean; avvisi: string[] }>('/attiva', json('POST', { id }));
export const disattivaContest = () => call<{ ok: boolean; avvisi: string[] }>('/disattiva', { method: 'POST' });
export const salvaImpostazioni = (id: string, imp: Impostazioni) => call<{ ok: boolean }>(`/impostazioni/${encodeURIComponent(id)}`, json('PUT', imp));
export const valuta = (stazioni: { call: string; grid: string }[]) => call<{ banda: string; modo: string; valori: Valore[] }>('/valuta', json('POST', { stazioni }));
export const getLavorati = (banda: string) => call<{ banda: string; moltiplicatori: { tipo: string; valori: string[] }[] }>(`/lavorati?banda=${encodeURIComponent(banda)}`);
export const getContestLog = () => call<{ qso: LogQso[]; totali: Totali | null }>('/log');
export const modificaQso = (n: number, patch: { exch_rcvd?: string; grid?: string; stato?: LogQso['stato'] }) => call<{ ok: boolean }>(`/log/${n}`, json('PATCH', patch));
export const azzeraLog = () => call<{ ok: boolean }>('/log/azzera', { method: 'POST' });
export const esporta = (formato?: string, banda?: string) => {
  const q = new URLSearchParams();
  if (formato) q.set('formato', formato);
  if (banda) q.set('banda', banda);
  const s = q.toString();
  return call<Esportazione>(`/esporta${s ? `?${s}` : ''}`);
};
export const getCalendario = () => call<{ contest: Prossimo[] }>('/calendario');

/** Una banda nel consiglio di banda e modo. */
export interface BandaConsiglio {
  banda: string;
  sentiti: number;
  non_doppi: number;
  molt_nuovi: string[];
  qso_ora: number;
  rate_stimato: number;
  affollamento: number;
  modo: string;
  khz: number | null;
  punteggio: number;
  /** false: nessuna decodifica recente su questa banda, non si consiglia. */
  dati: boolean;
}

export interface Consiglio {
  banda: string;
  modo: string;
  bande: BandaConsiglio[];
  migliore: string | null;
  modo_migliore: string | null;
  /** Cambi di banda nell'ora UTC in corso (solo se il regolamento li limita). */
  cambi_banda_ora: number | null;
  cambi_banda_limite: number | null;
  ora_ms: number;
}

/** Una stazione in ascolto sulla banda della mappa. */
export interface SentitoMappa {
  call: string;
  grid: string;
  snr: number;
  t: number;
  cq: boolean;
  molt_nuovi: string[];
  doppio: boolean;
  punti: number;
}

export interface Mappa {
  banda: string;
  tipo: string;
  /** I moltiplicatori sono locatori (campi o quadrati): si disegna la mappa. */
  griglia: boolean;
  mio_grid: string;
  lavorati: string[];
  sentiti: SentitoMappa[];
}

export const getConsiglio = () => call<Consiglio>('/consiglio');
export const getMappa = (banda: string) => call<Mappa>(`/mappa?banda=${encodeURIComponent(banda)}`);
