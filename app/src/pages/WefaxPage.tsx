import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getWefax, getWefaxImages, wefaxRx, wefaxRxPage } from '../api/client';
import { ListenGate } from '../components/ListenGate';
import { History } from '../components/wefax/History';
import { Controls } from '../components/wefax/Controls';
import { Viewer } from '../components/wefax/Viewer';
import { useT } from '../i18n';
import { gotoTextMode } from '../lib/digiGoto';
import { useListenWhileOpen } from '../lib/listenWhileOpen';
import { REMOTE_HOST } from '../lib/phone';
import '../styles/sstv.css';
import '../styles/wefax.css';

/** La pagina WEFAX: ricezione delle cartine meteo (visore adattabile allo schermo, comandi, cronologia). Solo ricezione. */
export function WefaxPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['wefax'], queryFn: getWefax, retry: false, refetchInterval: 1000 });
  const images = useQuery({ queryKey: ['wefax-images'], queryFn: getWefaxImages, retry: false });
  const refresh = useCallback(() => void client.invalidateQueries({ queryKey: ['wefax'] }), [client]);
  const refreshImages = useCallback(() => void client.invalidateQueries({ queryKey: ['wefax-images'] }), [client]);

  // aprire la pagina accende l'ascolto; lasciandola si spegne (salvo «Ascolto acceso all'avvio di Aurora»)
  const keepListening = useRef(false);
  keepListening.current = snap.data?.settings.auto_receive === true;
  // sul client remoto aprire la pagina non accende il motore: si ascolta solo dopo «Avvia ascolto»
  const [armed, setArmed] = useState(!REMOTE_HOST);
  useListenWhileOpen(wefaxRx, () => keepListening.current, refresh, wefaxRxPage, armed);

  // con la pagina aperta l'ascolto resta acceso finche' l'operatore non lo ferma: se lo stato lo dice spento, si riaccende
  const wantListen = useRef(true);
  const lastStart = useRef(0);
  const listening = snap.data?.rx.listening;
  useEffect(() => {
    if (!armed || listening !== false || !wantListen.current) return;
    if (Date.now() - lastStart.current < 2500) return;
    lastStart.current = Date.now();
    void wefaxRx('start').then(refresh, refresh);
  }, [armed, listening, refresh]);
  const onListen = useCallback((on: boolean) => {
    wantListen.current = on;
    if (on) lastStart.current = Date.now();
  }, []);

  const data = snap.data;
  const rx = data?.rx;
  const badge = rx?.receiving ? 'receiving' : rx?.listening ? 'listening' : 'off';

  return (
    <div className="sstv-page wefax-page">
      <header className="sstv-head">
        {/* il modo si cambia da qui: un modo testuale o l'SSTV aprono la loro pagina */}
        <select
          className="sstv-modesel"
          value="wefax"
          aria-label={t('digi.menu.mode')}
          onChange={(e) => {
            if (e.target.value === 'sstv') window.location.hash = '#/sstv';
            else if (e.target.value !== 'wefax') void gotoTextMode(e.target.value as 'psk31' | 'psk63' | 'rtty' | 'cw');
          }}
        >
          <option value="psk31">PSK31</option>
          <option value="psk63">PSK63</option>
          <option value="rtty">RTTY</option>
          <option value="cw">CW</option>
          <option value="sstv">SSTV</option>
          <option value="wefax">WEFAX</option>
        </select>
        <h1>{t('wefax.title')}</h1>
        {badge !== 'listening' ? (
          <span className={`sstv-badge sstv-badge--${badge}`} role="status">
            <span className="sstv-dot" aria-hidden="true" />
            {t(`wefax.state.${badge}`)}
          </span>
        ) : null}
        {/* nessun piano di banda: le frequenze meteo (portante meno 1,9 kHz in USB) si scelgono dalla radio */}
      </header>
      {armed ? null : <ListenGate onListen={() => setArmed(true)} />}

      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? (
        <div className="wefax-body">
          <div className="wefax-main">
            <Viewer snapRows={data.rx.rows} listening={data.rx.listening} receiving={data.rx.receiving} onSaved={refreshImages} onChanged={refresh} />
          </div>
          <Controls snap={data} onChanged={refresh} onListen={onListen} />
          <div className="wefax-hist-row">
            <History images={images.data ?? []} onChanged={refreshImages} />
          </div>
        </div>
      ) : null}
    </div>
  );
}
