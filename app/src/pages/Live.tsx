import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getStationInfo, postReply } from '../api/client';
import { openStream } from '../api/stream';
import { AlertSwitch } from '../components/AlertSwitch';
import { BandBar } from '../components/BandBar';
import { Dock } from '../components/Dock';
import { TxPanel } from '../components/TxPanel';
import { DecodeList } from '../components/DecodeList';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { StationPanel } from '../components/StationPanel';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { WaterfallSizeSwitch } from '../components/WaterfallSizeSwitch';
import { Waterfall } from '../components/Waterfall';
import { pickAlert, playAlert, readAlerts, saveAlerts, webAudioContext, type AlertPrefs, type ToneContext } from '../lib/alerts';
import { replyErrorMessage } from '../lib/replyError';
import { emitSpectrum, registerSpectrum } from '../lib/spectrumFeed';
import { browserStorage } from '../lib/theme';
import { clearHeight, readLayout, resizePanel, saveLayout, type Layout, type PanelId } from '../lib/layout';
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

  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  const [layout, setLayout] = useState<Layout>(() => readLayout(browserStorage()));
  const changeLayout = (next: Layout) => {
    saveLayout(browserStorage(), next);
    setLayout(next);
  };

  // Grande = la cascata prende lo spazio che resta; Ridotta = una fascia; Nascosta = niente cascata
  const changeSize = (next: WaterfallSize) => {
    saveSize(browserStorage(), next);
    setSize(next);
    changeLayout(next === 'small' ? resizePanel(layout, 'waterfall', 160) : clearHeight(layout, 'waterfall'));
  };

  const titles: Record<PanelId, string> = { waterfall: 'Cascata', station: 'Stazione', tx: 'Trasmissione', cq: 'CQ e novità', all: 'Attività di banda' };
  const renderPanel = (id: PanelId) => {
    switch (id) {
      case 'waterfall':
        return (
          <>
            <Waterfall register={registerSpectrum} />
            <FrequencyRuler fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} decodes={decodes} />
          </>
        );
      case 'station':
        return <StationPanel status={status} />;
      case 'tx':
        return <TxPanel txAllowed={info.data?.tx_allowed ?? false} status={status} />;
      case 'cq':
        return <DecodeList decodes={cqDecodes(decodes)} myGrid={status?.de_grid} onReply={onReply} emptyText="Nessun CQ per ora" />;
      case 'all':
        return <DecodeList decodes={decodes} myGrid={status?.de_grid} onReply={onReply} />;
    }
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
      <BandBar dialHz={status?.dial_hz ?? 0} />
      <Dock layout={layout} onChange={changeLayout} titles={titles} hidden={size === 'hidden' ? ['waterfall'] : []} render={renderPanel} />
      <p className="live-note" role="status" aria-live="polite">
        {note}
      </p>
      <StopButton variant="bar" />
    </div>
  );
}
