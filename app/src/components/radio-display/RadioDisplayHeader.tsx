import { useEffect, useRef, useState, type ReactNode } from 'react';
import { RadioExtras, useCatExtras } from './RadioExtras';
import { describeDisplay } from './describe';
import { Ft2000Display, Ic7610Display } from './RigFaceDisplays';
import { useT } from '../../i18n';
import { usePhone } from '../../lib/phone';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, isFaceStyle, useRadioDisplay, type DisplayModel } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { RadioDisplay } from './RadioDisplay';

/**
 * Il display in testata: piccolo; passandoci sopra col mouse si apre una copia identica ingrandita 2 volte (lente) con gli stessi
 * indicatori cliccabili, per leggerlo bene. Sul telefono niente zoom. Con "Nascondi il display" non c'e' affatto.
 */
export function RadioDisplayHeader() {
  const phone = usePhone();
  const pref = useRadioDisplay((s) => s.pref);
  const status = useStation((s) => s.status);
  const raw = displayModel(status);
  const meters = useMeterBallistics(raw.meters);
  if (pref.hidden) return null;
  const model = { ...raw, meters };

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

  if (isFaceStyle(pref.style)) {
    return (
      <Zoom>
        <FaceHeader model={model} style={pref.style} />
      </Zoom>
    );
  }

  // due blocchi uguali affiancati, oppure un blocco unico a doppio VFO (VFO A grande a sinistra, VFO B e indicatori a destra)
  return (
    <Zoom>
    <div className={`rd-header rd-duo rd-duo--${pref.layout}`}>
      <div className="rd-slot rd-trigger rd-trigger--static">
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </div>
      <div className="rd-slot rd-slot--extras">
        <RadioExtras />
      </div>
    </div>
    </Zoom>
  );
}

const OPEN_MS = 250;
const CLOSE_MS = 300;

/**
 * La lente: passando il mouse sul display, dopo un attimo si apre sotto una copia identica ingrandita 2 volte in larghezza E in altezza
 * (CSS `zoom`, come una lente: niente si stira). Resta aperta finche' il mouse e' sul display o sulla copia.
 */
function Zoom({ children }: { children: ReactNode }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [width, setWidth] = useState(0);
  const host = useRef<HTMLDivElement | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const later = (next: boolean, ms: number) => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      if (next && host.current) setWidth(host.current.offsetWidth);
      setOpen(next);
    }, ms);
  };
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <div className="rd-zoom-host" ref={host} onMouseEnter={() => later(true, OPEN_MS)} onMouseLeave={() => later(false, CLOSE_MS)}>
      {children}
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
      <div className="rd-header rd-face-slot">{style === 'ft2000' ? <Ft2000Display {...props} /> : <Ic7610Display {...props} />}</div>
      {/* la potenza della radio, a passi del 5%: nel pannello disegnato non c'e' posto per i pulsanti */}
      {power != null ? (
        <div className="rd-pwr" role="group" aria-label="PWR">
          <span className="rd-pwr-val data">{Math.round(power * 100)}%</span>
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
