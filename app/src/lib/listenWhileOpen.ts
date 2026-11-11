import { useEffect, useRef } from 'react';

/** Il comando dell'ascolto di un servizio (modi testuali, SSTV). */
export type RxCommand = (action: 'start' | 'stop') => Promise<void>;

/**
 * L'ascolto di una pagina (PSK·RTTY·CW, SSTV): entrando si accende, uscendo si spegne, salvo «Ascolto acceso all'avvio di Aurora».
 *
 * Entrando si accende sempre, senza guardare lo stato in memoria: la pagina parte dai dati della visita di prima (il servizio
 * allora ascoltava), mentre uscendo l'ascolto e' stato spento. Fidandosi di quei dati vecchi l'ascolto restava spento e il CW non
 * decodificava finche' non si usciva e rientrava una seconda volta (issue #45). Accendere un ascolto gia' acceso non cambia niente.
 */
export function useListenWhileOpen(rx: RxCommand, keepOnLeave: () => boolean, onChanged: () => void): void {
  const live = useRef({ rx, keepOnLeave, onChanged });
  live.current = { rx, keepOnLeave, onChanged };
  useEffect(() => {
    const done = () => live.current.onChanged();
    void live.current.rx('start').then(done, done);
    return () => {
      if (!live.current.keepOnLeave()) void live.current.rx('stop').catch(() => undefined);
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
