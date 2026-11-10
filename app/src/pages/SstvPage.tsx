import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getSstv, getSstvImageBlob, getSstvImages, postFrequency, sstvRx, type SstvImageEntry } from '../api/client';
import { FreqPicker } from '../components/FreqPicker';
import { gotoTextMode } from '../lib/digiGoto';
import { bandPlanRows } from '../lib/bandplan';
import { useStation } from '../state/station';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { Composer, type IncomingImage } from '../components/sstv/Composer';
import { History } from '../components/sstv/History';
import { ReceiveView } from '../components/sstv/ReceiveView';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { onSstv } from '../lib/sstvBus';
import { SstvSettingsForm } from '../settings/SstvTab';
import '../styles/sstv.css';

/** La pagina SSTV: ricezione o compositore nell'area principale, sotto le cronologie delle ricevute e delle trasmesse. */
export function SstvPage({ solo = false }: { solo?: boolean } = {}) {
  const groups = useGroups();
  const elsewhere = groups.some((g) => g.open && g.items.includes('sstv'));
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
  const started = useRef(false);

  const refresh = useCallback(() => void client.invalidateQueries({ queryKey: ['sstv'] }), [client]);
  const refreshImages = useCallback(() => void client.invalidateQueries({ queryKey: ['sstv-images'] }), [client]);

  // aprire la pagina accende l'ascolto (non disturba l'FT8); lasciarla non lo spegne
  useEffect(() => {
    if (started.current || !snap.data) return;
    started.current = true;
    if (!snap.data.rx.listening) void sstvRx('start').then(refresh, refresh);
  }, [snap.data, refresh]);

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
  const badge = sending ? 'sending' : rx?.receiving ? 'receiving' : rx?.listening ? 'listening' : 'off';

  return (
    <div className={`sstv-page${solo ? ' sstv-page--solo' : ''}`}>
      <header className="sstv-head">
        <h1>
          {t('sstv.title')}
        </h1>
        <span className={`sstv-badge sstv-badge--${badge}`} role="status">
          <span className="sstv-dot" aria-hidden="true" />
          {t(`sstv.state.${badge}`)}
        </span>
        {/* il modo si cambia da qui: un modo testuale apre la loro pagina */}
        <select className="sstv-modesel" value="sstv" aria-label={t('digi.menu.mode')} onChange={(e) => void gotoTextMode(e.target.value as 'psk31' | 'psk63' | 'rtty' | 'cw')}>
          <option value="psk31">PSK31</option>
          <option value="psk63">PSK63</option>
          <option value="rtty">RTTY</option>
          <option value="cw">CW</option>
          <option value="sstv">SSTV</option>
        </select>
        {/* banda e frequenze SSTV dal piano di banda IARU */}
        <FreqPicker dialHz={dialHz} mode="SSTV" rows={bandPlanRows('SSTV')} locked={sending} onPick={(hz) => void postFrequency(hz).catch(() => undefined)} />
        <div className="sstv-views" role="tablist">
          <button type="button" role="tab" aria-selected={view === 'rx'} className={view === 'rx' ? 'on' : ''} onClick={() => setView('rx')}>
            {t('sstv.view.rx')}
          </button>
          <button type="button" role="tab" aria-selected={view === 'tx'} className={view === 'tx' ? 'on' : ''} onClick={() => setView('tx')}>
            {t('sstv.view.tx')}
          </button>
        </div>
        {solo ? null : <DetachButton item="sstv" />}
      </header>

      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? (
        <>
          <div hidden={view !== 'rx'}>
            <ReceiveView
              snap={data}
              onChanged={refresh}
              onSaved={refreshImages}
              onTransmit={() => {
                setView('tx');
                setSendRequest((n) => n + 1);
              }}
            />
          </div>
          {/* il compositore resta montato: il lavoro non si perde passando a Ricevi */}
          <div hidden={view !== 'tx'}>
            <Composer snap={data} incoming={incoming} onChanged={refresh} sendRequest={sendRequest} />
          </div>
          <History images={images.data ?? []} modes={data.modes} onChanged={refreshImages} onReuse={reuse} />
          <details className="sstv-more">
            <summary>{t('sstv.set.title')}</summary>
            <SstvSettingsForm snap={data} />
          </details>
        </>
      ) : null}
    </div>
  );
}
