import { useCallback, useEffect, useState } from 'react';
import { ApiError, postReply } from '../api/client';
import { openStream } from '../api/stream';
import { DecodeList } from '../components/DecodeList';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { Waterfall } from '../components/Waterfall';
import { emitSpectrum, registerSpectrum } from '../lib/spectrumFeed';
import { useStation } from '../state/station';

function replyMessage(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 403) return 'Da questa connessione non si può trasmettere.';
    if (error.status === 404) return 'Quel messaggio non è più disponibile.';
    if (error.status === 401) return 'Dispositivo non associato: associalo di nuovo.';
  }
  return 'Risposta non inviata. Controlla la connessione e riprova.';
}

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
      setNote(replyMessage(error));
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
