import { useCallback, useEffect, useState } from 'react';
import { postReply } from '../api/client';
import { openStream } from '../api/stream';
import { DecodeList } from '../components/DecodeList';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { Waterfall } from '../components/Waterfall';
import { replyErrorMessage } from '../lib/replyError';
import { emitSpectrum, registerSpectrum } from '../lib/spectrumFeed';
import { useStation } from '../state/station';

export function Live() {
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const connection = useStation((s) => s.connection);
  const [note, setNote] = useState('');

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
      <StatusBar status={status} connection={connection} />
      <div className="live-body">
        <Waterfall register={registerSpectrum} />
        <DecodeList decodes={decodes} onReply={onReply} />
      </div>
      <p className="live-note" role="status" aria-live="polite">
        {note}
      </p>
      <StopButton />
    </div>
  );
}
