import { useEffect, useRef } from 'react';
import { useLang, useT } from '../i18n';
import { browserStorage } from '../lib/theme';
import { showAgentPopup } from './AgentPopup';

/** Il servizio dei messaggi del proprietario (lo stesso dei rapporti degli errori). */
const URL_MESSAGGI = 'https://aurora-rapporti.tizianocassone.workers.dev/v1/messaggi';
const SEEN_KEY = 'aurora-messaggi-visti';
const EVERY_MS = 60 * 60_000;

declare const __AURORA_UI_VERSION__: string;

type Testi = Partial<Record<'it' | 'en' | 'de' | 'fr' | 'es' | 'pt', string>>;
interface Messaggio {
  id: string;
  titolo: Testi;
  testo: Testi;
  link: string;
  tema: 'normale' | 'festa';
}

function seen(): string[] {
  try {
    const v: unknown = JSON.parse(browserStorage()?.getItem(SEEN_KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

function markSeen(id: string): void {
  try {
    browserStorage()?.setItem(SEEN_KEY, JSON.stringify([...seen(), id].slice(-200)));
  } catch {
    // senza archivio il messaggio potrebbe tornare al prossimo avvio
  }
}

/** Il testo nella lingua di chi legge; altrimenti inglese, poi italiano. */
function inLang(t: Testi, lang: string): string {
  const key = lang.slice(0, 2) as keyof Testi;
  return t[key] ?? t.en ?? t.it ?? '';
}

/**
 * I messaggi scritti dal proprietario (auguri, avvisi): si leggono all'avvio e poi ogni ora e ognuno compare una volta sola, come
 * cartolina con il suono. Sempre accesi (scelta del proprietario). Solo nella finestra principale del PC.
 */
export function OwnerMessages() {
  const lang = useLang((s) => s.lang);
  const t = useT();
  // lingua e testi letti al momento: il controllo parte una volta sola, non a ogni cambio
  const now = useRef({ lang, t });
  now.current = { lang, t };
  useEffect(() => {
    let stop = false;
    const version = typeof __AURORA_UI_VERSION__ === 'string' ? __AURORA_UI_VERSION__ : '';
    const check = async () => {
      try {
        const r = await fetch(`${URL_MESSAGGI}?v=${encodeURIComponent(version)}`, { cache: 'no-store' });
        if (!r.ok) return;
        const { messaggi } = (await r.json()) as { messaggi?: Messaggio[] };
        const done = new Set(seen());
        // uno alla volta: il piu' recente non ancora visto
        const next = (messaggi ?? []).find((m) => !done.has(m.id));
        if (!next || stop) return;
        const { lang, t } = now.current;
        markSeen(next.id);
        showAgentPopup(
          {
            kind: next.tema === 'festa' ? '✦ IU3WJQ' : 'IU3WJQ',
            title: inLang(next.titolo, lang),
            text: inLang(next.testo, lang),
            primary: next.link ? t('agent.msg.open') : t('upd.poster.ok'),
            secondary: next.link ? t('upd.poster.ok') : '',
            link: next.link || undefined,
            festive: next.tema === 'festa',
          },
          20,
        );
      } catch {
        // senza internet si riprova al giro dopo
      }
    };
    void check();
    const timer = window.setInterval(() => void check(), EVERY_MS);
    return () => {
      stop = true;
      window.clearInterval(timer);
    };
  }, []);
  return null;
}
