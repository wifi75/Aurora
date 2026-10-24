import { useCallback, useEffect, useState } from 'react';
import { postReply } from '../api/client';
import { openStream } from '../api/stream';
import { DecodeList } from '../components/DecodeList';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Panel } from '../components/Panel';
import { StationPanel } from '../components/StationPanel';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { WaterfallSizeSwitch } from '../components/WaterfallSizeSwitch';
import { Waterfall } from '../components/Waterfall';
import { replyErrorMessage } from '../lib/replyError';
import { emitSpectrum, registerSpectrum } from '../lib/spectrumFeed';
import { browserStorage } from '../lib/theme';
import { readSize, saveSize, type WaterfallSize } from '../lib/waterfallSize';
import { cqDecodes } from '../state/selectors';
import { useStation } from '../state/station';

const F_MIN_HZ = 0;
const F_MAX_HZ = 3000;

export function Live() {
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const connection = useStation((s) => s.connection);
  const [note, setNote] = useState('');
  const [size, setSize] = useState<WaterfallSize>(() => readSize(browserStorage()));

  const changeSize = (next: WaterfallSize) => {
    saveSize(browserStorage(), next);
    setSize(next);
  };

  useEffect(() => {
    const { apply, setConnection } = useStation.getState();
    const stream = openStream({
      onEvent: apply,
      onSpectrum: (frame) => emitSpectrum(frame.bins),
      onState: setConnection,
    });
    return () => stream.close();
  }, []);

  const onReply = useCallback(async (id: number) => {
    const target = useStation.getState().decodes.find((d) => d.id === id);
    try {
      await postReply(id);
      setNote(`Risposta inviata: ${target?.text ?? ''}`);
    } catch (error) {
      setNote(replyErrorMessage(error));
    }
  }, []);

  return (
    <div className="live">
      <div className="live-top">
        <StatusBar status={status} connection={connection} />
        <div className="live-actions">
          <WaterfallSizeSwitch value={size} onChange={changeSize} />
          <StopButton variant="compact" />
        </div>
      </div>
      <div className={`live-body live-body--${size}`}>
        <section className="desk-main" aria-label="Cascata">
          <Waterfall register={registerSpectrum} />
          <FrequencyRuler fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} decodes={decodes} />
        </section>
        <aside className="desk-side">
          <Panel title="Stazione" className="station-panel">
            <StationPanel status={status} />
          </Panel>
          <Panel title="Chiamate CQ" className="panel--cq">
            <DecodeList decodes={cqDecodes(decodes)} onReply={onReply} emptyText="Nessun CQ per ora" />
          </Panel>
          <Panel title="Tutti i messaggi" className="panel--all">
            <DecodeList decodes={decodes} onReply={onReply} />
          </Panel>
        </aside>
      </div>
      <p className="live-note" role="status" aria-live="polite">
        {note}
      </p>
      <StopButton variant="bar" />
    </div>
  );
}
