import { useEffect, useRef } from 'react';

/** Il comando dell'ascolto di un servizio (modi testuali, SSTV). */
export type RxCommand = (action: 'start' | 'stop') => Promise<void>;

/** Il conteggio delle pagine aperte di un servizio: battito (`hold`) e uscita (`release`) della pagina `id`. */
export type PageCommand = (action: 'hold' | 'release', id: string) => Promise<void>;

/** Ogni quanto la pagina dice al core che e' ancora aperta (il core la considera chiusa dopo 90 s senza battiti). */
export const HOLD_EVERY_MS = 20_000;

function newPageId(): string {
  try {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  } catch {
    // contesto non sicuro (telefono su http): si usa il numero a caso
  }
  return `p${Math.random().toString(36).slice(2)}${Date.now().toString(36)}`;
}

/**
 * L'ascolto di una pagina (PSK·RTTY·CW, SSTV): entrando si accende, uscendo si spegne, salvo «Ascolto acceso all'avvio di Aurora».
 *
 * Con `page` la pagina si fa contare dal core: un battito all'apertura e ogni 20 s, l'uscita allo smontaggio e alla chiusura della
 * finestra. Il core spegne l'ascolto solo quando non resta nessuna pagina aperta (finestra principale, finestra staccata, telefono) e
 * decide lui per «Ascolto acceso all'avvio»; una finestra chiusa senza salutare esce da sola dopo 90 s. Senza `page` uscendo si spegne
 * subito (se `keepOnLeave` non dice di tenerlo).
 *
 * Entrando si accende sempre, senza guardare lo stato in memoria: la pagina parte dai dati della visita di prima (il servizio
 * allora ascoltava), mentre uscendo l'ascolto e' stato spento. Fidandosi di quei dati vecchi l'ascolto restava spento e il CW non
 * decodificava finche' non si usciva e rientrava una seconda volta (issue #45). Accendere un ascolto gia' acceso non cambia niente.
 */
export function useListenWhileOpen(rx: RxCommand, keepOnLeave: () => boolean, onChanged: () => void, page?: PageCommand): void {
  const live = useRef({ rx, keepOnLeave, onChanged, page });
  live.current = { rx, keepOnLeave, onChanged, page };
  useEffect(() => {
    const id = newPageId();
    const done = () => live.current.onChanged();
    const hold = () => void live.current.page?.('hold', id).catch(() => undefined);
    let left = false;
    const leave = () => {
      if (left) return;
      left = true;
      void live.current.page?.('release', id).catch(() => undefined);
    };
    void live.current.rx('start').then(done, done);
    hold();
    const timer = live.current.page ? setInterval(hold, HOLD_EVERY_MS) : undefined;
    // la finestra (staccata o del telefono) si chiude: lo smontaggio di React non arriva, si saluta da qui
    if (live.current.page) window.addEventListener('pagehide', leave);
    return () => {
      if (timer !== undefined) clearInterval(timer);
      window.removeEventListener('pagehide', leave);
      if (live.current.page) leave();
      else if (!live.current.keepOnLeave()) void live.current.rx('stop').catch(() => undefined);
    };
  }, []);
}

/**
 * I comandi dell'ascolto vanno al servizio uno dopo l'altro: uscendo e rientrando subito nella pagina lo spegnimento e
 * l'accensione partono quasi insieme e, su due connessioni, potevano arrivare scambiati (ascolto spento con la pagina aperta).
 */
export function inOrder(): <T>(run: () => Promise<T>) => Promise<T> {
  let last: Promise<unknown> = Promise.resolve();
  return <T>(run: () => Promise<T>) => {
    const next = last.then(run, run);
    last = next.catch(() => undefined);
    return next;
  };
}
