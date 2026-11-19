import { useGroups } from '../../lib/detached';
import { DetachButton } from '../DetachButton';
import { createPortal } from 'react-dom';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { formatVfo } from '../../lib/radioExtras';
import { RadioExtras, useCatExtras, TuneChip } from './RadioExtras';
import { describeDisplay } from './describe';
import { Ft2000Display, Ic7610Display } from './RigFaceDisplays';
import { useT } from '../../i18n';
import { usePhone } from '../../lib/phone';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, isFaceStyle, radioAnswering, isSquareStyle, useNarrowScreen, useRadioDisplay, type DisplayModel, type RadioDisplayStyle } from '../../lib/radioDisplay';
import { IcBlackSquare, IcBlueSquare, YaesuSquare } from './ModernDisplays';
import { useStation } from '../../state/station';
import { RadioDisplay } from './RadioDisplay';
import { CompactRadioDisplay } from './CompactRadioDisplay';
import { useQuery } from '@tanstack/react-query';
import { getCat } from '../../api/client';
import { SetupCta } from '../SetupCta';
import { RigSwitch } from './RigSwitch';
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
  const t = useT();
  const detached = useGroups().some((g) => g.open && g.items.includes('display'));
  const phone = usePhone();
  const narrow = useNarrowScreen();
  const pref = useRadioDisplay((s) => s.pref);
  const status = useStation((s) => s.status);
  const raw = displayModel(status);
  const meters = useMeterBallistics(raw.meters);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const tuning = useTuning();
  // la radio risponde se il CAT e' collegato, se arriva una frequenza senza errori (issue #55) o se arrivano strumenti veri dalla radio
  // (issue #60: S-meter e dati in piu' spariti con il CAT che funziona ma con uno stato diverso da «collegato»)
  const radioOff = !!cat.data && cat.data.config.backend !== 'none' && !radioAnswering(cat.data, raw.meters);
  // il display «spento» come quello vero: scuro con la radio spenta, si illumina quando la radio risponde (classe sul body, cosi' vale per
  // ogni stile e anche per il display staccato)
  useEffect(() => {
    document.body.classList.toggle('rd-radio-off', radioOff);
    // la scritta grande al centro di ogni display spento, nella lingua scelta (la usa il CSS con `content: var(--rd-off-text)`): testo vero,
    // non piu' un'immagine generata (issue #55: se l'immagine non si generava restava solo un riquadro nero)
    document.body.style.setProperty('--rd-off-text', JSON.stringify(t('rd.off')));
    return () => document.body.classList.remove('rd-radio-off');
  }, [radioOff, t]);
  if (pref.hidden) return null;
  // con la radio in split niente lente (richiesta del proprietario): resta solo l'indicazione SPLIT
  const inSplit = cat.data?.status.rig?.split === true;
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

  // schermo stretto: il display grande non entra in testata, resta una riga con frequenza, potenza, ROS e TUNE (scelta forzata)
  if (narrow && !solo && !phone) return <CompactRadioDisplay model={model} />;

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

  // il riquadro PWR + SPLIT sta accanto al display, uguale in ogni stile (fuori dalla lente e dal pulsantino di stacco)
  const withBox = (display: ReactNode) => <div className="rd-face-row">{display}</div>;

  if (isSquareStyle(pref.style)) return <ModernHeader model={model} style={pref.style} solo={solo} />;
  if (isFaceStyle(pref.style)) {
    return withBox(solo ? <FaceHeader model={model} style={pref.style} /> : (
      <Zoom off={inSplit}>
        <FaceHeader model={model} style={pref.style} />
      </Zoom>
    ));
  }

  // due blocchi uguali affiancati, oppure un blocco unico a doppio VFO (VFO A grande a sinistra, VFO B e indicatori a destra)
  const vfoBHz = cat.data?.status.rig?.extras?.vfo_b_hz;
  const vfoBText = !radioOff && vfoBHz ? `${formatVfo(vfoBHz)}${cat.data?.status.rig?.extras?.vfo_b_mode ? ` ${cat.data.status.rig.extras.vfo_b_mode}` : ''}` : null;
  const duo = (
    <div className={`rd-header rd-duo rd-duo--${pref.layout}`}>
      <div className="rd-slot rd-trigger rd-trigger--static rd-slot--mini">
        <RadioDisplay model={model} style={pref.style} size="mini" vfoB={vfoBText} />
        {/* disposizione unica: TUNE sta dentro il display e a destra non resta spazio vuoto; con i due blocchi affiancati resta la scheda */}
        {pref.layout === 'single' ? <TuneChip className="rd-tune--in" /> : null}
      </div>
      {pref.layout === 'single' ? null : (
        <div className="rd-slot rd-slot--extras">
          <RadioExtras hidePwr hideVfos />
        </div>
      )}
    </div>
  );
  return withBox(solo ? duo : <Zoom off={inSplit}>{duo}</Zoom>);
}

const OPEN_MS = 250;
const CLOSE_MS = 300;

/**
 * La lente: passando il mouse sul display, dopo un attimo si apre sotto una copia identica ingrandita 1,5 volte in larghezza e in altezza
 * (CSS `zoom`, come una lente: niente si stira). Resta aperta finche' il mouse e' sul display o sulla copia.
 */
function Zoom({ children, off = false }: { children: ReactNode; off?: boolean }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [width, setWidth] = useState(0);
  const host = useRef<HTMLDivElement | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  // sul pulsantino per staccare il display (e sul suo menu) la lente resta chiusa: prima si apriva sopra il menu e lampeggiava
  const onCorner = useRef(false);
  const later = (next: boolean, ms: number) => {
    clearTimeout(timer.current);
    if (next && (onCorner.current || off)) return;
    timer.current = setTimeout(() => {
      if (next && host.current) setWidth(host.current.offsetWidth);
      setOpen(next);
    }, ms);
  };
  useEffect(() => () => clearTimeout(timer.current), []);
  // lo split si accende con la lente aperta: si chiude subito
  useEffect(() => {
    if (!off) return;
    clearTimeout(timer.current);
    setOpen(false);
  }, [off]);
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
      {open && !off ? (
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
  const data = { extras: rig?.extras ?? {} };
  const props = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t), data, waiting, onAction: (item: Parameters<typeof click>[0]) => void click(item) };
  return <div className={`rd-header rd-face-slot rd-face-slot--${style === 'ft2000' ? 'ft' : 'ic'}`}>{style === 'ft2000' ? <Ft2000Display {...props} /> : <Ic7610Display {...props} />}</div>;
}

/** I display moderni in testata: gli estesi nella riga dei comandi, i quasi quadrati fissi nell'angolo (le icone diventano piccole). */
function ModernHeader({ model, style, solo = false }: { model: DisplayModel; style: RadioDisplayStyle; solo?: boolean }) {
  const t = useT();
  const { rig, waiting, click } = useCatExtras();
  const data = { extras: rig?.extras ?? {} };
  const props = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t), data, waiting, onAction: (item: Parameters<typeof click>[0]) => void click(item) };
  const square = isSquareStyle(style);
  useEffect(() => {
    if (!square || solo) return;
    document.body.classList.add('rd-square');
    return () => document.body.classList.remove('rd-square');
  }, [square, solo]);
  const view =
    style === 'icblueq' ? <IcBlueSquare {...props} />
    : style === 'icblackq' ? <IcBlackSquare {...props} />
    : <YaesuSquare {...props} />;
  // il riquadro PWR + SPLIT sta a sinistra del display, fuori dal suo angolo (dove si vede il pulsantino di stacco)
  // il quasi quadrato va sulla pagina (non dentro la riga dei comandi, che fa da contenitore e lo spostava sotto la testata)
  if (square && solo) return <div className="rd-square-preview">{view}</div>;
  if (square)
    return createPortal(
      <div className="rd-square-slot rd-detachable">
        {view}
        <span className="rd-detach-corner">
          <DetachButton item="display" className="dock-detach" />
        </span>
      </div>,
      document.body,
    );
  return <div className="rd-modern-slot">{view}</div>;
}
