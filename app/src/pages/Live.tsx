import type { DecodedMessage } from '../api/schemas';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { FreqPicker } from '../components/FreqPicker';
import { getCat, getControls, getOnline, getStationInfo, postBand, postBandHopNext, postControl, postFrequency, postMode, postQueueReply, postReply, postRxFrequency, postTxFrequency } from '../api/client';
import { ClusterPanel } from '../components/ClusterPanel';
import { CqStrip } from '../components/CqStrip';
import { DecodeList } from '../components/DecodeList';
import { DiagnosisBar } from '../components/DiagnosisBar';
import { GridBoard, type Cell } from '../components/GridBoard';
import { DetachButton } from '../components/DetachButton';
import { detachedItems, useGroups } from '../lib/detached';
import { useBoard, type BoardId } from '../lib/boardLayout';
import { BandsOpen } from '../components/BandsOpen';
import type { ItemId } from '../lib/detached';
import { EDIT_LAYOUT } from '../lib/bottomLayout';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { Log4omPanel, LogHeader } from '../components/Log4omPanel';
import { ModeSwitch, q65Parts } from '../components/ModeSwitch';
import { Q65Extras } from '../components/Q65Extras';
import { PhoneTabs } from '../components/PhoneTabs';
import { RadioDisplayHeader } from '../components/radio-display/RadioDisplayHeader';
import { TxCommands } from '../components/TxCommands';
import { RxControls } from '../components/RxControls';
import { NewDxAlert } from '../components/NewDxAlert';
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
import { useLayout, type PanelId } from '../lib/layout';
import { replyErrorMessage } from '../lib/replyError';
import { usePhone, useStationView } from '../lib/phone';
import { PhoneLive } from './PhoneLive';
import { useRadioDisplay } from '../lib/radioDisplay';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { browserStorage } from '../lib/theme';
import { applyCqPrefs, useCqPrefs } from '../lib/cqPrefs';
import { useStation, useStatusPick } from '../state/station';
import { toast, useToasts } from '../lib/toast';


/** Le decodifiche con i nostri Tx in mezzo, in ordine d'orario (le piu' recenti per prime), come nel vecchio programma. */
function withOwnTx(shown: DecodedMessage[], txLog: DecodedMessage[]): DecodedMessage[] {
  return txLog.length === 0 ? shown : [...txLog, ...shown].sort((a, b) => b.time_ms - a.time_ms);
}

export function Live({ soloPanel }: { soloPanel?: PanelId } = {}) {
  const t = useT();
  const phone = usePhone();
  const view = useStationView();
  const status = useStatusPick(['de_call', 'de_grid', 'dial_hz', 'dx_call', 'dx_grid', 'mode', 'own_continent', 'rx_df', 'transmitting', 'tx_df', 'tx_enabled']);
  const decodes = useStation((s) => s.decodes);
  const txLog = useStation((s) => s.txLog);
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

  // finche' la radio non risulta comandabile (all'avvio si collega dopo qualche secondo, o dopo «Accendi») si richiede ogni 3 s:
  // senza, il menu delle frequenze restava senza comandi fino alla riapertura della finestra
  const info = useQuery({
    queryKey: ['station-info'],
    queryFn: getStationInfo,
    retry: false,
    refetchInterval: (query) => (query.state.data?.rig_control === true ? false : 3_000),
  });
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
  const layout = useLayout();

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
        return <TxCommands txAllowed={info.data?.tx_allowed ?? false} />;
      case 'tx':
        return <TxPanel txAllowed={info.data?.tx_allowed ?? false} decodes={decodes} />;
      case 'cq':
        {
          const cqList = <CqStrip decodes={applyCqPrefs(shown, cqPrefs)} onReply={onReply} onQueue={onQueue} emptyText={t('decode.noCq')} />;
          // con il calendario NG3K acceso il pannello si divide: CQ a sinistra, DXpedition a destra
          if (!dxpedOn) return cqList;
          return (
            <div className="cq-split">
              <div className="cq-split-main">{cqList}</div>
              <DxpedPanel onCall={(id) => void onReply(id)} heardCalls={heardCalls} foxCalls={[...foxCalls(decodes.slice(0, 300))]} keepDays={prefs.dxpedDays} soonDays={120} />
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
        rigControl={info.data?.rig_control === true || info.data?.manual_dial === true}
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
      <div className="bands-with-cmds bands-only">
        <BandsOpen />
      </div>
    ) : id === 'cmds' ? (
      // i Comandi in una finestra a se', sotto Bande aperte (richiesta del proprietario)
      <div className="bands-with-cmds cmds-only">
        <section className="bands-cmds" aria-label={t('panel.station')}>
          <TxCommands txAllowed={info.data?.tx_allowed ?? false} />
        </section>
      </div>
    ) : id === 'tx' ? (
      <TxPanel part="combo" txAllowed={info.data?.tx_allowed ?? false} decodes={decodes} />
    ) : (
      renderPanel(id)
    );
  const blockTitle = (id: BoardId): string => (id === 'dxmap' ? t('bands.title') : id === 'cmds' ? t('panel.station') : titles[id]);
  const blockExtra = (id: BoardId): ReactNode => (id === 'log' ? <LogHeader /> : id === 'cq' && dxpedOn ? (
    <div className="dock-grip-extra">
      <h2>{t('dxp.title')}</h2>
      <span className="muted">NG3K</span>
    </div>
  ) : null);
  // le finestre staccate sul secondo schermo non si vedono qui (la Trasmissione porta con se' anche il QSO); idem quelle nascoste dalle Impostazioni
  const groups = useGroups();
  // il blocco `dxmap` della Stazione e' Bande aperte: staccato diventa `bands`; la Mappa DX staccata (`dxmap`) non lo toglie da qui
  const awayIds: PanelId[] = [
    ...(layout.hidden ?? []),
    ...(detachedItems(groups).filter((i) => i !== 'dxmap').map((i) => (i === 'bands' ? 'dxmap' : i)) as PanelId[]),
  ];
  const visibleCells = Object.fromEntries(Object.entries(boardCells).filter(([id]) => !(awayIds as string[]).includes(id)));
  // con una o piu' schede staccate (o nascoste) la pagina si sblocca: le altre si spostano e si ridimensionano per usare il posto libero.
  // Queste misure si ricordano a parte, una disposizione per ogni insieme di schede staccate: rimesse tutte a posto, torna la pagina
  // bloccata con le misure di sempre (richiesta del proprietario)
  const awayKey = [...new Set(awayIds as string[])].filter((id) => id in boardCells).sort().join(',');
  const [awayBoards, setAwayBoards] = useState<Record<string, Record<string, Cell>>>(readAwayBoards);
  // senza misure salvate per questo insieme, le finestre vicine si allargano da sole a riempire il posto lasciato libero
  const awayCells = !awayKey
    ? visibleCells
    : awayBoards[awayKey]
      ? { ...visibleCells, ...pickCells(awayBoards[awayKey], visibleCells) }
      : fillGaps(visibleCells, Object.entries(boardCells).filter(([id]) => (awayIds as string[]).includes(id)).map(([, c]) => c));
  const saveAwayCells = (next: Record<string, Cell>) => {
    const all = { ...awayBoards, [awayKey]: { ...awayCells, ...next } };
    setAwayBoards(all);
    try {
      browserStorage()?.setItem(AWAY_KEY, JSON.stringify(all));
    } catch {
      // senza archivio le misure valgono finche' la finestra resta aperta
    }
  };

  return (
    <div className="live">
      <div className="live-top" ref={alignDisplay}>
        {/* a sinistra modi, bande e controlli di ricezione una sotto l'altra, vicini; a destra il display della radio */}
        <div className="live-left">
        <div className="live-left-row">
        <div className="band-row">
          <ModeSwitch
            mode={status?.mode ?? ''}
            // il modo non si cambia mentre si trasmette (anche il motore lo rifiuta); con Abilita Tx acceso ma in ricezione si puo'
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control || info.data?.engine === 'native' ? (mode) => void pickMode(mode) : undefined}
          />
          {info.data?.engine === 'native' && q65Parts(status?.mode ?? '') ? <Q65Extras disabled={status?.transmitting ?? false} /> : null}
          <FreqPicker
            bands={visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null)}
            dialHz={status?.dial_hz ?? 0}
            mode={status?.mode ?? ''}
            locked={status?.transmitting ?? false}
            onPick={info.data?.rig_control || info.data?.manual_dial ? (hz) => void attempt(async () => void (await postFrequency(hz)), null) : undefined}
            onHop={info.data?.rig_control ? () => void attempt(() => postBandHopNext(), t('note.bandHop')) : undefined}
          />
        </div>
        <StatusBar connection={connection} hideDial={!displayHidden} />
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
      {phone ? null : <NewDxAlert onReply={(id) => void onReply(id)} />}
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
            cells={awayKey ? awayCells : visibleCells}
            locked={!EDIT_LAYOUT && !awayKey}
            swap
            free
            onChange={(next) => (awayKey && !EDIT_LAYOUT ? saveAwayCells(next) : useBoard.getState().set({ ...boardCells, ...next }))}
            render={(id, grip) => (
              <section className={`dock-panel dock-panel--${id} grid-panel`} data-panel={id}>
                <header className={`dock-grip${EDIT_LAYOUT ? '' : ' dock-grip--swap'}`} onPointerDown={grip.onPointerDown}>
                  <span className="dock-grip-dots" aria-hidden="true" />
                  <h2>{blockTitle(id as BoardId)}</h2>
                  {blockExtra(id as BoardId)}
                  {id === 'cmds' ? null : <DetachButton item={(id === 'dxmap' ? 'bands' : id) as ItemId} className="dock-detach" />}
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

/** Le disposizioni della Stazione con schede staccate, una per ogni insieme (chiave: gli id staccati in ordine, separati da virgola). */
const AWAY_KEY = 'aurora-board-staccate';

function readAwayBoards(): Record<string, Record<string, Cell>> {
  try {
    const raw = browserStorage()?.getItem(AWAY_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : null;
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, Record<string, Cell>>) : {};
  } catch {
    return {};
  }
}

/** Solo le misure valide delle schede ancora in vista. */
/**
 * Riempie i buchi lasciati dalle finestre nascoste o staccate: per ogni buco si allargano le finestre che lo toccano su un lato e lo coprono
 * esattamente da un capo all'altro (prima a sinistra, poi a destra, sopra, sotto). Cosi' nascondendo per esempio Log e DX Cluster, Attivita' di
 * banda si allarga al posto del Log e Bande aperte con Comandi al posto del DX Cluster. Un buco che nessuno copre esattamente resta vuoto.
 */
export function fillGaps(cells: Record<string, Cell>, holes: readonly Cell[]): Record<string, Cell> {
  const out: Record<string, Cell> = { ...cells };
  const eq = (a: number, b: number) => Math.abs(a - b) < 0.01;
  const pending = [...holes];
  for (let round = 0; round < 4 && pending.length > 0; round++) {
    for (let i = pending.length - 1; i >= 0; i--) {
      const h = pending[i]!;
      const list = Object.entries(out);
      // le finestre su un lato del buco che, messe insieme, lo coprono da un capo all'altro senza uscire
      const side = (touch: (c: Cell) => boolean, from: (c: Cell) => number, to: (c: Cell) => number, start: number, end: number) => {
        const g = list.filter(([, c]) => touch(c) && from(c) >= start - 0.01 && to(c) <= end + 0.01).sort((a, b) => from(a[1]) - from(b[1]));
        if (g.length === 0 || !eq(from(g[0]![1]), start) || !eq(to(g[g.length - 1]![1]), end)) return null;
        for (let k = 1; k < g.length; k++) if (!eq(from(g[k]![1]), to(g[k - 1]![1]))) return null;
        return g.map(([id]) => id);
      };
      const y0 = h.y, y1 = h.y + h.h, x0 = h.x, x1 = h.x + h.w;
      const left = side((c) => eq(c.x + c.w, x0), (c) => c.y, (c) => c.y + c.h, y0, y1);
      const right = left ? null : side((c) => eq(c.x, x1), (c) => c.y, (c) => c.y + c.h, y0, y1);
      const above = left || right ? null : side((c) => eq(c.y + c.h, y0), (c) => c.x, (c) => c.x + c.w, x0, x1);
      const below = left || right || above ? null : side((c) => eq(c.y, y1), (c) => c.x, (c) => c.x + c.w, x0, x1);
      if (left) for (const id of left) out[id] = { ...out[id]!, w: out[id]!.w + h.w };
      else if (right) for (const id of right) out[id] = { ...out[id]!, x: out[id]!.x - h.w, w: out[id]!.w + h.w };
      else if (above) for (const id of above) out[id] = { ...out[id]!, h: out[id]!.h + h.h };
      else if (below) for (const id of below) out[id] = { ...out[id]!, y: out[id]!.y - h.h, h: out[id]!.h + h.h };
      else continue;
      pending.splice(i, 1);
    }
  }
  return out;
}

function pickCells(saved: Record<string, Cell> | undefined, shown: Record<string, Cell>): Record<string, Cell> {
  if (!saved) return {};
  const ok = (c: Cell | undefined): c is Cell => !!c && [c.x, c.y, c.w, c.h].every((v) => typeof v === 'number' && Number.isFinite(v)) && c.w > 0 && c.h > 0;
  return Object.fromEntries(Object.keys(shown).filter((id) => ok(saved[id])).map((id) => [id, saved[id]!]));
}

/**
 * Il display in alto a destra si allinea al bordo destro del pannello piu' a destra della prima fila (con la pagina a blocchi i pannelli
 * hanno misure fisse e non arrivano al bordo della finestra). La distanza si scrive nella variabile `--rd-right` della riga e si ricalcola
 * quando cambia la misura della finestra o dei blocchi.
 */
let alignStop: (() => void) | null = null;

function alignDisplay(row: HTMLDivElement | null): void {
  alignStop?.();
  alignStop = null;
  if (!row) return;
  const page = row.parentElement;
  if (!page) return;
  const measure = () => {
    const top = row.getBoundingClientRect();
    // larghezza del display (sta sopra la riga, a destra): modo, frequenza e interruttori si fermano prima di lui
    const display = row.lastElementChild instanceof HTMLElement && getComputedStyle(row.lastElementChild).position === 'absolute' ? row.lastElementChild : null;
    const width = `${display ? Math.round(display.getBoundingClientRect().width) : 0}px`;
    if (row.style.getPropertyValue('--rd-w') !== width) row.style.setProperty('--rd-w', width);
    const panels = [...page.querySelectorAll<HTMLElement>('.grid-block, .dock-panel')].map((p) => p.getBoundingClientRect()).filter((r) => r.width > 0 && r.top > top.bottom - 1);
    if (panels.length === 0) {
      row.style.removeProperty('--rd-right');
      return;
    }
    const firstTop = Math.min(...panels.map((r) => r.top));
    const right = Math.max(...panels.filter((r) => r.top < firstTop + 4).map((r) => r.right));
    const value = `${Math.max(0, Math.round(top.right - right))}px`;
    // si riscrive solo se cambia: a ogni controllo periodico non si ridisegna niente
    if (row.style.getPropertyValue('--rd-right') !== value) row.style.setProperty('--rd-right', value);
  };
  measure();
  const ro = new ResizeObserver(measure);
  ro.observe(page);
  page.querySelectorAll('.grid-board, .dock').forEach((el) => ro.observe(el));
  // i blocchi si spostano anche senza cambiare la misura della pagina: si ricontrolla ogni tanto
  const timer = window.setInterval(measure, 1500);
  alignStop = () => {
    ro.disconnect();
    window.clearInterval(timer);
  };
}
