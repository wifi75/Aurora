import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { getCat, postControl, postRxFrequency, postTxFrequency } from '../api/client';
import { useT } from '../i18n';
import { signalBandwidthHz } from '../lib/markers';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { useStation } from '../state/station';
import { FrequencyMarkers } from './FrequencyMarkers';
import { FrequencyRuler } from './FrequencyRuler';
import { RxControls } from './RxControls';
import { StopButton } from './StopButton';
import { Waterfall } from './Waterfall';

/**
 * Quello che serve alla Fox senza cambiare pagina: Abilita Tx e STOP, i comandi di ricezione, la cascata con le barre Rx/Tx
 * (la Fox trasmette sotto i 1000 Hz, gli Hound chiamano sopra) e gli ultimi messaggi ricevuti.
 */
export function FoxStation() {
  const t = useT();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const passbandHz = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 }).data?.status.rig?.passband_hz;
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const wf = useWfPrefs((s) => s.prefs);
  const { from, to } = visibleRange(wf.startHz, spanHz);
  useEffect(() => {
    useSpectrumSpan.getState().set(spanFromPassband(passbandHz));
  }, [passbandHz]);
  const txOn = status?.tx_enabled ?? false;
  const recent = decodes.slice(0, 40);

  return (
    <div className="fox-station">
      <div className="fox-station-bar">
        <button type="button" className={`glass fox-tx${txOn ? ' fox-tx--on' : ''}`} onClick={() => void postControl('enable_tx', !txOn)}>
          {t('fox.enableTx')}
        </button>
        <StopButton variant="bar" />
        <span className="data fox-freq">
          Tx {status?.tx_df ?? '—'} Hz · Rx {status?.rx_df ?? '—'} Hz
        </span>
        <RxControls mode={status?.mode ?? ''} transmitting={status?.transmitting ?? false} />
      </div>
      <div className="fox-wf">
        <div className="wf-stack">
          <Waterfall register={registerSpectrum} mode={status?.mode} />
          <FrequencyMarkers
            fMinHz={from}
            fMaxHz={to}
            rxHz={status?.rx_df}
            txHz={status?.tx_df}
            bandwidthHz={signalBandwidthHz(status?.mode)}
            filter={null}
            showBars={wf.bars}
            showFreq={wf.freq}
            onRx={(hz) => void postRxFrequency(hz)}
            onTx={(hz) => void postTxFrequency(hz)}
          />
        </div>
        <FrequencyRuler fMinHz={from} fMaxHz={to} decodes={decodes} />
      </div>
      <div className="fox-box fox-rx">
        <h2>{t('fox.received')}</h2>
        <ul className="fox-rx-list data">
          {recent.length === 0 ? <li className="muted">{t('fox.empty')}</li> : null}
          {recent.map((d) => {
            const mine = status?.de_call ? d.text.split(' ')[0] === status.de_call : false;
            return (
              <li key={d.id} className={mine ? 'fox-rx--mine' : undefined}>
                <span className="muted">{d.snr > 0 ? `+${d.snr}` : d.snr}</span> <span className="muted">{d.freq_hz}</span> {d.text}
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
