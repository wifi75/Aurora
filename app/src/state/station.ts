import { create } from 'zustand';
import type { AuroraEvent, DecodedMessage, StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';

const MAX_DECODES = 200;
const MAX_TX_LOG = 50;

interface StationState {
  status: StationStatus | null;
  /** I piu' recenti per primi. */
  decodes: DecodedMessage[];
  /** I nostri messaggi trasmessi, piu' recenti per primi, con id negativi (non sono decodifiche e non si rispondono). */
  txLog: DecodedMessage[];
  connection: ConnectionState;
  apply(event: AuroraEvent): void;
  setConnection(state: ConnectionState): void;
  reset(): void;
}

let nextTxId = -1;

export const useStation = create<StationState>((set) => ({
  status: null,
  decodes: [],
  txLog: [],
  connection: 'connecting',
  apply(event) {
    switch (event.type) {
      case 'status': {
        const { type: _type, ...status } = event;
        set((s) => {
          const message = status.tx_message?.trim() ?? '';
          const starts = status.transmitting && message !== '' && (!s.status?.transmitting || s.status.tx_message !== status.tx_message);
          if (!starts) return { status };
          const row: DecodedMessage = {
            id: nextTxId--,
            time_ms: Date.now() % 86_400_000,
            snr: 0,
            delta_time: 0,
            freq_hz: status.tx_df ?? 0,
            text: message,
            tag: null,
          };
          return { status, txLog: [row, ...s.txLog].slice(0, MAX_TX_LOG) };
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
  setConnection(connection) {
    set({ connection });
  },
  reset() {
    set({ status: null, decodes: [], txLog: [], connection: 'connecting' });
  },
}));
