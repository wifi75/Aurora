import { create } from 'zustand';
import type { AuroraEvent, DecodedMessage, StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';

const MAX_DECODES = 200;

interface StationState {
  status: StationStatus | null;
  /** I piu' recenti per primi. */
  decodes: DecodedMessage[];
  connection: ConnectionState;
  apply(event: AuroraEvent): void;
  setConnection(state: ConnectionState): void;
  reset(): void;
}

export const useStation = create<StationState>((set) => ({
  status: null,
  decodes: [],
  connection: 'connecting',
  apply(event) {
    switch (event.type) {
      case 'status': {
        const { type: _type, ...status } = event;
        set({ status });
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
    set({ status: null, decodes: [], connection: 'connecting' });
  },
}));
