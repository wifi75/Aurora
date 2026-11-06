import type { DecodedMessage } from '../api/schemas';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { getCat, getControls, getOnline, getStationInfo, postBand, postBandHopNext, postControl, postMode, postQueueReply, postReply, postRxFrequency, postTxFrequency } from '../api/client';
import { BandBar } from '../components/BandBar';
import { ClusterPanel } from '../components/ClusterPanel';
import { CqStrip } from '../components/CqStrip';
import { DecodeList } from '../components/DecodeList';
import { DiagnosisBar } from '../components/DiagnosisBar';
import { GridBoard } from '../components/GridBoard';
import { DetachButton } from '../components/DetachButton';
import { detachedItems, useGroups } from '../lib/detached';
import { useBoard, type BoardId } from '../lib/boardLayout';
import { DxMapPage } from './DxMapPage';
import type { ItemId } from '../lib/detached';
import { EDIT_LAYOUT } from '../lib/bottomLayout';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Log4omPanel, LogHeader } from '../components/Log4omPanel';
import { ModeSwitch } from '../components/ModeSwitch';
import { PhoneTabs } from '../components/PhoneTabs';
import { RadioDisplayHeader } from '../components/radio-display/RadioDisplayHeader';
import { TxCommands } from '../components/TxCommands';
import { RxControls } from '../components/RxControls';
import { StatusBar } from '../components/StatusBar';
import { SpotMarkers } from '../components/SpotMarkers';
import { StopButton } from '../components/StopButton';
import { DxpedPanel } from '../components/DxpedPanel';
import { foxCalls } from '../lib/fox';
import { usePrefs } from '../lib/prefs';
import { senderOf } from '../lib/wishlist';
import { TxPanel } from '../components/TxPanel';
import { DxccCard } from '../components/DxccCard';
import { subjectBand, subjectMode } from '../lib/dxccCard';
import { Waterfall } from '../components/Waterfall';
import { WaterfallToolbar } from '../components/WaterfallToolbar';
import { useT } from '../i18n';
import { applyFilters, callOf, useFilters } from '../lib/filters';
import { useAlertPrefs } from '../lib/alerts';
import { pickSoundEvent, ringState, soundFor } from '../lib/soundAssign';
import { armAudio, playSound } from '../lib/soundBank';
import { useBandFilter, visibleBands } from '../lib/bandFilter';
import { bandOf } from '../lib/format';
import { filterWindowHz, signalBandwidthHz } from '../lib/markers';
import { readLayout, type Layout, type PanelId } from '../lib/layout';
import { replyErrorMessage } from '../lib/replyError';
import { usePhone, useStationView } from '../lib/phone';
import { PhoneLive } from './PhoneLive';
import { useRadioDisplay } from '../lib/radioDisplay';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { browserStorage } from '../lib/theme';
import { applyCqPrefs, useCqPrefs } from '../lib/cqPrefs';
import { useStation } from '../state/station';
import { toast, useToasts } from '../lib/toast';


/** Le decodifiche con i nostri Tx in mezzo, in ordine d'orario (le piu' recenti per prime), come nel vecchio programma. */
function withOwnTx(shown: DecodedMessage[], txLog: DecodedMessage[]): DecodedMessage[] {
  return txLog.length === 0 ? shown : [...txLog, ...shown].sort((a, b) => b.time_ms - a.time_ms);
}

export function Live({ soloPanel }: { soloPanel?: PanelId } = {}) {
  const t = useT();
  const phone = usePhone();
  const view = useStationView();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const txLog = useStation((s) => s.txLog);
  const pendingBand = useStation((s) => s.pendingBand);
  const connection = useStation((s) => s.connection);
  const displayHidden = useRadioDisplay((s) => s.pref.hidden);
  const chosenBands = useBandFilter((s) => s.bands);
  const filters = useFilters((s) => s.filters);
  const cqPrefs = useCqPrefs((s) => s.prefs);
  const shown = applyFilters(decodes, filters, status?.own_continent);
  const alerts = useAlertPrefs((s) => s.prefs);


  // avvisi gia' accesi da una visita precedente: l'audio si abilita al primo tocco sulla pagina
  useEffect(() => {
    if (!alerts.enabled) return;
    window.addEventListener('pointerdown', armAudio, { once: true });
    return () => window.removeEventListener('pointerdown', armAudio);
  }, [alerts.enabled]);

  // suona solo per i messaggi arrivati dopo l'apertura della pagina, con il suono assegnato alla funzione (senza suono assegnato tace)
  useEffect(() => {
    let lastId: number | null = null;
    let lastTx: number | null | undefined;
    return useStation.subscribe((state) => {
      const transmitting = state.status?.transmitting ?? false;
      const st = ringState(transmitting);
      const picked = pickSoundEvent(state.decodes, lastId, st, state.status?.de_call, state.status?.dx_call);
      lastId = picked.lastId;
      if (picked.sound) playSound(picked.sound, st.volume);
      // «Mio messaggio in trasmissione»: una riga Tx nuova
      const txTop = state.txLog[0]?.id ?? null;
      if (lastTx !== undefined && txTop !== null && txTop !== lastTx) {
        const id = soundFor('tx_msg', st);
        if (id) playSound(id, st.volume);
      }
      lastTx = txTop;
    });
  }, []);

  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  // la cascata mostra quello che passa dal filtro della radio (letto con il CAT), 0-3000 Hz se non si sa
  const passbandHz = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 }).data?.status.rig?.passband_hz;
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const wf = useWfPrefs((s) => s.prefs);
  // il tratto mostrato: da «Start» del Wide Graph fino all'ampiezza del filtro della radio
  const { from: F_MIN_HZ, to: F_MAX_HZ } = visibleRange(wf.startHz, spanHz);
  useEffect(() => {
    useSpectrumSpan.getState().set(spanFromPassband(passbandHz));
  }, [passbandHz]);
  const dxpedOn = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false, staleTime: 60_000 }).data?.dxped === true;
  const prefs = usePrefs((s) => s.prefs);
  // i nominativi sentiti negli ultimi messaggi: una DXpedition in onda e sentita adesso va in cima
  const heardCalls = decodes.slice(0, 300).map((d) => senderOf(d.text)?.call ?? '').filter(Boolean);
  const switches = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false }).data;
  const filterWindow = switches?.filter === true ? filterWindowHz(status?.mode, switches.hound === true) : null;
  const [layout] = useState<Layout>(() => readLayout(browserStorage()));

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
            <WaterfallToolbar />
            <div className="wf-stack">
              <Waterfall register={registerSpectrum} mode={status?.mode} band={status ? (bandOf(status.dial_hz) ?? undefined) : undefined} />
              <FrequencyMarkers
                fMinHz={F_MIN_HZ}
                fMaxHz={F_MAX_HZ}
                rxHz={status?.rx_df}
                txHz={status?.tx_df}
                bandwidthHz={signalBandwidthHz(status?.mode)}
                filter={filterWindow}
                showBars={wf.bars}
                showFreq={wf.freq}
                onRx={(hz) => void setFrequency('Rx', hz, postRxFrequency)}
                onTx={(hz) => void setFrequency('Tx', hz, postTxFrequency)}
              />
              <SpotMarkers fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} dialHz={status?.dial_hz} onPick={(hz) => void setFrequency('Rx', hz, postRxFrequency)} />
            </div>
            <FrequencyRuler fMinHz={F_MIN_HZ} fMaxHz={F_MAX_HZ} decodes={decodes} rxHz={status?.rx_df} txHz={status?.tx_df} bandwidthHz={signalBandwidthHz(status?.mode)} />
          </>
        );
      case 'station':
        return <TxCommands txAllowed={info.data?.tx_allowed ?? false} status={status} />;
      case 'tx':
        return <TxPanel txAllowed={info.data?.tx_allowed ?? false} status={status} decodes={decodes} />;
      case 'cq':
        {
          const cqList = <CqStrip decodes={applyCqPrefs(shown, cqPrefs)} onReply={onReply} onQueue={onQueue} emptyText={t('decode.noCq')} />;
          // con il calendario NG3K acceso il pannello si divide: CQ a sinistra, DXpedition a destra
          if (!dxpedOn) return cqList;
          return (
            <div className="cq-split">
              <div className="cq-split-main">{cqList}</div>
              <DxpedPanel heardCalls={heardCalls} foxCalls={[...foxCalls(decodes.slice(0, 300))]} keepDays={prefs.dxpedDays} />
            </div>
          );
        }
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
        return <DecodeList decodes={withOwnTx(shown, txLog)} myGrid={status?.de_grid} myCall={status?.de_call} dxCall={status?.dx_call} dxGrid={status?.dx_grid} onReply={onReply} onQueue={onQueue} />;
    }
  };

  const attempt = async (action: () => Promise<void>, done: string | null) => {
    // l'avviso compare subito, al clic, senza aspettare la risposta del motore; se il comando fallisce diventa un errore
    const shown = done ? toast(done, 'ok') : null;
    try {
      await action();
    } catch (error) {
      if (shown !== null) useToasts.getState().dismiss(shown);
      toast(replyErrorMessage(error, t), 'error');
    }
  };

  const setFrequency = (_which: string, hz: number, send: (hz: number) => Promise<void>) =>
    // spostando le barre si mandano tante frequenze di fila: nessun avviso se va bene (la barra si vede gia' spostata), solo gli errori
    attempt(() => send(hz), null);
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

  const client = useQueryClient();
  const onReply = useCallback(
    async (id: number) => {
      const target = useStation.getState().decodes.find((d) => d.id === id);
      // la stazione chiamata compare subito nel blocco DX e nella barra, senza aspettare che il motore la riporti
      const call = target ? callOf(target.text) : null;
      if (call) useStation.getState().setPendingDx(call);
      await attempt(
        async () => {
          // la stazione e' scelta: perche' il motore la chiami serve anche Abilita Tx. Prima la risposta (il motore sceglie il messaggio e il periodo),
          // poi Abilita Tx: in parallelo poteva arrivare per primo, a stazione ancora non scelta. Se il comando fallisce lo si dice, non si ignora.
          await postReply(id);
          if (info.data?.tx_allowed !== false) {
            await postControl('enable_tx', true);
            client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, enable_tx: true }));
            void client.invalidateQueries({ queryKey: ['controls'] });
          }
        },
        t('note.replySent', { call: call ?? target?.text ?? '' }),
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t, info.data?.tx_allowed, client],
  );

  // tasto destro su una stazione: in coda per il prossimo QSO (subito, se non ce n'e' uno in corso); di nuovo la toglie
  const onQueue = useCallback(
    async (id: number) => {
      const target = useStation.getState().decodes.find((d) => d.id === id);
      await attempt(async () => {
        await postQueueReply(id);
      }, t('note.queued', { text: target?.text ?? '' }));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t],
  );

  // sotto i 768 px (o su un tablet in orizzontale, a due colonne): il telecomando compatto, con gli stessi dati e comandi di questa pagina
  // finestra a parte: solo il pannello chiesto, a tutta finestra
  if (soloPanel) {
    return <div className={`solo-panel dock-panel dock-panel--${soloPanel}`}><div className="dock-body">{renderPanel(soloPanel)}</div></div>;
  }

  if (view !== 'desktop') {
    return (
      <PhoneLive
        tablet={view === 'tablet'}
        waterfall={
          <div className="wf-stack">
            <Waterfall register={registerSpectrum} mode={status?.mode} spectrumPx={22} />
            <FrequencyMarkers
              fMinHz={F_MIN_HZ}
              fMaxHz={F_MAX_HZ}
              rxHz={status?.rx_df}
              txHz={status?.tx_df}
              bandwidthHz={signalBandwidthHz(status?.mode)}
              filter={filterWindow}
              showBars
              showFreq={false}
              onRx={(hz) => void setFrequency('Rx', hz, postRxFrequency)}
              onTx={(hz) => void setFrequency('Tx', hz, postTxFrequency)}
            />
          </div>
        }
        decodes={withOwnTx(shown, txLog)}
        onReply={(id) => void onReply(id)}
        onQueue={(id) => void onQueue(id)}
        pickBand={(band) => void pickBand(band)}
        pickMode={(mode) => void pickMode(mode)}
        rigControl={info.data?.rig_control === true}
        modeControl={info.data?.rig_control === true || info.data?.engine === 'native'}
        txAllowed={info.data?.tx_allowed ?? false}
      />
    );
  }

  // la pagina a blocchi: ogni finestra ha la sua posizione e la sua misura (lib/boardLayout.ts) e, in fase di progetto (`EDIT_LAYOUT`),
  // si sposta dalla barra del titolo e si ridimensiona dall'angolo in basso a destra; poi le misure salvate restano fisse.
  const boardCells = useBoard((st) => st.cells);
  const blockBody = (id: BoardId): ReactNode =>
    id === 'dxmap' ? (
      <DxMapPage solo bare />
    ) : id === 'tx' ? (
      <TxPanel part="combo" txAllowed={info.data?.tx_allowed ?? false} status={status} decodes={decodes} />
    ) : (
      renderPanel(id)
    );
  const blockTitle = (id: BoardId): string => (id === 'dxmap' ? t('nav.dxmap') : titles[id]);
  const blockExtra = (id: BoardId): ReactNode => (id === 'log' ? <LogHeader /> : id === 'cq' && dxpedOn ? (
    <div className="dock-grip-extra">
      <h2>{t('dxp.title')}</h2>
      <span className="muted">NG3K</span>
    </div>
  ) : null);
  // le finestre staccate sul secondo schermo non si vedono qui (la Trasmissione porta con se' anche il QSO); idem quelle nascoste dalle Impostazioni
  const groups = useGroups();
  const awayIds: PanelId[] = [...(layout.hidden ?? []), ...(detachedItems(groups) as PanelId[])];
  const visibleCells = Object.fromEntries(Object.entries(boardCells).filter(([id]) => !(awayIds as string[]).includes(id)));

  return (
    <div className="live">
      <div className="live-top">
        {/* a sinistra modi, bande e controlli di ricezione una sotto l'altra, vicini; a destra il display della radio */}
        <div className="live-left">
        <div className="live-left-row">
        <div className="band-row">
          <ModeSwitch
            mode={status?.mode ?? ''}
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control || info.data?.engine === 'native' ? (mode) => void pickMode(mode) : undefined}
          />
          <BandBar
            bands={visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null)}
            dialHz={status?.dial_hz ?? 0}
            pendingBand={pendingBand}
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control ? (band) => void pickBand(band) : undefined}
            onHop={info.data?.rig_control ? () => void attempt(() => postBandHopNext(), t('note.bandHop')) : undefined}
          />
        </div>
        <StatusBar status={status} connection={connection} hideDial={!displayHidden} />
        </div>
        {phone ? null : (
          <div className="live-left-line">
            <RxControls
              mode={status?.mode ?? ''}
              transmitting={status?.transmitting ?? false}
            />
          </div>
        )}
        </div>
        <RadioDisplayHeader />
      </div>
      <DiagnosisBar />
      {phone ? (
        <PhoneTabs
          titles={titles}
          render={renderPanel}
          extra={{
            station: (
              <>
                <RxControls mode={status?.mode ?? ''} transmitting={status?.transmitting ?? false} />
              </>
            ),
          }}
        />
      ) : (
        <div className="live-board">
          <GridBoard
            cells={visibleCells}
            locked={!EDIT_LAYOUT}
            free
            onChange={(next) => useBoard.getState().set({ ...boardCells, ...next })}
            render={(id, grip) => (
              <section className={`dock-panel dock-panel--${id} grid-panel`} data-panel={id}>
                <header className={`dock-grip${EDIT_LAYOUT ? '' : ' dock-grip--fixed'}`} onPointerDown={grip.onPointerDown}>
                  <span className="dock-grip-dots" aria-hidden="true" />
                  <h2>{blockTitle(id as BoardId)}</h2>
                  {blockExtra(id as BoardId)}
                  <DetachButton item={id as ItemId} className="dock-detach" />
                </header>
                <div className="dock-body">{blockBody(id as BoardId)}</div>
              </section>
            )}
          />
        </div>
      )}
      <StopButton variant="bar" />
    </div>
  );
}
