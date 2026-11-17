import { create } from 'zustand';

/** Vero finche' la guida del primo avvio e' aperta: gli avvisi della stazione aspettano (non si avvisa di un nominativo che si sta scrivendo). */
export const useFirstRunOpen = create<{ open: boolean }>(() => ({ open: false }));
