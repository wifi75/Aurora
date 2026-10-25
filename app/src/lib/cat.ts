import type { CatCandidate, CatConfig, CatModel } from '../api/client';

/** Le velocita' offerte (le stesse che accetta il core). */
export const BAUDS = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200] as const;

/** La configurazione di base, uguale a quella del core. */
export const DEFAULT_CAT: CatConfig = {
  backend: 'hamlib',
  model: 0,
  rig_name: '',
  port: '',
  baud: 0,
  data_bits: 0,
  stop_bits: 0,
  handshake: 'default',
  force_dtr: null,
  force_rts: null,
  ptt: 'cat',
  ptt_port: '',
  split: 'none',
  cat_mode: 'none',
  civ_address: 0,
  poll_secs: 2,
  auto_connect: false,
};

/** Una porta seriale (COM3, /dev/ttyUSB0) e non un indirizzo di rete (host:porta). */
export function isSerialPort(port: string): boolean {
  return /^com\d+$/i.test(port) || /^\/dev\//i.test(port);
}

const PORT = /^[A-Za-z0-9_\-.:/\\]{1,64}$/;

/**
 * I controlli che fa anche il core: il codice dell'errore (lo stesso del core) o `null` se i valori vanno.
 * Serve a non mandare cio' che verrebbe rifiutato e a dirlo subito.
 */
export function catIssue(c: CatConfig): string | null {
  if (c.rig_name.length > 80) return 'bad_rig_name';
  if (c.port !== '' && !PORT.test(c.port)) return 'bad_port';
  if (c.ptt_port !== '' && !PORT.test(c.ptt_port)) return 'bad_port';
  if (c.baud !== 0 && !(BAUDS as readonly number[]).includes(c.baud)) return 'bad_baud';
  if (![0, 7, 8].includes(c.data_bits) || ![0, 1, 2].includes(c.stop_bits)) return 'bad_serial';
  if (!Number.isInteger(c.poll_secs) || c.poll_secs < 1 || c.poll_secs > 99) return 'bad_poll';
  if ((c.ptt === 'dtr' || c.ptt === 'rts') && c.port === '' && c.ptt_port === '') return 'bad_ptt';
  if (!Number.isInteger(c.civ_address) || c.civ_address < 0 || c.civ_address > 255) return 'bad_civ';
  return null;
}

/** Quali campi mostrare, come in Decodium: dipendono dal backend, dal tipo di porta e dal metodo del PTT. */
export function visibleFields(c: CatConfig, model: CatModel | undefined) {
  const hamlib = c.backend === 'hamlib';
  const network = c.port !== '' && !isSerialPort(c.port);
  return {
    rig: hamlib,
    port: hamlib,
    baud: hamlib && !network,
    serial: hamlib && !network,
    civ: hamlib && ((model?.manufacturer ?? c.rig_name).toLowerCase().includes('icom') || c.civ_address !== 0),
    pttPort: hamlib && (c.ptt === 'dtr' || c.ptt === 'rts'),
    ptt: hamlib,
    split: true,
    catMode: true,
    poll: hamlib,
  };
}

/** Il nome mostrato di un modello: «Yaesu FT-2000». */
export function modelLabel(m: CatModel): string {
  return `${m.manufacturer} ${m.model}`;
}

const normal = (s: string) => s.toLowerCase().replace(/[^a-z0-9 ]/g, '');

/** Ricerca per parole (tutte devono comparire, in qualunque ordine, ignorando trattini e maiuscole). Senza testo tutti i modelli. */
export function filterModels(models: readonly CatModel[], query: string): CatModel[] {
  const words = query.toLowerCase().split(/\s+/).map((w) => w.replace(/[^a-z0-9]/g, '')).filter((w) => w !== '');
  if (words.length === 0) return [...models];
  return models.filter((m) => {
    const text = normal(modelLabel(m)).replace(/ /g, '');
    return words.every((w) => text.includes(w));
  });
}

/** Il modello che corrisponde a una radio rilevata: il numero se il core lo sa, altrimenti il testo cercato nell'elenco (nome esatto prima). */
export function modelForCandidate(models: readonly CatModel[], c: CatCandidate): CatModel | undefined {
  if (c.rig_model !== null) {
    const byId = models.find((m) => m.id === c.rig_model);
    if (byId) return byId;
  }
  const token = c.rig_token.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (token === '') return undefined;
  const hits = models.filter((m) => m.model.toLowerCase().replace(/[^a-z0-9]/g, '').includes(token));
  return hits.find((m) => m.model.toLowerCase().replace(/[^a-z0-9]/g, '') === token) ?? [...hits].sort((a, b) => a.model.length - b.model.length)[0];
}

/** Porta una radio rilevata nella configurazione in modifica (non salva nulla: resta una bozza finche' non si applica). */
export function applyCandidate(c: CatConfig, cand: CatCandidate, model: CatModel | undefined): CatConfig {
  return {
    ...c,
    backend: 'hamlib',
    model: model?.id ?? c.model,
    rig_name: model ? modelLabel(model) : c.rig_name,
    port: cand.cat_port || c.port,
    baud: cand.baud ?? c.baud,
    civ_address: cand.civ_address ?? c.civ_address,
  };
}

/** I valori di due configurazioni sono uguali (per sapere se ci sono modifiche non applicate). */
export function sameCat(a: CatConfig, b: CatConfig): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
