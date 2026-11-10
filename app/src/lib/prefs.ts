import { create } from 'zustand';
import { browserStorage } from './theme';

/** Come si mostrano i messaggi (scheda Generali del motore): unita', paese, bandiera, distanza, DT. */
export interface Prefs {
  distanceUnit: 'km' | 'mi';
  showCountry: boolean;
  showFlag: boolean;
  showDistance: boolean;
  showDt: boolean;
  /** La colonna dell'ora mostra quanto e' vecchio il messaggio (45 s, 3 min) invece dell'ora UTC (richiesta di Luc). */
  showAge: boolean;
  /** Il messaggio piu' recente in fondo, (richiesta di Luc): Attivita' di banda e Finestra RX separate. */
  bandBottom: boolean;
  rxBottom: boolean;
  /** Con la Finestra RX aperta, i messaggi a me e della stazione del QSO solo li' e non anche in Attivita' di banda. Spento di fabbrica:
   *  si vedono da tutte e due le parti (Luc ON7KEC, 2026-11-10: il CQ del DX mancava da Attivita' di banda e sembrava un errore). */
  rxExclusive: boolean;
  /** Il doppio clic su un messaggio risponde a quella stazione (come nel motore); spento seleziona soltanto. */
  doubleClickReply: boolean;
  /** Il tasto Esc ferma la trasmissione e spegne AutoTX e Abilita Tx. */
  escStop: boolean;
  /** Al cambio di banda o di modo si svuota l'attivita' di banda: i messaggi dell'altra banda non servono piu'. */
  clearOnChange: boolean;
  /** Quanti QSO di Log4OM2 mostra la finestra Log (1-200). */
  log4omCount: number;
  /** Per quanti giorni si vedono ancora le DXpedition finite. */
  dxpedDays: number;
  /** «Riga vuota tra i periodi di decodifica» (`InsertBlank`): uno spazio tra i messaggi di un periodo e quelli del successivo. */
  insertBlank: boolean;
  /** «Visualizza i tuoi messaggi Tx nella finestra Rx» (`Tx2QSO`): le righe Tx si vedono nell'elenco. */
  showTx: boolean;
  /** «Mostra l'azimut nei messaggi con locatore» (`showAzimuth`): accanto alla distanza. */
  showAzimuth: boolean;
  /** Pulsante «Tune» nel pannello Comandi (spento di base). */
  cmdTune: boolean;
  /** Pulsante «Log QSO» nel pannello Comandi (spento di base). */
  cmdLogQso: boolean;
  /** Pagina QMAP nel menu (ricezione panoramica Q65 da un SDR): spenta di base, serve solo a chi ha un SDR con TCI. */
  showQmap: boolean;
  /** Indirizzo di chi aiuta, per «Manda per email» del pacchetto per l'assistenza (vuoto di fabbrica). */
  supportEmail: string;
}

const KEY = 'aurora-prefs';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultPrefs(): Prefs {
  return { distanceUnit: 'km', showCountry: true, showFlag: true, showDistance: true, showDt: true, showAge: false, bandBottom: false, rxBottom: false, rxExclusive: false, doubleClickReply: true, escStop: true, clearOnChange: true, log4omCount: 20, dxpedDays: 15, insertBlank: false, showTx: true, showAzimuth: false, cmdTune: false, cmdLogQso: false, showQmap: false, supportEmail: '' };
}

export function readPrefs(storage: ReadableStorage): Prefs {
  const prefs = defaultPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (parsed.distanceUnit === 'km' || parsed.distanceUnit === 'mi') prefs.distanceUnit = parsed.distanceUnit;
    for (const key of ['showCountry', 'showFlag', 'showDistance', 'showDt', 'showAge', 'bandBottom', 'rxBottom', 'rxExclusive', 'doubleClickReply', 'escStop', 'clearOnChange', 'insertBlank', 'showTx', 'showAzimuth', 'cmdTune', 'cmdLogQso', 'showQmap'] as const) {
      if (typeof parsed[key] === 'boolean') prefs[key] = parsed[key];
    }
    if (typeof parsed.dxpedDays === 'number' && Number.isFinite(parsed.dxpedDays)) prefs.dxpedDays = Math.min(90, Math.max(0, Math.round(parsed.dxpedDays)));
  if (typeof parsed.supportEmail === 'string') prefs.supportEmail = parsed.supportEmail.trim().slice(0, 200);
  if (typeof parsed.log4omCount === 'number' && Number.isFinite(parsed.log4omCount)) prefs.log4omCount = Math.min(200, Math.max(1, Math.round(parsed.log4omCount)));
  } catch {
    return defaultPrefs();
  }
  return prefs;
}

export function savePrefs(storage: WritableStorage, prefs: Prefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio le preferenze non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

export function formatDistance(km: number, unit: 'km' | 'mi'): string {
  return unit === 'mi' ? `${Math.round(km * 0.621371)} mi` : `${Math.round(km)} km`;
}

interface PrefsState {
  prefs: Prefs;
  set(change: Partial<Prefs>): void;
  reset(): void;
}

export const usePrefs = create<PrefsState>((set, get) => ({
  prefs: readPrefs(browserStorage()),
  set(change) {
    const next = { ...get().prefs, ...change };
    savePrefs(browserStorage(), next);
    set({ prefs: next });
  },
  reset() {
    savePrefs(browserStorage(), defaultPrefs());
    set({ prefs: defaultPrefs() });
  },
}));

// le preferenze cambiate nella finestra a parte delle Impostazioni (per esempio «Pagina QMAP nel menu») valgono subito anche qui
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key === KEY) usePrefs.setState({ prefs: readPrefs(browserStorage()) });
  });
}
