import type { DecodedMessage } from '../api/schemas';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getEngineConfig, getStationInfo, postBand, postControl, postMode, postReply, postRxFrequency, postTxFrequency } from '../api/client';
import { AlertSwitch } from '../components/AlertSwitch';
import { BandBar } from '../components/BandBar';
import { ClusterPanel } from '../components/ClusterPanel';
import { DecodeList } from '../components/DecodeList';
import { DiagnosisBar } from '../components/DiagnosisBar';
import { Dock } from '../components/Dock';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Log4omPanel } from '../components/Log4omPanel';
import { ModeSwitch } from '../components/ModeSwitch';
import { PhoneTabs } from '../components/PhoneTabs';
import { RadioDisplayHeader } from '../components/radio-display/RadioDisplayHeader';
import { TxCommands } from '../components/TxCommands';
import { RxControls } from '../components/RxControls';
import { StatusBar } from '../components/StatusBar';
import { SpotMarkers } from '../components/SpotMarkers';
import { StopButton } from '../components/StopButton';
import { TxBanner } from '../components/TxBanner';
import { TxPanel } from '../components/TxPanel';
import { DxccCard } from '../components/DxccCard';
import { subjectBand, subjectMode } from '../lib/dxccCard';
import { Waterfall } from '../components/Waterfall';
import { useT } from '../i18n';
import { applyFilters, callOf, useFilters } from '../lib/filters';
import { useNotify } from '../lib/notify';
import { pickAlert, playAlert, readAlerts, saveAlerts, webAudioContext, type AlertPrefs, type ToneContext } from '../lib/alerts';
import { replyAccepted } from '../lib/replyMode';
import { useBandFilter, visibleBands } from '../lib/bandFilter';
import { bandOf } from '../lib/format';
import { signalBandwidthHz } from '../lib/markers';
import { readLayout, saveLayout, type Layout, type PanelId } from '../lib/layout';
import { replyErrorMessage } from '../lib/replyError';
import { usePhone } from '../lib/phone';
import { useRadioDisplay } from '../lib/radioDisplay';
import { registerSpectrum } from '../lib/spectrumFeed';
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
  const phone = usePhone();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const txLog = useStation((s) => s.txLog);
  const pendingBand = useStation((s) => s.pendingBand);
  const connection = useStation((s) => s.connection);
  const displayHidden = useRadioDisplay((s) => s.pref.hidden);
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

  const engineConfig = useQuery({ queryKey: ['engine-config'], queryFn: getEngineConfig, retry: false, staleTime: 30_000 });
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
                bandwidthHz={signalBandwidthHz(status?.mode)}
                onRx={(hz) => void setFrequency('Rx', hz, postRxFrequency)}
                onTx={(hz) => void setFrequency('Tx', hz, postTxFrequency)}
              />
              <SpotMarkers fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} dialHz={status?.dial_hz} onPick={(hz) => void setFrequency('Rx', hz, postRxFrequency)} />
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
        // il log in alto e, sotto, la scheda DXCC: lo stesso pannello diviso in due
        return (
          <div className="log-dxcc">
            <div className="log-dxcc-top">
              <Log4omPanel />
            </div>
            <div className="log-dxcc-bottom">
              <DxccCard panel call={status?.dx_call ?? ''} grid={status?.dx_grid ?? undefined} band={subjectBand(status?.dial_hz)} mode={subjectMode(status?.mode)} />
            </div>
          </div>
        );
      case 'all':
        return <DecodeList decodes={withOwnTx(shown, txLog)} myGrid={status?.de_grid} myCall={status?.de_call} dxCall={status?.dx_call} onReply={onReply} />;
    }
  };

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
  // il pulsante scelto si accende subito e resta cosi' finche' il motore non conferma; se il comando fallisce torna com'era
  const pickMode = (mode: string) => {
    const before = useStation.getState().status?.mode;
    useStation.getState().patchStatus({ mode });
    return attempt(async () => {
      try {
        await postMode(mode);
      } catch (error) {
        if (before !== undefined) useStation.getState().revertStatus(['mode'], { mode: before });
        throw error;
      }
    }, t('note.modeSet', { mode }));
  };
  const pickBand = (band: string) => {
    const store = useStation.getState();
    store.setPendingBand(band);
    return attempt(async () => {
      try {
        const done = await postBand(band);
        if (done.dial_hz) useStation.getState().patchStatus({ dial_hz: done.dial_hz });
      } catch (error) {
        useStation.getState().setPendingBand(null);
        throw error;
      }
    }, t('note.bandSet', { band }));
  };

  const onReply = useCallback(
    async (id: number) => {
      const target = useStation.getState().decodes.find((d) => d.id === id);
      // la stazione chiamata compare subito nel blocco DX e nella barra, senza aspettare che il motore la riporti
      const call = target ? callOf(target.text) : null;
      if (call) useStation.getState().setPendingDx(call);
      await attempt(
        async () => {
          // la stazione e' scelta: perche' il motore la chiami da solo al momento giusto serve anche Abilita Tx
          // (con 1 QSO acceso il motore lo spegne a ogni QSO finito); senza trasmissione aperta il server rifiuta e resta com'e'.
          // Le due richieste partono insieme: ogni attimo conta, la trasmissione parte solo se Abilita Tx e' acceso in tempo.
          const arm = info.data?.tx_allowed ? postControl('enable_tx', true).catch(() => undefined) : Promise.resolve();
          await postReply(id);
          await arm;
        },
        // se il motore ignorera' quel tipo di messaggio lo si dice, invece di non fare nulla in silenzio
        replyAccepted(engineConfig.data?.AcceptUDPReplyMessages, target?.text ?? '')
          ? t('note.replySent', { text: target?.text ?? '' })
          : `${t('note.replySent', { text: target?.text ?? '' })} ${t('note.replyMode')}`,
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t, info.data?.tx_allowed, engineConfig.data?.AcceptUDPReplyMessages],
  );

  return (
    <div className="live">
      <div className="live-top">
        <div className="band-row">
          <ModeSwitch
            mode={status?.mode ?? ''}
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control ? (mode) => void pickMode(mode) : undefined}
          />
          <BandBar
            bands={visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null)}
            dialHz={status?.dial_hz ?? 0}
            pendingBand={pendingBand}
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control ? (band) => void pickBand(band) : undefined}
          />
        </div>
        {/* il display sta a destra della riga delle bande, al posto dei controlli che ora sono sulla seconda riga */}
        <RadioDisplayHeader />
        <StatusBar status={status} connection={connection} hideDial={!displayHidden} />
        {phone ? (
          <div className="live-actions">
            <TxBanner status={status} />
          </div>
        ) : null}
      </div>
      {phone ? null : (
        <RxControls
          mode={status?.mode ?? ''}
          transmitting={status?.transmitting ?? false}
          trailing={
            <>
              <TxBanner status={status} />
              <AlertSwitch value={alerts} onChange={changeAlerts} />
              <StopButton variant="compact" />
            </>
          }
        />
      )}
      <DiagnosisBar />
      {phone ? (
        <PhoneTabs
          titles={titles}
          render={renderPanel}
          extra={{
            station: (
              <>
                <RxControls mode={status?.mode ?? ''} transmitting={status?.transmitting ?? false} />
                <AlertSwitch value={alerts} onChange={changeAlerts} />
              </>
            ),
          }}
        />
      ) : (
        <Dock layout={layout} onChange={changeLayout} titles={titles} render={renderPanel} />
      )}
      {note ? (
        <p className="live-toast" role="status" aria-live="polite">
          {note}
        </p>
      ) : null}
      <StopButton variant="bar" />
    </div>
  );
}
