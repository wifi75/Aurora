import { create } from 'zustand';
import type { AuroraEvent, DecodedMessage, StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { bandOf } from '../lib/format';
import { usePrefs } from '../lib/prefs';

const MAX_DECODES = 200;
const MAX_TX_LOG = 50;
/** Quanto resta la stazione scelta col doppio clic se il motore non la conferma. */
const PENDING_MS = 20_000;
/** Quanto vale un cambio comandato (banda, modo) prima che il motore lo confermi: oltre, vince lo stato del motore. */
const PATCH_MS = 8_000;

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
  /** Cambi comandati (banda, modo) che il motore non ha ancora confermato: finche' non coincidono, hanno la precedenza. */
  pending: Partial<StationStatus>;
  pendingAt: number;
  /** Banda appena scelta con un clic, evidenziata subito anche se la frequenza non e' ancora cambiata. */
  pendingBand: string | null;
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

/** Banda o modo diversi da prima (e prima noti): si svuota l'attivita' di banda, se l'opzione e' accesa. */
export function bandOrModeChanged(before: StationStatus | null | undefined, after: StationStatus | null | undefined): boolean {
  if (!before || !after) return false;
  const band = (s: StationStatus) => (s.dial_hz > 0 ? bandOf(s.dial_hz) : null);
  const bandChanged = band(before) !== null && band(after) !== null && band(before) !== band(after);
  const modeChanged = before.mode !== '' && after.mode !== '' && before.mode !== after.mode;
  return bandChanged || modeChanged;
}

function clearedOnChange(before: StationStatus | null | undefined, after: StationStatus | null | undefined): { decodes: DecodedMessage[]; txLog: DecodedMessage[] } | Record<string, never> {
  return usePrefs.getState().prefs.clearOnChange && bandOrModeChanged(before, after) ? { decodes: [], txLog: [] } : {};
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
  apply(event) {
    switch (event.type) {
      case 'status': {
        const { type: _type, ...reported } = event;
        set((s) => {
          const { status, pending } = withPending(s, reported);
          // la banda scelta resta evidenziata finche' lo stato del motore non e' su quella banda
          const keep = { pending, pendingBand: s.pendingBand !== null && bandOf(status.dial_hz) === s.pendingBand ? null : s.pendingBand };
          const message = status.tx_message?.trim() ?? '';
          const starts = status.transmitting && message !== '' && (!s.status?.transmitting || s.status.tx_message !== status.tx_message);
          const cleared = clearedOnChange(s.status, status);
          if (!starts) return { status, ...cleared, ...keep, ...settledPending(s, status.dx_call) };
          const row: DecodedMessage = {
            id: nextTxId--,
            time_ms: Date.now() % 86_400_000,
            snr: 0,
            delta_time: 0,
            freq_hz: status.tx_df ?? 0,
            text: message,
            tag: null,
          };
          return { status, txLog: [row, ...s.txLog].slice(0, MAX_TX_LOG), ...keep, ...settledPending(s, status.dx_call), ...(Object.keys(cleared).length > 0 ? { decodes: [], txLog: [row] } : {}) };
        });
        break;
      }
      case 'decode': {
        const { type: _type, ...decode } = event;
        set((s) => ({ decodes: [decode, ...s.decodes].slice(0, MAX_DECODES) }));
        break;
      }
      case 'tx_halted':
        set((s) => (s.status ? { status: { ...s.status, transmitting: false } } : s));
        break;
      case 'qso_logged':
        break;
    }
  },
  patchStatus(patch) {
    set((s) => {
      if (!s.status) return s;
      const status = { ...s.status, ...patch };
      return { status, ...clearedOnChange(s.status, status), pending: { ...s.pending, ...patch }, pendingAt: Date.now() };
    });
  },
  revertStatus(keys, previous) {
    set((s) => {
      if (!s.status) return s;
      const pending = { ...s.pending } as Record<string, unknown>;
      for (const key of keys) delete pending[key];
      return { status: { ...s.status, ...previous }, pending: pending as Partial<StationStatus>, pendingBand: null };
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
    set({ status: null, decodes: [], txLog: [], pendingDx: null, pendingSince: 0, connection: 'connecting', pending: {}, pendingAt: 0, pendingBand: null });
  },
}));
