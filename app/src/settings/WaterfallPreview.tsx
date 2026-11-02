import { useEffect, useState, useSyncExternalStore } from 'react';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Waterfall } from '../components/Waterfall';
import { useT } from '../i18n';
import { signalBandwidthHz } from '../lib/markers';
import { registerSpectrum, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { useStation } from '../state/station';
import { Help } from './layout';

const OPEN_KEY = 'aurora.wfPreview.open';
/** Senza righe per tanto tempo l'anteprima dice che il motore non manda dati. */
const QUIET_MS = 3000;

function readOpen(): boolean {
  try {
    return localStorage.getItem(OPEN_KEY) === '1';
  } catch {
    return false;
  }
}

function writeOpen(open: boolean): void {
  try {
    localStorage.setItem(OPEN_KEY, open ? '1' : '0');
  } catch {
    // memoria del browser non disponibile: la scelta vale solo adesso
  }
}

function subscribeVisibility(cb: () => void): () => void {
  document.addEventListener('visibilitychange', cb);
  return () => document.removeEventListener('visibilitychange', cb);
}

/** Vero solo con la finestra visibile (non nascosta ne' ridotta a icona). */
export function usePageVisible(): boolean {
  return useSyncExternalStore(subscribeVisibility, () => document.visibilityState !== 'hidden');
}

/** Vero se nelle ultime `QUIET_MS` e' arrivata almeno una riga di spettro. */
function useReceiving(): boolean {
  const [receiving, setReceiving] = useState(true);
  useEffect(() => {
    let last = Date.now();
    let seen = false;
    const off = registerSpectrum(() => {
      last = Date.now();
      seen = true;
    });
    const timer = setInterval(() => setReceiving(seen && Date.now() - last < QUIET_MS), 1000);
    return () => {
      off();
      clearInterval(timer);
    };
  }, []);
  return receiving;
}

/** Il corpo dell'anteprima: montato solo con l'anteprima aperta e la finestra visibile. */
function LivePreview() {
  const t = useT();
  const wf = useWfPrefs((s) => s.prefs);
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const status = useStation((s) => s.status);
  const receiving = useReceiving();
  const { from, to } = visibleRange(wf.startHz, spanHz);
  return (
    <div className="wfs-preview" data-testid="wf-preview">
      <div className="wf-stack">
        <Waterfall register={registerSpectrum} mode={status?.mode} />
        <FrequencyMarkers
          fMinHz={from}
          fMaxHz={to}
          rxHz={status?.rx_df}
          txHz={status?.tx_df}
          bandwidthHz={signalBandwidthHz(status?.mode)}
          showBars={wf.bars}
          showFreq={wf.freq}
          onRx={() => {}}
          onTx={() => {}}
        />
        {!receiving && <p className="waterfall-note wfs-preview-quiet">{t('wfs.preview.noData')}</p>}
      </div>
      <FrequencyRuler fMinHz={from} fMaxHz={to} decodes={[]} />
    </div>
  );
}

/**
 * Anteprima dal vivo della cascata e dello spettro nelle Impostazioni: gli stessi componenti della pagina Stazione con le righe
 * vere del motore. Si spegne del tutto (niente renderer, niente abbonamento alle righe) quando e' chiusa, quando si lascia la
 * scheda o quando la finestra non e' visibile.
 */
export function WaterfallPreview() {
  const t = useT();
  const [open, setOpen] = useState(readOpen);
  const visible = usePageVisible();
  const toggle = () => {
    setOpen((o) => {
      writeOpen(!o);
      return !o;
    });
  };
  return (
    <div className="wfs-preview-box">
      <div className="wfs-preview-head">
        <button title={t('tip.ui.preview')} type="button" className="glass wfs-preview-toggle" aria-pressed={open} onClick={toggle}>
          {open ? t('wfs.preview.hide') : t('wfs.preview.show')}
        </button>
        <Help tip={t('tip.wfs.preview')} />
      </div>
      {open && visible && <LivePreview />}
    </div>
  );
}
