import { useEffect } from 'react';
import { postControl, postHalt } from '../api/client';
import { usePrefs } from './prefs';

/**
 * Esc ferma la trasmissione e spegne AutoTX e Abilita Tx (spegnere e' sempre permesso). Le tre chiamate partono
 * insieme e un errore dell'una non ferma le altre: e' il tasto d'emergenza. Si accende e spegne in Impostazioni.
 */
export function useEscapeStop(): void {
  const enabled = usePrefs((s) => s.prefs.escStop);
  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      void postHalt().catch(() => undefined);
      void postControl('auto_tx', false).catch(() => undefined);
      void postControl('enable_tx', false).catch(() => undefined);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [enabled]);
}
