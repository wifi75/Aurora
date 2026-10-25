import { useEffect } from 'react';
import { postControl, postHalt } from '../api/client';
import { useStation } from '../state/station';
import { usePrefs } from './prefs';

/** Due Esc entro questo tempo contano come un doppio Esc. */
const DOUBLE_MS = 700;

/**
 * Esc ferma la trasmissione e spegne Abilita Tx (spegnere e' sempre permesso), come in JTDX: `keyPressEvent` chiama `haltTx`, che spegne solo
 * Abilita Tx e non tocca AutoTX (il «doppio clic accende Abilita Tx») ne' AutoSeq (verificato nell'eseguibile, `REA 0x4bd4e0` e `0x4ac620`).
 * Le due chiamate partono insieme e un errore dell'una non ferma l'altra: e' il tasto d'emergenza. Si accende e spegne in Impostazioni.
 * Con un doppio Esc si libera anche la stazione da chiamare: come "Canc DX" del motore, che pulisce nominativo e griglia
 * DX (anche quelli arrivati con un Reply via UDP), e Aurora dimentica la stazione scelta con il doppio clic.
 */
export function useEscapeStop(): void {
  const enabled = usePrefs((s) => s.prefs.escStop);
  useEffect(() => {
    if (!enabled) return;
    let lastEsc = 0;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      const now = Date.now();
      const doubled = now - lastEsc <= DOUBLE_MS;
      lastEsc = doubled ? 0 : now;
      if (doubled) {
        void postControl('clear_dx').catch(() => undefined);
        useStation.getState().setPendingDx(null);
      }
      // la pagina mostra subito che la trasmissione e' ferma e Abilita Tx spento, senza aspettare lo stato del motore (arriva dopo un attimo)
      useStation.getState().patchStatus({ transmitting: false, tx_enabled: false });
      void postHalt().catch(() => undefined);
      void postControl('enable_tx', false).catch(() => undefined);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [enabled]);
}
