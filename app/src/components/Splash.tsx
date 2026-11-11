import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { useT } from '../i18n';
import { Logo } from './Logo';

/** La versione dell'interfaccia scritta al momento della compilazione (vite.config.ts, dal package.json). */
declare const __AURORA_UI_VERSION__: string;
const UI_VERSION = typeof __AURORA_UI_VERSION__ === 'string' ? __AURORA_UI_VERSION__ : '';

/** I modi che Aurora decodifica e trasmette oggi: sigle tecniche, uguali in tutte le lingue. */
const MODES = ['FT8', 'FT4', 'JT65', 'JT9', 'Q65', 'MSK144', 'WSPR'] as const;

/** Mostrata una sola volta per avvio del programma: anche quando la finestra si ricarica (aggiornamento, ricollegamento, Ctrl+R)
 *  la schermata e il saluto sonoro non tornano (lo si ricorda per la sessione della finestra). */
const SEEN_KEY = 'aurora-splash-seen';
function seenThisSession(): boolean {
  try {
    return sessionStorage.getItem(SEEN_KEY) === '1';
  } catch {
    return false;
  }
}
function markSeen(): void {
  try {
    sessionStorage.setItem(SEEN_KEY, '1');
  } catch {
    // senza archivio si torna al comportamento di prima
  }
}
let shown = seenThisSession();
let chirped = shown;

const SHOW_MS = 2600;
const FADE_MS = 500;

/** Otto toni crescenti come in FT8: un saluto breve, solo se il sistema lo permette. */
function chirp() {
  try {
    const Ctx = window.AudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    // un contesto nato sospeso (finestra non ancora attiva) si riaccende: senza, i toni partono ma non si sentono
    if (ctx.state === 'suspended') void ctx.resume();
    const start = ctx.currentTime + 0.05;
    for (let i = 0; i < 8; i++) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.frequency.value = 440 * Math.pow(2, i / 6);
      gain.gain.setValueAtTime(0.0001, start + i * 0.09);
      gain.gain.exponentialRampToValueAtTime(0.06, start + i * 0.09 + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + i * 0.09 + 0.14);
      osc.connect(gain).connect(ctx.destination);
      osc.start(start + i * 0.09);
      osc.stop(start + i * 0.09 + 0.16);
    }
    window.setTimeout(() => void ctx.close(), 1500);
  } catch {
    /* l'audio puo' essere bloccato: lo splash resta muto */
  }
}

export function Splash() {
  const t = useT();
  const [phase, setPhase] = useState<'show' | 'fade' | 'gone'>(shown ? 'gone' : 'show');
  const { data } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });

  useEffect(() => {
    if (shown) return;
    if (!chirped) {
      chirped = true;
      markSeen();
      chirp();
    }
    const a = window.setTimeout(() => setPhase('fade'), SHOW_MS);
    const b = window.setTimeout(() => {
      shown = true;
      setPhase('gone');
    }, SHOW_MS + FADE_MS);
    return () => {
      window.clearTimeout(a);
      window.clearTimeout(b);
    };
  }, []);

  if (phase === 'gone') return null;
  const version = data?.version || UI_VERSION;
  const skip = () => {
    shown = true;
    setPhase('gone');
  };
  return (
    <div className={`splash${phase === 'fade' ? ' splash--fade' : ''}`} role="presentation" onClick={skip} onKeyDown={skip}>
      <Logo size={200} animated className="splash-logo" />
      <h1 className="splash-name">AURORA</h1>
      <p className="splash-motto">{t('splash.motto')}</p>
      <ul className="splash-modes" aria-label={t('splash.modes')}>
        {MODES.map((m) => (
          <li key={m}>{m}</li>
        ))}
      </ul>
      <p className="splash-author">{t('splash.author')}</p>
      {/* la versione del core quando risponde, altrimenti quella dell'interfaccia: mai scritta a mano */}
      <p className="splash-version">{version ? `v${version}` : ''}</p>
    </div>
  );
}
