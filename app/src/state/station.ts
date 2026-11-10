import { useFox } from './fox';
import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type { AudioLevel, AuroraEvent, DecodedMessage, StationStatus, UpdateNotice, WantedAlert } from '../api/schemas';

/** I colori chiesti da un programma collegato via UDP per un nominativo (Highlight Callsign). */
export interface CallHighlight {
  background: string | null;
  foreground: string | null;
  lastOnly: boolean;
}

/** Quante evidenziazioni si tengono al massimo. */
const MAX_HIGHLIGHTS = 500;
/** Solo colori `#rrggbb`: niente altro finisce nello stile della riga. */
const COLOR = /^#[0-9a-f]{6}$/i;
const colorOrNull = (c: string | null): string | null => (c !== null && COLOR.test(c) ? c : null);
import type { ConnectionState } from '../api/stream';
import { bandOf } from '../lib/format';
import { usePrefs } from '../lib/prefs';

const MAX_DECODES = 200;
/** Decodifiche arrivate e non ancora aggiunte alla lista (vedi il caso «decode»). */
let pendingDecodes: DecodedMessage[] = [];
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const MAX_TX_LOG = 50;
const MAX_WANTED = 20;
/** Quanto resta la stazione scelta col doppio clic se il motore non la conferma. */
const PENDING_MS = 20_000;
/** Quanto vale un cambio comandato (banda, modo) prima che il motore lo confermi: oltre, vince lo stato del motore. */
const PATCH_MS = 8_000;
/** Dopo un cambio di banda uno stato in ritardo con il vecchio chiamante si ignora per questo tempo. */
const STALE_DX_MS = 3_000;
/** Cio' che mostra la Stazione DX quando e' azzerata. */
const DX_CLEARED = { dx_call: null, dx_grid: null, report: null } as const;

interface StationState {
  status: StationStatus | null;
  /** I piu' recenti per primi. */
  decodes: DecodedMessage[];
  /** I nostri messaggi trasmessi, piu' recenti per primi, con id negativi (non sono decodifiche e non si rispondono). */
  txLog: DecodedMessage[];
  /** Stazione scelta con il doppio clic, mostrata come DX finche' il motore non riporta la sua. */
  pendingDx: string | null;
  /** Quando e' stata scelta (ms Unix): dopo un po' si smette di aspettare il motore. */
  pendingSince: number;
  connection: ConnectionState;
  /** Gli ultimi avvisi della lista dei desiderati, i piu' recenti per primi. */
  wanted: WantedAlert[];
  /** L'avviso mostrato nella barra in alto, finche' non si chiude. */
  wantedBanner: WantedAlert | null;
  dismissWanted(): void;
  /** L'ultima nuova versione comunicata dal core, mostrata subito senza aspettare il controllo periodico della pagina. */
  updateNotice: UpdateNotice | null;
  /** Cambi comandati (banda, modo) che il motore non ha ancora confermato: finche' non coincidono, hanno la precedenza. */
  pending: Partial<StationStatus>;
  pendingAt: number;
  /** Banda appena scelta con un clic, evidenziata subito anche se la frequenza non e' ancora cambiata. */
  pendingBand: string | null;
  /** L'ultima misura del livello dell'ingresso audio e quando e' arrivata (ms Unix); senza misure recenti l'indicatore si nasconde. */
  audioLevel: AudioLevel | null;
  audioLevelAt: number;
  /** Quando e' arrivato l'ultimo messaggio decodificato e il primo stato del motore (ms Unix): servono alla diagnosi. */
  lastDecodeAt: number | null;
  sessionStartAt: number | null;
  /** Gli avvisi arrivati dal motore (audio, radio, OmniRig): id → dettaglio e quando. */
  notices: Record<string, { detail: string; at: number }>;
  /** La banda per cui si e' gia' svuotato tutto (stato o evento del motore): il secondo avviso per lo stesso cambio non svuota di nuovo. */
  clearedBand: string | null;
  /** Il chiamante appena azzerato: per un attimo lo stato in arrivo che lo riporta ancora (ritardo del motore) non lo fa ricomparire. */
  clearedDx: { call: string; until: number } | null;
  /** Quanti QSO sono stati registrati da quando la pagina e' aperta: chi mostra dati del log (scheda DXCC) li rilegge quando cambia. */
  qsosLogged: number;
  /** I QSO appena messi a log (dall'evento), per mostrarli subito nel pannello Log prima che Log4OM li scriva. */
  justLogged: { call: string; band: string; mode: string; rst_sent: string; rst_rcvd: string; time_ms: number; at: number }[];
  /** I nominativi colorati dai programmi collegati via UDP (Highlight Callsign), per nominativo in maiuscolo. */
  highlights: Record<string, CallHighlight>;
  apply(event: AuroraEvent): void;
  /** Mostra subito un cambio comandato (banda, modo) e lo tiene finche' il motore non lo riporta uguale (o scade). */
  patchStatus(patch: Partial<StationStatus>): void;
  /** Annulla un cambio comandato che non e' andato a buon fine, rimettendo il valore di prima. */
  revertStatus(keys: Array<keyof StationStatus>, previous: Partial<StationStatus>): void;
  setPendingBand(band: string | null): void;
  setPendingDx(call: string | null): void;
  setConnection(state: ConnectionState): void;
  reset(): void;
}

let nextTxId = -1;

/** La stazione scelta resta finche' il motore non riporta la stessa (o scade): uno stato del motore con la stazione di prima non la cancella. */
function settledPending(s: { pendingDx: string | null; pendingSince: number }, dxCall: string | null | undefined): { pendingDx: null } | Record<string, never> {
  if (!s.pendingDx) return {};
  const confirmed = (dxCall ?? '').toUpperCase() === s.pendingDx.toUpperCase();
  const expired = Date.now() - s.pendingSince > PENDING_MS;
  return confirmed || expired ? { pendingDx: null } : {};
}

/** Lo stato del motore con sopra i cambi comandati ancora non confermati; quelli confermati o scaduti si scartano. */
function withPending(s: { pending: Partial<StationStatus>; pendingAt: number }, status: StationStatus): { status: StationStatus; pending: Partial<StationStatus> } {
  const keys = Object.keys(s.pending) as Array<keyof StationStatus>;
  if (keys.length === 0 || Date.now() - s.pendingAt > PATCH_MS) return { status, pending: {} };
  const merged = { ...status } as Record<string, unknown>;
  const pending: Record<string, unknown> = { ...s.pending };
  for (const key of keys) {
    if (status[key] === s.pending[key]) delete pending[key];
    else merged[key] = s.pending[key];
  }
  return { status: merged as unknown as StationStatus, pending: pending as Partial<StationStatus> };
}

function bandOfStatus(s: StationStatus | null | undefined): string | null {
  return s && s.dial_hz > 0 ? bandOf(s.dial_hz) : null;
}

/** La banda e' diversa da prima (e prima era nota): al cambio di banda si svuota sempre tutto, come JTDX. */
/** Aggiunge subito alla lista le decodifiche ancora in attesa (le prove non aspettano i 60 ms della raccolta). */
export function flushDecodes(): void {
  if (flushTimer !== null) clearTimeout(flushTimer);
  flushTimer = null;
  const batch = pendingDecodes;
  pendingDecodes = [];
  if (batch.length > 0) useStation.setState((s) => ({ decodes: [...batch, ...s.decodes].slice(0, MAX_DECODES), lastDecodeAt: Date.now() }));
}

export function bandChanged(before: StationStatus | null | undefined, after: StationStatus | null | undefined): boolean {
  const b = bandOfStatus(before);
  const a = bandOfStatus(after);
  return b !== null && a !== null && b !== a;
}

/** Banda o modo diversi da prima (e prima noti). */
export function bandOrModeChanged(before: StationStatus | null | undefined, after: StationStatus | null | undefined): boolean {
  if (!before || !after) return false;
  const modeChanged = before.mode !== '' && after.mode !== '' && before.mode !== after.mode;
  return bandChanged(before, after) || modeChanged;
}

type Cleared = { decodes: DecodedMessage[]; txLog: DecodedMessage[] } & Partial<Pick<StationState, 'pendingDx' | 'pendingSince' | 'clearedBand' | 'clearedDx'>>;

/** Il cambio di banda svuota attivita' di banda, CQ e novita' (le stesse decodifiche), il registro dei nostri messaggi e la Stazione DX;
 *  lo storico dei 30 giorni non si tocca. Il cambio di solo modo svuota le liste solo con l'opzione «svuota al cambio». */
function clearedOnChange(s: { clearedBand: string | null }, before: StationStatus | null | undefined, after: StationStatus | null | undefined): Cleared | Record<string, never> {
  if (bandChanged(before, after) && s.clearedBand !== bandOfStatus(after)) {
    const call = before?.dx_call?.toUpperCase() ?? '';
    return { ...clearedForBand(bandOfStatus(after)), clearedDx: call ? { call, until: Date.now() + STALE_DX_MS } : null };
  }
  // cambio di frequenza nella stessa banda (per esempio 14.074 -> 14.090): i messaggi della frequenza di prima non servono piu', si svuota sempre
  // (un cambio di banda gia' svuotato, per esempio dall'avviso band_changed del motore, non svuota di nuovo: lo stato che lo conferma
  // non deve cancellare i primi messaggi della banda nuova)
  if (dialChanged(before, after) && !bandChanged(before, after)) {
    dropPending();
    return { decodes: [], txLog: [] };
  }
  return usePrefs.getState().prefs.clearOnChange && bandOrModeChanged(before, after) && !bandChanged(before, after) ? { decodes: [], txLog: [] } : {};
}

/** La frequenza della radio e' cambiata di almeno 1 kHz (non i piccoli ritocchi del RIT o della lettura). */
export function dialChanged(before: StationStatus | null | undefined, after: StationStatus | null | undefined): boolean {
  const b = before?.dial_hz ?? 0;
  const a = after?.dial_hz ?? 0;
  return b > 0 && a > 0 && Math.abs(a - b) >= 1_000;
}

function dropPending(): void {
  pendingDecodes = [];
}

function clearedForBand(band: string | null): Cleared {
  dropPending();
  return { decodes: [], txLog: [], pendingDx: null, pendingSince: 0, clearedBand: band };
}

/** Lo stato con la Stazione DX azzerata (nominativo, locatore, rapporto). */
function withoutDx(status: StationStatus): StationStatus {
  return { ...status, ...DX_CLEARED };
}

export const useStation = create<StationState>((set) => ({
  status: null,
  decodes: [],
  txLog: [],
  pendingDx: null,
  pendingSince: 0,
  connection: 'connecting',
  pending: {},
  pendingAt: 0,
  pendingBand: null,
  wanted: [],
  wantedBanner: null,
  updateNotice: null,
  audioLevel: null,
  audioLevelAt: 0,
  lastDecodeAt: null,
  sessionStartAt: null,
  notices: {},
  clearedBand: null,
  clearedDx: null,
  qsosLogged: 0,
  justLogged: [],
  highlights: {},
  dismissWanted() {
    set({ wantedBanner: null });
  },
  apply(event) {
    switch (event.type) {
      case 'status': {
        const { type: _type, ...reported } = event;
        set((s) => {
          const { status: shown, pending } = withPending(s, reported);
          const cleared = clearedOnChange(s, s.status, shown);
          const band = bandChanged(s.status, shown) && 'clearedBand' in cleared;
          const stale = s.clearedDx !== null && Date.now() < s.clearedDx.until && (shown.dx_call ?? '').toUpperCase() === s.clearedDx.call;
          // il motore azzera il DX da solo al cambio di banda: se il suo stato porta ancora quello di prima, e' in ritardo
          const status = band || stale ? withoutDx(shown) : shown;
          // la banda scelta resta evidenziata finche' lo stato del motore non e' su quella banda
          const keep = { pending, pendingBand: s.pendingBand !== null && bandOf(status.dial_hz) === s.pendingBand ? null : s.pendingBand };
          const message = status.tx_message?.trim() ?? '';
          const starts = status.transmitting && message !== '' && (!s.status?.transmitting || s.status.tx_message !== status.tx_message);
          const session = { sessionStartAt: s.sessionStartAt ?? Date.now() };
          if (!starts) return { status, ...session, ...cleared, ...keep, ...settledPending(s, status.dx_call) };
          const row: DecodedMessage = {
            id: nextTxId--,
            time_ms: Date.now() % 86_400_000,
            snr: 0,
            delta_time: 0,
            freq_hz: status.tx_df ?? 0,
            text: message,
            tag: null,
          };
          return { status, ...session, txLog: [row, ...s.txLog].slice(0, MAX_TX_LOG), ...keep, ...settledPending(s, status.dx_call), ...(Object.keys(cleared).length > 0 ? { decodes: [], txLog: [row] } : {}) };
        });
        break;
      }
      case 'band_changed': {
        // la banda e' cambiata (avviso del motore, anche dalla radio o da uno spot): se lo stato non l'ha gia' fatto, si svuota tutto
        set((s) => {
          if (event.band !== null && s.clearedBand === event.band) return s;
          const call = s.status?.dx_call?.toUpperCase() ?? '';
          return { ...clearedForBand(event.band), clearedDx: call ? { call, until: Date.now() + STALE_DX_MS } : null, ...(s.status ? { status: withoutDx(s.status) } : {}) };
        });
        break;
      }
      case 'decode': {
        const { type: _type, ...decode } = event;
        // le decodifiche di un periodo arrivano a raffica (anche 40 in pochi millisecondi): si raccolgono e si aggiungono tutte insieme,
        // cosi' la lista si ridisegna una volta sola invece che una volta per riga
        pendingDecodes.unshift(decode);
        if (flushTimer === null) {
          flushTimer = setTimeout(() => {
            flushTimer = null;
            const batch = pendingDecodes;
            pendingDecodes = [];
            set((s) => ({ decodes: [...batch, ...s.decodes].slice(0, MAX_DECODES), lastDecodeAt: Date.now() }));
          }, 60);
        }
        break;
      }
      case 'wanted': {
        const { type: _type, ...alert } = event;
        set((s) => ({ wanted: [alert, ...s.wanted].slice(0, MAX_WANTED), wantedBanner: alert }));
        break;
      }
      case 'update_available': {
        const { type: _type, ...notice } = event;
        set({ updateNotice: notice });
        break;
      }
      case 'audio_level': {
        const { type: _type, ...level } = event;
        set({ audioLevel: level, audioLevelAt: Date.now() });
        break;
      }
      case 'notice':
        set((s) => ({ notices: { ...s.notices, [event.id]: { detail: event.detail, at: Date.now() } } }));
        break;
      case 'tx_halted':
        set((s) => (s.status ? { status: { ...s.status, transmitting: false } } : s));
        break;
      case 'qso_logged':
        set((s) => ({
          qsosLogged: s.qsosLogged + 1,
          justLogged: [{ call: event.call, band: event.band, mode: event.mode, rst_sent: event.rst_sent, rst_rcvd: event.rst_rcvd, time_ms: event.time_ms, at: Date.now() }, ...s.justLogged].slice(0, 10),
        }));
        break;
      case 'highlight_callsign': {
        const call = event.call.trim().toUpperCase();
        if (call === '') break;
        const background = colorOrNull(event.background);
        const foreground = colorOrNull(event.foreground);
        set((s) => {
          const next = { ...s.highlights };
          delete next[call];
          // due colori non validi tolgono l'evidenziazione, come nel protocollo
          if (background !== null || foreground !== null) next[call] = { background, foreground, lastOnly: event.last_only };
          const keys = Object.keys(next);
          for (const old of keys.slice(0, Math.max(0, keys.length - MAX_HIGHLIGHTS))) delete next[old];
          return { highlights: next };
        });
        break;
      }
      case 'fox':
        useFox.setState({ fox: event });
        break;
      case 'clear_decodes':
        // come «Cancella»: le liste dei messaggi si svuotano (i colori dei nominativi restano)
        pendingDecodes = [];
        set({ decodes: [], txLog: [] });
        break;
    }
  },
  patchStatus(patch) {
    set((s) => {
      if (!s.status) return s;
      const moved = { ...s.status, ...patch };
      const cleared = clearedOnChange(s, s.status, moved);
      const status = bandChanged(s.status, moved) && 'clearedBand' in cleared ? withoutDx(moved) : moved;
      return { status, ...cleared, pending: { ...s.pending, ...patch }, pendingAt: Date.now() };
    });
  },
  revertStatus(keys, previous) {
    set((s) => {
      if (!s.status) return s;
      const pending = { ...s.pending } as Record<string, unknown>;
      for (const key of keys) delete pending[key];
      return { status: { ...s.status, ...previous }, pending: pending as Partial<StationStatus>, pendingBand: null, clearedBand: null };
    });
  },
  setPendingBand(band) {
    set({ pendingBand: band });
    if (band) {
      // se la radio non risponde la banda evidenziata torna quella vera
      setTimeout(() => useStation.setState((st) => (st.pendingBand === band ? { pendingBand: null } : st)), PATCH_MS);
    }
  },
  setPendingDx(call) {
    set({ pendingDx: call, pendingSince: Date.now() });
  },
  setConnection(connection) {
    set({ connection });
  },
  reset() {
    set({ status: null, decodes: [], txLog: [], pendingDx: null, pendingSince: 0, connection: 'connecting', pending: {}, pendingAt: 0, pendingBand: null, wanted: [], wantedBanner: null, updateNotice: null, audioLevel: null, audioLevelAt: 0, lastDecodeAt: null, sessionStartAt: null, clearedBand: null, clearedDx: null, highlights: {} });
  },
}));

/**
 * Solo i campi dello stato che servono a un componente: lo stato arriva piu' volte al secondo (S-meter, livelli) e chi lo legge intero si
 * ridisegnava a ogni arrivo anche se gli serviva solo la banda. Con questa scelta si ridisegna solo quando cambia uno dei suoi campi.
 */
export function useStatusPick<K extends keyof StationStatus>(keys: readonly K[]): Pick<StationStatus, K> | null {
  return useStation(
    useShallow((s) => {
      if (!s.status) return null as unknown as Pick<StationStatus, K>;
      const out = {} as Pick<StationStatus, K>;
      for (const k of keys) out[k] = s.status[k];
      return out;
    }),
  ) as Pick<StationStatus, K> | null;
}
