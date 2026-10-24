import { useCallback, useEffect, useRef, useState } from 'react';
import { postReply } from '../api/client';
import { openStream } from '../api/stream';
import { AlertSwitch } from '../components/AlertSwitch';
import { DecodeList } from '../components/DecodeList';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Panel } from '../components/Panel';
import { StationPanel } from '../components/StationPanel';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { WaterfallSizeSwitch } from '../components/WaterfallSizeSwitch';
import { Waterfall } from '../components/Waterfall';
import { pickAlert, playAlert, readAlerts, saveAlerts, webAudioContext, type AlertPrefs, type ToneContext } from '../lib/alerts';
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

  const [alerts, setAlerts] = useState<AlertPrefs>(() => readAlerts(browserStorage()));
  const alertsRef = useRef(alerts);
  alertsRef.current = alerts;
  const audioRef = useRef<ToneContext | null>(null);

  const changeAlerts = (next: AlertPrefs) => {
    // il contesto audio si crea al primo gesto dell'utente: i browser non lo permettono prima
    if (next.enabled && !audioRef.current) audioRef.current = webAudioContext();
    saveAlerts(browserStorage(), next);
    setAlerts(next);
  };

  // avvisi gia' accesi da una visita precedente: l'audio si abilita al primo tocco sulla pagina
  useEffect(() => {
    if (!alerts.enabled || audioRef.current) return;
    const arm = () => {
      audioRef.current ??= webAudioContext();
    };
    window.addEventListener('pointerdown', arm, { once: true });
    return () => window.removeEventListener('pointerdown', arm);
  }, [alerts.enabled]);

  // suona solo per i messaggi arrivati dopo l'apertura della pagina
  useEffect(() => {
    let lastId: number | null = null;
    return useStation.subscribe((state) => {
      const picked = pickAlert(state.decodes, lastId);
      lastId = picked.lastId;
      const prefs = alertsRef.current;
      if (picked.kind && prefs.enabled && audioRef.current) playAlert(picked.kind, prefs.volume, audioRef.current);
    });
  }, []);

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
          <AlertSwitch value={alerts} onChange={changeAlerts} />
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
          <Panel title="CQ e novità" className="panel--cq">
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
