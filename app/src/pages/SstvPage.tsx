import { Icon } from '../components/Icon';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getSstv, getSstvImageBlob, getSstvImages, postFrequency, sstvRx, sstvRxPage, sstvTune, type SstvImageEntry } from '../api/client';
import { FreqPicker } from '../components/FreqPicker';
import { gotoTextMode } from '../lib/digiGoto';
import { useListenWhileOpen } from '../lib/listenWhileOpen';
import { bandPlanRows } from '../lib/bandplan';
import { useBandFilter, visibleBands } from '../lib/bandFilter';
import { bandOf } from '../lib/format';
import { useStation } from '../state/station';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { Composer, type IncomingImage } from '../components/sstv/Composer';
import { SstvLogCard } from '../components/sstv/LogCard';
import { History } from '../components/sstv/History';
import { ReceiveView } from '../components/sstv/ReceiveView';
import { SstvTuning } from '../components/sstv/SstvTuning';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { retuneForSstv } from '../lib/modeRetune';
import { toast } from '../lib/toast';
import { REMOTE_HOST } from '../lib/phone';
import { ListenGate } from '../components/ListenGate';
import { openSettingsWindow } from '../lib/settingsWindow';
import { onSstv } from '../lib/sstvBus';
import '../styles/sstv.css';

/** La pagina SSTV: ricezione o compositore nell'area principale, sotto le cronologie delle ricevute e delle trasmesse. */
/** Sotto questo livello (dB sul fondo scala) l'ingresso e' muto: radio spenta o scheda audio senza segnale. */
const SILENT_LEVEL_DB = -95;

export function SstvPage({ solo = false }: { solo?: boolean } = {}) {
  const t = useT();
  const groups = useGroups();
  const elsewhere = groups.some((g) => g.open && g.items.includes('sstv'));
  // entrando nella pagina SSTV la radio va sui 20 m (14,230 MHz), la banda principale dell'SSTV; non in una finestra a parte ne' dal telefono
  const wasElsewhere = useRef(false);
  useEffect(() => {
    if (solo || REMOTE_HOST || elsewhere || wasElsewhere.current) return;
    wasElsewhere.current = true;
    void retuneForSstv().catch(() => toast(t('mode.retuneFail', { mode: 'SSTV' }), 'error'));
  }, [solo, elsewhere, t]);
  if (elsewhere && !solo) return <DetachedNote item="sstv" />;
  return <SstvBody solo={solo} />;
}

function SstvBody({ solo }: { solo: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['sstv'], queryFn: getSstv, retry: false, refetchInterval: 1000 });
  const images = useQuery({ queryKey: ['sstv-images'], queryFn: getSstvImages, retry: false });
  const [view, setView] = useState<'rx' | 'tx'>('rx');
  const [incoming, setIncoming] = useState<IncomingImage | null>(null);
  const [sendRequest, setSendRequest] = useState(0);
  const seq = useRef(0);

  const refresh = useCallback(() => void client.invalidateQueries({ queryKey: ['sstv'] }), [client]);
  const refreshImages = useCallback(() => void client.invalidateQueries({ queryKey: ['sstv-images'] }), [client]);

  // aprire la pagina accende l'ascolto; lasciandola (per esempio tornando all'FT8) si spegne, cosi' il motore non consuma in secondo piano,
  // salvo «Ascolto acceso all'avvio di Aurora»
  const keepListening = useRef(false);
  keepListening.current = snap.data?.settings.auto_receive === true;
  // sul client remoto (telefono, portatile in rete) aprire la pagina non accende il motore: si ascolta solo dopo «Ascolta»
  const [armed, setArmed] = useState(!REMOTE_HOST);
  useListenWhileOpen(sstvRx, () => keepListening.current, refresh, sstvRxPage, armed);

  // con la pagina aperta l'ascolto resta acceso finche' l'operatore non preme «Ferma ascolto», anche mentre compone (la ricezione vive
  // nel motore e continua in sottofondo). L'accensione mandata una volta sola all'apertura poteva perdersi: motore non ancora pronto
  // quando la finestra si apre gia' sulla pagina SSTV, oppure un'altra vista (finestra staccata, telefono) che lasciando la pagina spegne
  // l'ascolto. Allora l'ascolto restava spento fino alla visita successiva: ora, se lo stato dice spento, si riaccende.
  const wantListen = useRef(true);
  const lastStart = useRef(0);
  const listening = snap.data?.rx.listening;
  const txState = snap.data?.tx.state;
  useEffect(() => {
    if (!armed || listening !== false || !wantListen.current || txState === 'sending') return;
    if (Date.now() - lastStart.current < 2500) return;
    lastStart.current = Date.now();
    void sstvRx('start').then(refresh, refresh);
  }, [armed, listening, txState, refresh]);
  const onListen = useCallback((on: boolean) => {
    wantListen.current = on;
    if (on) lastStart.current = Date.now();
  }, []);

  // a fine trasmissione la cronologia delle trasmesse si aggiorna
  useEffect(
    () =>
      onSstv((ev) => {
        if (ev.kind === 'tx') {
          refresh();
          if (ev.state === 'sending' || ev.state === 'done') refreshImages();
          // finita la trasmissione (anche fermata o in errore) si torna subito alla ricezione, che non si e' mai fermata
          if (ev.state === 'done' || ev.state === 'aborted' || ev.state === 'error') {
            setView('rx');
            wantListen.current = true;
            void sstvRx('start').then(refresh, refresh);
          }
        }
      }),
    [refresh, refreshImages],
  );

  const reuse = (entry: SstvImageEntry) => {
    void getSstvImageBlob(entry.id).then((blob) => {
      seq.current += 1;
      setIncoming({ blob, call: entry.direction === 'tx' ? null : entry.call ?? null, seq: seq.current });
      setView('tx');
    });
  };

  const data = snap.data;
  const rx = data?.rx;
  const sending = data?.tx.state === 'sending';
  const dialHz = useStation((x) => x.status?.dial_hz ?? 0);
  // solo le bande accese nelle Impostazioni (piu' quella in uso), come nella barra dell'FT8
  const chosenBands = useBandFilter((x) => x.bands);
  // una frequenza nuova della radio riporta a 0 la correzione di sintonia a mano: il segnale nuovo si guarda da capo
  const lastDial = useRef(0);
  const tuneHz = rx?.tune_hz ?? 0;
  useEffect(() => {
    const before = lastDial.current;
    lastDial.current = dialHz;
    if (before > 0 && dialHz > 0 && before !== dialHz && tuneHz !== 0) void sstvTune(0).then(refresh, refresh);
  }, [dialHz, tuneHz, refresh]);
  // «Ricezione in corso» solo se arriva davvero un segnale: con la radio spenta il livello e' al minimo (silenzio digitale, -100 dB) e il bollino verde
  // diceva il falso; in quel caso resta «in ascolto»
  const silentInput = (rx?.level_db ?? 0) <= SILENT_LEVEL_DB;
  const badge = sending ? 'sending' : rx?.receiving && !silentInput ? 'receiving' : rx?.receiving || rx?.listening ? 'listening' : 'off';

  return (
    <div className={`sstv-page${solo ? ' sstv-page--solo' : ''}`}>
      <header className="sstv-head">
        {/* il modo si cambia da qui: un modo testuale apre la loro pagina */}
        <select className="sstv-modesel" value="sstv" aria-label={t('digi.menu.mode')} onChange={(e) => {
          if (e.target.value === 'wefax') window.location.hash = '#/wefax';
          else void gotoTextMode(e.target.value as 'psk31' | 'psk63' | 'rtty' | 'cw' | 'ssb' | 'olivia' | 'contestia' | 'qpsk31' | 'throb' | 'amtor' | 'mt63');
        }}>
          <option value="psk31">PSK31</option>
          <option value="psk63">PSK63</option>
          <option value="qpsk31">QPSK31</option>
          <option value="throb">Throb</option>
          <option value="mt63">MT63</option>
          <option value="amtor">Amtor</option>
          <option value="rtty">RTTY</option>
          <option value="cw">CW</option>
          <option value="olivia">Olivia</option>
          <option value="contestia">Contestia</option>
          <option value="ssb">SSB</option>
          <option value="sstv">SSTV</option>
          <option value="wefax">WEFAX</option>
        </select>
        <h1>
          {t('sstv.title')}
        </h1>
        {/* «In ascolto» non serve (l'ascolto e' sempre acceso con la pagina aperta): l'etichetta compare solo negli altri stati */}
        {badge !== 'listening' ? (
          <span className={`sstv-badge sstv-badge--${badge}`} role="status">
            <span className="sstv-dot" aria-hidden="true" />
            {t(`sstv.state.${badge}`)}
          </span>
        ) : null}
        {/* banda e frequenze SSTV dal piano di banda IARU */}
        <FreqPicker bands={visibleBands(chosenBands, dialHz > 0 ? bandOf(dialHz) : null)} dialHz={dialHz} mode="SSTV" rows={bandPlanRows('SSTV')} locked={sending} onPick={(hz) => void postFrequency(hz).catch(() => undefined)} />
        <div className="sstv-views" role="tablist">
          <button type="button" role="tab" aria-selected={view === 'rx'} className={view === 'rx' ? 'on' : ''} onClick={() => setView('rx')}>
            {t('sstv.view.rx')}
          </button>
          <button type="button" role="tab" aria-selected={view === 'tx'} className={view === 'tx' ? 'on' : ''} onClick={() => setView('tx')}>
            {t('sstv.view.tx')}
          </button>
        </div>
        {/* la rotella di sempre: apre le Impostazioni sulle schede dell'SSTV (finestra a parte, cosi' la pagina e l'ascolto restano) */}
        {REMOTE_HOST ? null : (
          <button type="button" className="solo-btn icon-btn icon-btn--sm sstv-gear" title={t('sstv.set.title')} aria-label={t('sstv.set.title')} onClick={() => openSettingsWindow('sstv')}>
            <Icon name="gear" />
          </button>
        )}
        {solo ? null : <DetachButton item="sstv" />}
      </header>
      {armed ? null : <ListenGate onListen={() => setArmed(true)} />}

      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? (
        <div className="sstv-body">
          <div className="sstv-main">
          <div hidden={view !== 'rx'}>
            {/* sotto il visore il riquadro di sintonia, largo quanto la scheda; a destra della scheda del modo il log del QSO */}
            <ReceiveView
              snap={data}
              onChanged={refresh}
              onSaved={refreshImages}
              onTransmit={() => {
                setView('tx');
                setSendRequest((n) => n + 1);
              }}
              onListen={onListen}
              tuning={rx ? <SstvTuning rx={rx} onChanged={refresh} /> : null}
              log={<SstvLogCard dialHz={dialHz} />}
            />
          </div>
          {/* il compositore resta montato: il lavoro non si perde passando a Ricevi */}
          <div hidden={view !== 'tx'}>
            <Composer snap={data} incoming={incoming} onChanged={refresh} sendRequest={sendRequest} />
          </div>
          </div>
          {/* le immagini ricevute e trasmesse stanno sotto: tutto resta in una pagina; le impostazioni sono nelle Impostazioni (rotella) */}
          <aside className="sstv-side-col">
            <History images={images.data ?? []} modes={data.modes} onChanged={refreshImages} onReuse={reuse} />
          </aside>
        </div>
      ) : null}
    </div>
  );
}
