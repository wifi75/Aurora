import { create } from 'zustand';
import type { AuroraEvent, DecodedMessage, StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';

const MAX_DECODES = 200;
const MAX_TX_LOG = 50;
/** Quanto resta la stazione scelta col doppio clic se JTDX non la conferma. */
const PENDING_MS = 20_000;

interface StationState {
  status: StationStatus | null;
  /** I piu' recenti per primi. */
  decodes: DecodedMessage[];
  /** I nostri messaggi trasmessi, piu' recenti per primi, con id negativi (non sono decodifiche e non si rispondono). */
  txLog: DecodedMessage[];
  /** Stazione scelta con il doppio clic, mostrata come DX finche' JTDX non riporta la sua. */
  pendingDx: string | null;
  /** Quando e' stata scelta (ms Unix): dopo un po' si smette di aspettare JTDX. */
  pendingSince: number;
  connection: ConnectionState;
  apply(event: AuroraEvent): void;
  /** Mostra subito un cambio comandato (banda, modo) senza aspettare il prossimo stato di JTDX. */
  patchStatus(patch: Partial<StationStatus>): void;
  setPendingDx(call: string | null): void;
  setConnection(state: ConnectionState): void;
  reset(): void;
}

let nextTxId = -1;

/** La stazione scelta resta finche' JTDX non riporta la stessa (o scade): uno stato di JTDX con la stazione di prima non la cancella. */
function settledPending(s: { pendingDx: string | null; pendingSince: number }, dxCall: string | null | undefined): { pendingDx: null } | Record<string, never> {
  if (!s.pendingDx) return {};
  const confirmed = (dxCall ?? '').toUpperCase() === s.pendingDx.toUpperCase();
  const expired = Date.now() - s.pendingSince > PENDING_MS;
  return confirmed || expired ? { pendingDx: null } : {};
}

export const useStation = create<StationState>((set) => ({
  status: null,
  decodes: [],
  txLog: [],
  pendingDx: null,
  pendingSince: 0,
  connection: 'connecting',
  apply(event) {
    switch (event.type) {
      case 'status': {
        const { type: _type, ...status } = event;
        set((s) => {
          const message = status.tx_message?.trim() ?? '';
          const starts = status.transmitting && message !== '' && (!s.status?.transmitting || s.status.tx_message !== status.tx_message);
          if (!starts) return { status, ...settledPending(s, status.dx_call) };
          const row: DecodedMessage = {
            id: nextTxId--,
            time_ms: Date.now() % 86_400_000,
            snr: 0,
            delta_time: 0,
            freq_hz: status.tx_df ?? 0,
            text: message,
            tag: null,
          };
          return { status, txLog: [row, ...s.txLog].slice(0, MAX_TX_LOG), ...settledPending(s, status.dx_call) };
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
    set((s) => (s.status ? { status: { ...s.status, ...patch } } : s));
  },
  setPendingDx(call) {
    set({ pendingDx: call, pendingSince: Date.now() });
  },
  setConnection(connection) {
    set({ connection });
  },
  reset() {
    set({ status: null, decodes: [], txLog: [], pendingDx: null, pendingSince: 0, connection: 'connecting' });
  },
}));
