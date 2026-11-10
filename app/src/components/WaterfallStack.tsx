import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { getCat, getControls, postRxFrequency, postTxFrequency } from '../api/client';
import { bandOf } from '../lib/format';
import { filterWindowHz, signalBandwidthHz, signalStartOffsetHz } from '../lib/markers';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { useStation } from '../state/station';
import { FrequencyMarkers } from './FrequencyMarkers';
import { FrequencyRuler } from './FrequencyRuler';
import { SpotMarkers } from './SpotMarkers';
import { Waterfall } from './Waterfall';

/**
 * La cascata della pagina Stazione senza cornice (usata da Fox e Contest): filtro, spot del cluster, barre Rx/Tx trascinabili,
 * clic che sposta Rx (Maiusc+clic la Tx) e righello con i messaggi. Riempie il contenitore, che ne decide l'altezza.
 */
export function WaterfallStack() {
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const passbandHz = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 }).data?.status.rig?.passband_hz;
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const wf = useWfPrefs((s) => s.prefs);
  const { from, to } = visibleRange(wf.startHz, spanHz);
  useEffect(() => {
    useSpectrumSpan.getState().set(spanFromPassband(passbandHz));
  }, [passbandHz]);
  const switches = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false }).data;
  const filterWindow = switches?.filter === true ? filterWindowHz(status?.mode, switches.hound === true) : null;
  return (
    <>
      <div className="wf-stack">
        <Waterfall register={registerSpectrum} mode={status?.mode} band={status ? (bandOf(status.dial_hz) ?? undefined) : undefined} />
        <FrequencyMarkers
          fMinHz={from}
          fMaxHz={to}
          rxHz={status?.rx_df}
          txHz={status?.tx_df}
          bandwidthHz={signalBandwidthHz(status?.mode)} startOffsetHz={signalStartOffsetHz(status?.mode)}
          filter={filterWindow}
          showBars={wf.bars}
          showFreq={wf.freq}
          onRx={(hz) => void postRxFrequency(hz)}
          onTx={(hz) => void postTxFrequency(hz)}
        />
        <SpotMarkers fMinHz={from} fMaxHz={to} dialHz={status?.dial_hz} onPick={(hz) => void postRxFrequency(hz)} />
      </div>
      <FrequencyRuler fMinHz={from} fMaxHz={to} decodes={decodes} rxHz={status?.rx_df} txHz={status?.tx_df} bandwidthHz={signalBandwidthHz(status?.mode)} startOffsetHz={signalStartOffsetHz(status?.mode)} />
    </>
  );
}
