import { powerPct } from '../../lib/radioDisplay';
import { useGroups } from '../../lib/detached';
import { DetachButton } from '../DetachButton';
import { createPortal } from 'react-dom';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { RadioExtras, useCatExtras } from './RadioExtras';
import { describeDisplay } from './describe';
import { Ft2000Display, Ic7610Display } from './RigFaceDisplays';
import { useT } from '../../i18n';
import { usePhone } from '../../lib/phone';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, isFaceStyle, isSquareStyle, useRadioDisplay, type DisplayModel, type RadioDisplayStyle } from '../../lib/radioDisplay';
import { IcBlackSquare, IcBlueDisplay, IcBlueSquare, IcDualDisplay, LedLineDisplay, YaesuSquare } from './ModernDisplays';
import { useStation } from '../../state/station';
import { RadioDisplay } from './RadioDisplay';
import { useQuery } from '@tanstack/react-query';
import { getCat } from '../../api/client';
import { SetupCta } from '../SetupCta';
import { RigSwitch } from './RigSwitch';
import { SplitOffButton } from './SplitOffButton';
import { useTuning } from './RadioExtras';

/**
 * Il display in testata: piccolo; passandoci sopra col mouse si apre una copia identica ingrandita 2 volte (lente) con gli stessi
 * indicatori cliccabili, per leggerlo bene. Sul telefono niente zoom. Con "Nascondi il display" non c'e' affatto.
 */
export function RadioDisplayHeader({ solo = false }: { solo?: boolean } = {}) {
  const pref = useRadioDisplay((s) => s.pref);
  // display nascosto: niente, nemmeno il contenitore vuoto
  if (pref.hidden) return null;
  // con OmniRig il blocchetto Rig 1 / Rig 2 sta a sinistra del display, nello stesso stile
  return (
    <div className="rd-with-rig">
      <RigSwitch style={pref.style} />
      <RadioDisplayHeaderInner solo={solo} />
    </div>
  );
}

function RadioDisplayHeaderInner({ solo = false }: { solo?: boolean } = {}) {
  const detached = useGroups().some((g) => g.open && g.items.includes('display'));
  const phone = usePhone();
  const pref = useRadioDisplay((s) => s.pref);
  const status = useStation((s) => s.status);
  const raw = displayModel(status);
  const meters = useMeterBallistics(raw.meters);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const tuning = useTuning();
  const radioOff = !!cat.data && cat.data.config.backend !== 'none' && cat.data.status.state !== 'connected';
  // il display «spento» come quello vero: scuro con la radio spenta, si illumina quando la radio risponde (classe sul body, cosi' vale per
  // ogni stile e anche per il display staccato)
  useEffect(() => {
    document.body.classList.toggle('rd-radio-off', radioOff);
    return () => document.body.classList.remove('rd-radio-off');
  }, [radioOff]);
  if (pref.hidden) return null;
  // il display sta in una finestra a parte (secondo schermo): qui resta solo un segnaposto vuoto
  if (detached && !solo) return <span className="rd-detached" aria-hidden="true" />;
  // TX acceso anche mentre la radio accorda (TUNE del display) o con la portante di accordo della SDR TCI accesa
  const tciTune = cat.data?.config.backend === 'tci' && cat.data?.status.rig?.extras?.tuner === true;
  // radio spenta o che non risponde (CAT non collegato): niente frequenza, S-meter e TX rimasti dall'ultima lettura o presi dall'audio; il
  // display mostra «radio spenta» (prima restavano la frequenza vecchia e lo S-meter che si muoveva con il rumore: proprietario, 2026-11-13)
  const live = { ...raw, meters, transmitting: raw.transmitting || tuning || tciTune };
  const model = radioOff ? { ...live, dialHz: 0, meters: null, transmitting: false } : live;
  // radio mai configurata: un solo riquadro che porta alla configurazione. Radio configurata ma spenta o che non risponde: resta il display,
  // spento, con la scritta «Radio non risponde» (il CAT riprova da solo; il riquadro «configura» qui era fuorviante)
  // «Nessuno (senza CAT)» scelto apposta: il display non ha niente da mostrare (frequenza, S-meter, potenza arrivano dal CAT) e sparisce
  if (cat.data?.config.backend === 'none') return null;
  const state = cat.data?.status.state;
  // configurata: Hamlib con un modello scelto, oppure qualunque altro sistema (OmniRig, TCI, FT-991(A) Link, Cat4OM non hanno un modello
  // Hamlib: prima con Cat4OM spento il display spariva e al suo posto compariva «configura la radio»)
  const configured = !!cat.data && (cat.data.config.backend !== 'hamlib' || cat.data.config.model !== 0);
  if (!phone && cat.data && !configured && (state === 'disconnected' || state === 'error')) {
    return <SetupCta className="setup-cta--radio" href="#/impostazioni/radio" title="cta.radio.none" action="cta.radio.go" />;
  }

  if (phone) {
    return (
      <div className="rd-header rd-header--phone">
        <div className="rd-trigger rd-trigger--static">
          <RadioDisplay model={model} style={pref.style} size="mini" />
        </div>
        <RadioExtras />
      </div>
    );
  }

  if (isSquareStyle(pref.style) || pref.style === 'icdual' || pref.style === 'icblue' || pref.style === 'ledline') {
    return solo ? <ModernHeader model={model} style={pref.style} solo /> : isSquareStyle(pref.style) ? <ModernHeader model={model} style={pref.style} /> : <Detachable><ModernHeader model={model} style={pref.style} /></Detachable>;
  }

  if (isFaceStyle(pref.style)) {
    return solo ? <FaceHeader model={model} style={pref.style} /> : (
      <Zoom>
        <FaceHeader model={model} style={pref.style} />
      </Zoom>
    );
  }

  // due blocchi uguali affiancati, oppure un blocco unico a doppio VFO (VFO A grande a sinistra, VFO B e indicatori a destra)
  const duo = (
    <div className={`rd-header rd-duo rd-duo--${pref.layout}`}>
      <div className="rd-slot rd-trigger rd-trigger--static">
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </div>
      <div className="rd-slot rd-slot--extras">
        <RadioExtras />
      </div>
    </div>
  );
  return solo ? duo : <Zoom>{duo}</Zoom>;
}

/** Il pulsantino discreto per portare il display su un altro schermo, nell'angolo in alto a destra (si vede passandoci sopra). */
function Detachable({ children }: { children: ReactNode }) {
  return (
    <div className="rd-detachable">
      {children}
      <span className="rd-detach-corner">
        <DetachButton item="display" className="dock-detach" />
      </span>
    </div>
  );
}

const OPEN_MS = 250;
const CLOSE_MS = 300;

/**
 * La lente: passando il mouse sul display, dopo un attimo si apre sotto una copia identica ingrandita 1,5 volte in larghezza e in altezza
 * (CSS `zoom`, come una lente: niente si stira). Resta aperta finche' il mouse e' sul display o sulla copia.
 */
function Zoom({ children }: { children: ReactNode }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [width, setWidth] = useState(0);
  const host = useRef<HTMLDivElement | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  // sul pulsantino per staccare il display (e sul suo menu) la lente resta chiusa: prima si apriva sopra il menu e lampeggiava
  const onCorner = useRef(false);
  const later = (next: boolean, ms: number) => {
    clearTimeout(timer.current);
    if (next && onCorner.current) return;
    timer.current = setTimeout(() => {
      if (next && host.current) setWidth(host.current.offsetWidth);
      setOpen(next);
    }, ms);
  };
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <div className="rd-zoom-host" ref={host} onMouseEnter={() => later(true, OPEN_MS)} onMouseLeave={() => later(false, CLOSE_MS)}>
      {children}
      <span
        className="rd-detach-corner"
        onMouseEnter={() => {
          onCorner.current = true;
          later(false, 0);
        }}
        onMouseLeave={() => {
          onCorner.current = false;
        }}
      >
        <DetachButton item="display" className="dock-detach" />
      </span>
      {open ? (
        <div className="rd-zoom" role="group" aria-label={t('rd.popover')}>
          <div className="rd-zoom-lens" style={{ width: width || undefined }}>
            {children}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** Pannello unico in testata (FT-2000, IC-7610): stessa misura dei due blocchi, con gli indicatori cliccabili come nel display esteso. */
function FaceHeader({ model, style }: { model: DisplayModel; style: 'ft2000' | 'ic7610' }) {
  const t = useT();
  const { rig, waiting, click } = useCatExtras();
  const data = { extras: rig?.extras ?? {}, passbandHz: rig?.passband_hz ?? null, split: rig?.split ?? null };
  const props = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t), data, waiting, onAction: (item: Parameters<typeof click>[0]) => void click(item) };
  const power = rig?.extras?.power_set;
  return (
    <div className="rd-face-row">
      {/* «Spegni split» a sinistra del display (richiesta del proprietario) */}
      <SplitOffButton split={rig?.split} />
      <div className="rd-header rd-face-slot">{style === 'ft2000' ? <Ft2000Display {...props} /> : <Ic7610Display {...props} />}</div>
      {/* la potenza della radio, a passi del 5%: nel pannello disegnato non c'e' posto per i pulsanti */}
      {power != null && style !== 'ft2000' ? (
        <div className="rd-pwr" role="group" aria-label="PWR">
          <span className="rd-pwr-val data">{powerPct(power)}%</span>
          <div className="rd-pwr-btns">
            <button type="button" className="rd-pwr-btn" title={t('rx.act.pwrDown')} aria-label={t('rx.act.pwrDown')} disabled={waiting !== null} onClick={() => void click('pwr_down')}>
              −
            </button>
            <button type="button" className="rd-pwr-btn" title={t('rx.act.pwrUp')} aria-label={t('rx.act.pwrUp')} disabled={waiting !== null} onClick={() => void click('pwr_up')}>
              +
            </button>
          </div>
          <span className="rd-pwr-lbl">PWR</span>
        </div>
      ) : null}
    </div>
  );
}

/** I display moderni in testata: gli estesi nella riga dei comandi, i quasi quadrati fissi nell'angolo (le icone diventano piccole). */
function ModernHeader({ model, style, solo = false }: { model: DisplayModel; style: RadioDisplayStyle; solo?: boolean }) {
  const t = useT();
  const { rig, waiting, click } = useCatExtras();
  const data = { extras: rig?.extras ?? {}, passbandHz: rig?.passband_hz ?? null, split: rig?.split ?? null };
  const props = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t), data, waiting, onAction: (item: Parameters<typeof click>[0]) => void click(item) };
  const square = isSquareStyle(style);
  useEffect(() => {
    if (!square || solo) return;
    document.body.classList.add('rd-square');
    return () => document.body.classList.remove('rd-square');
  }, [square, solo]);
  const view =
    style === 'icdual' ? <IcDualDisplay {...props} />
    : style === 'icblue' ? <IcBlueDisplay {...props} />
    : style === 'ledline' ? <LedLineDisplay {...props} />
    : style === 'icblueq' ? <IcBlueSquare {...props} />
    : style === 'icblackq' ? <IcBlackSquare {...props} />
    : <YaesuSquare {...props} />;
  const splitOff = <SplitOffButton split={rig?.split} />;
  // il quasi quadrato va sulla pagina (non dentro la riga dei comandi, che fa da contenitore e lo spostava sotto la testata)
  if (square && solo) return <div className="rd-square-preview">{splitOff}{view}</div>;
  if (square)
    return createPortal(
      <div className="rd-square-slot rd-detachable">
        {splitOff}
        {view}
        <span className="rd-detach-corner">
          <DetachButton item="display" className="dock-detach" />
        </span>
      </div>,
      document.body,
    );
  return <div className="rd-modern-slot">{view}</div>;
}
