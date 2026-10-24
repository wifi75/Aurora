import type { DecodedMessage } from '../api/schemas';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getStationInfo, postBand, postControl, postMode, postReply, postRxFrequency, postTxFrequency } from '../api/client';
import { openStream } from '../api/stream';
import { AlertSwitch } from '../components/AlertSwitch';
import { BandBar } from '../components/BandBar';
import { ClusterPanel } from '../components/ClusterPanel';
import { DecodeList } from '../components/DecodeList';
import { Dock } from '../components/Dock';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Log4omPanel } from '../components/Log4omPanel';
import { ModeSwitch } from '../components/ModeSwitch';
import { TxCommands } from '../components/TxCommands';
import { RxControls } from '../components/RxControls';
import { StatusBar } from '../components/StatusBar';
import { StopButton } from '../components/StopButton';
import { TxBanner } from '../components/TxBanner';
import { TxPanel } from '../components/TxPanel';
import { Waterfall } from '../components/Waterfall';
import { useT } from '../i18n';
import { applyFilters, callOf, useFilters } from '../lib/filters';
import { useNotify } from '../lib/notify';
import { pickAlert, playAlert, readAlerts, saveAlerts, webAudioContext, type AlertPrefs, type ToneContext } from '../lib/alerts';
import { useBandFilter, visibleBands } from '../lib/bandFilter';
import { bandOf } from '../lib/format';
import { readLayout, saveLayout, type Layout, type PanelId } from '../lib/layout';
import { replyErrorMessage } from '../lib/replyError';
import { emitSpectrum, registerSpectrum } from '../lib/spectrumFeed';
import { browserStorage } from '../lib/theme';
import { cqDecodes } from '../state/selectors';
import { useStation } from '../state/station';

const F_MIN_HZ = 0;
const F_MAX_HZ = 3000;

/** Le decodifiche con i nostri Tx in mezzo, in ordine d'orario (le piu' recenti per prime), come nel vecchio programma. */
function withOwnTx(shown: DecodedMessage[], txLog: DecodedMessage[]): DecodedMessage[] {
  return txLog.length === 0 ? shown : [...txLog, ...shown].sort((a, b) => b.time_ms - a.time_ms);
}

export function Live() {
  const t = useT();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const txLog = useStation((s) => s.txLog);
  const connection = useStation((s) => s.connection);
  const chosenBands = useBandFilter((s) => s.bands);
  const filters = useFilters((s) => s.filters);
  const shown = applyFilters(decodes, filters, status?.own_continent);
  const [note, setNote] = useState('');
  // l'esito di un comando appare un attimo sopra la pagina e sparisce: non occupa spazio
  useEffect(() => {
    if (!note) return;
    const timer = setTimeout(() => setNote(''), 4000);
    return () => clearTimeout(timer);
  }, [note]);

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
      const picked = pickAlert(state.decodes, lastId, useNotify.getState().notify.sound, useStation.getState().status?.de_call);
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

  const titles: Record<PanelId, string> = {
    waterfall: t('panel.waterfall'),
    station: t('panel.station'),
    tx: t('panel.tx'),
    cq: t('panel.cq'),
    all: t('panel.all'),
    log: t('panel.log'),
    cluster: t('panel.cluster'),
  };

  const renderPanel = (id: PanelId) => {
    switch (id) {
      case 'waterfall':
        return (
          <>
            <div className="wf-stack">
              <Waterfall register={registerSpectrum} />
              <FrequencyMarkers
                fMinHz={F_MIN_HZ}
                fMaxHz={F_MAX_HZ}
                rxHz={status?.rx_df}
                txHz={status?.tx_df}
                onRx={(hz) => void setFrequency('Rx', hz, postRxFrequency)}
                onTx={(hz) => void setFrequency('Tx', hz, postTxFrequency)}
              />
            </div>
            <FrequencyRuler fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} decodes={decodes} />
          </>
        );
      case 'station':
        return <TxCommands txAllowed={info.data?.tx_allowed ?? false} status={status} />;
      case 'tx':
        return <TxPanel txAllowed={info.data?.tx_allowed ?? false} status={status} decodes={decodes} />;
      case 'cq':
        return (
          <DecodeList
            decodes={cqDecodes(shown)}
            myGrid={status?.de_grid}
            myCall={status?.de_call}
            dxCall={status?.dx_call}
            onReply={onReply}
            emptyText={t('decode.noCq')}
          />
        );
      case 'cluster':
        return <ClusterPanel />;
      case 'log':
        return <Log4omPanel />;
      case 'all':
        return <DecodeList decodes={withOwnTx(shown, txLog)} myGrid={status?.de_grid} myCall={status?.de_call} dxCall={status?.dx_call} onReply={onReply} />;
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

  const attempt = async (action: () => Promise<void>, done: string) => {
    try {
      await action();
      setNote(done);
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  const setFrequency = (which: string, hz: number, send: (hz: number) => Promise<void>) =>
    attempt(() => send(hz), t('note.freqSet', { which, hz }));
  const pickMode = (mode: string) =>
    attempt(async () => {
      await postMode(mode);
      useStation.getState().patchStatus({ mode });
    }, t('note.modeSet', { mode }));
  const pickBand = (band: string) =>
    attempt(async () => {
      const done = await postBand(band);
      if (done.dial_hz) useStation.getState().patchStatus({ dial_hz: done.dial_hz });
    }, t('note.bandSet', { band }));

  const onReply = useCallback(
    async (id: number) => {
      const target = useStation.getState().decodes.find((d) => d.id === id);
      await attempt(
        async () => {
          await postReply(id);
          // la stazione e' scelta: perche' JTDX la chiami da solo al momento giusto serve anche Abilita Tx
          // (con 1 QSO acceso JTDX lo spegne a ogni QSO finito); senza trasmissione aperta il server rifiuta e resta com'e'
          if (info.data?.tx_allowed) await postControl('enable_tx', true).catch(() => undefined);
          // la stazione chiamata compare subito nel blocco DX, prima che JTDX la riporti
          const call = target ? callOf(target.text) : null;
          if (call) useStation.getState().setPendingDx(call);
        },
        t('note.replySent', { text: target?.text ?? '' }),
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t, info.data?.tx_allowed],
  );

  return (
    <div className="live">
      <div className="live-top">
        <StatusBar status={status} connection={connection} />
        <div className="live-actions">
          <TxBanner status={status} />
          <AlertSwitch value={alerts} onChange={changeAlerts} />
          <StopButton variant="compact" />
        </div>
      </div>
      <div className="band-row">
        <ModeSwitch
          mode={status?.mode ?? ''}
          locked={status?.transmitting ?? false}
          onPick={info.data?.rig_control ? (mode) => void pickMode(mode) : undefined}
        />
        <BandBar
          bands={visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null)}
          dialHz={status?.dial_hz ?? 0}
          locked={status?.transmitting ?? false}
          onPick={info.data?.rig_control ? (band) => void pickBand(band) : undefined}
        />
      </div>
      <RxControls mode={status?.mode ?? ''} />
      <Dock layout={layout} onChange={changeLayout} titles={titles} render={renderPanel} />
      {note ? (
        <p className="live-toast" role="status" aria-live="polite">
          {note}
        </p>
      ) : null}
      <StopButton variant="bar" />
    </div>
  );
}
