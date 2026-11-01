import { create } from 'zustand';
import type { FoxSnapshot } from '../api/schemas';

/** L'ultimo stato della Fox arrivato dal motore (evento «fox»). */
export const useFox = create<{ fox: FoxSnapshot | null }>(() => ({ fox: null }));
