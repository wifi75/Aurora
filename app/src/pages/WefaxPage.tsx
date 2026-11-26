import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { getWefax, getWefaxImages, postFrequency, putWefax, wefaxRx, wefaxRxPage } from '../api/client';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { ListenGate } from '../components/ListenGate';
import { useGroups } from '../lib/detached';
import { History } from '../components/wefax/History';
import { WefaxMini } from '../components/wefax/Mini';
import { Controls } from '../components/wefax/Controls';
import { FreqPicker } from '../components/wefax/FreqPicker';
import { AfcSql } from '../components/wefax/AfcSql';
import { WefaxScope } from '../components/wefax/Scope';
import { Viewer } from '../components/wefax/Viewer';
import { useT } from '../i18n';
import { gotoTextMode } from '../lib/digiGoto';
import { useListenWhileOpen } from '../lib/listenWhileOpen';
import { REMOTE_HOST } from '../lib/phone';
import { rememberDialBeforeReceiveOnly } from '../lib/modeRetune';
import { effectivePresets, useWefaxFreq } from '../lib/wefaxFreqs';
import { presetAt, wefaxGroups, type WefaxPreset } from '../lib/wefaxStations';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';
import '../styles/sstv.css';
import '../styles/wefax.css';

const LAST_KEY = 'aurora.wefax.lastPreset';

/** L'ultima frequenza WEFAX scelta nel menu (vuota se non ce n'e' una). */
function lastWefaxId(): string {
  try {
    return browserStorage()?.getItem(LAST_KEY) ?? '';
  } catch {
    return '';
  }
}

function saveWefaxId(id: string): void {
  try {
    browserStorage()?.setItem(LAST_KEY, id);
  } catch {
    // senza archivio la scelta non si ricorda tra una apertura e l'altra
  }
}

/** La pagina WEFAX: ricezione delle cartine meteo (visore adattabile allo schermo, comandi, cronologia). Solo ricezione. */
/** Una scheda con titolo, bordo e sfondo come quelle delle altre pagine (modi testuali, Satelliti). */
function Card({ title, className = '', children }: { title: string; className?: string; children: ReactNode }) {
  return (
    <section className={`wefax-card ${className}`}>
      <h2 className="wefax-card-title">{title}</h2>
      <div className="wefax-card-body">{children}</div>
    </section>
  );
}

export function WefaxPage({ solo = false }: { solo?: boolean } = {}) {
  const groups = useGroups();
  const elsewhere = groups.some((g) => g.open && g.items.includes('wefax'));
  if (elsewhere && !solo) return <DetachedNote item="wefax" />;
  return <WefaxBody solo={solo} />;
}

function WefaxBody({ solo }: { solo: boolean }) {
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

  // il corpo della pagina prende tutta l'altezza che resta nella finestra (sotto la testata della pagina e sopra il pie' di pagina dell'app): niente
  // scorrimento della pagina e niente fasce tagliate in fondo, con qualunque dimensione di finestra e scala del testo
  const bodyBox = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const fit = () => {
      const el = bodyBox.current;
      if (!el) return;
      const zoomText = getComputedStyle(document.documentElement).getPropertyValue('--ui-zoom').trim();
      const zoom = Number.parseFloat(zoomText) > 0 ? Number.parseFloat(zoomText) : 1;
      const footer = document.querySelector('footer');
      const footerH = footer ? footer.getBoundingClientRect().height : 0;
      const top = el.getBoundingClientRect().top;
      const free = (window.innerHeight - footerH - top) / zoom - 8;
      el.style.height = `${Math.max(300, Math.floor(free))}px`;
    };
    fit();
    const id = window.setTimeout(fit, 400);
    window.addEventListener('resize', fit);
    return () => {
      window.clearTimeout(id);
      window.removeEventListener('resize', fit);
    };
  });
  const dialHz = useStation((x) => x.status?.dial_hz ?? 0);
  // la scelta resta visibile anche se la radio non ha ancora (o non puo') cambiare frequenza
  const [chosen, setChosen] = useState(() => lastWefaxId());
  // le frequenze sono quelle dell'utente (Impostazioni › WEFAX › Frequenze), di base quelle di fabbrica
  const presets = effectivePresets(useWefaxFreq());
  const current = presetAt(dialHz, presets);
  const groups = wefaxGroups(presets);

  // scegliendo una stazione si impostano anche IOC e righe al minuto di quella stazione: il ricevitore non riconosce l'IOC da solo
  // (con IOC sbagliato l'immagine ha la larghezza e il tono di avvio sbagliati) e le righe al minuto di partenza sono quelle dell'IOC
  const applyTransmission = (p: WefaxPreset) => {
    const s = snap.data?.settings;
    if (!s) return;
    const key = p.ioc === 288 ? 'lpm_288' : 'lpm_576';
    if (s.ioc === p.ioc && s[key] === p.lpm) return;
    void putWefax({ ...s, ioc: p.ioc, [key]: p.lpm }).then(() => snap.refetch(), () => undefined);
  };

  // una stazione scelta (dal menu o con le frecce): la radio ci va, la scelta si ricorda e si impostano IOC e righe al minuto
  const selectPreset = (p: WefaxPreset) => {
    setChosen(p.id);
    saveWefaxId(p.id);
    // la frequenza di adesso (se e' in una banda radioamatoriale) si ricorda: lasciando il WEFAX la radio torna li'
    rememberDialBeforeReceiveOnly(dialHz);
    void postFrequency(p.dialHz, true).catch(() => undefined);
    applyTransmission(p);
  };
  // le frecce passano alla frequenza piu' alta o piu' bassa dell'elenco (ordinato per frequenza): cambio rapido senza aprire il menu
  const ordered = [...groups.flatMap((g) => g.presets)].sort((a, b) => a.khz - b.khz || a.call.localeCompare(b.call));
  const nowId = current?.id ?? chosen;
  const nowIdx = ordered.findIndex((x) => x.id === nowId);
  const step = (d: 1 | -1) => {
    const base = nowIdx >= 0 ? nowIdx : d === 1 ? -1 : ordered.length;
    const p = ordered[base + d];
    if (p) selectPreset(p);
  };

  // entrando nella pagina la radio torna sulla frequenza WEFAX dell'ultima volta (la banda di prima si ricorda per l'uscita)
  const restored = useRef(false);
  useEffect(() => {
    if (restored.current || dialHz <= 0) return;
    restored.current = true;
    const last = groups.flatMap((g) => g.presets).find((x) => x.id === lastWefaxId());
    if (!last || last.dialHz === dialHz) return;
    rememberDialBeforeReceiveOnly(dialHz);
    void postFrequency(last.dialHz, true).catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dialHz]);

  return (
    <div className={`sstv-page wefax-page${solo ? ' sstv-page--solo' : ''}`}>
      <header className="sstv-head">
        {/* il modo si cambia da qui: un modo testuale o l'SSTV aprono la loro pagina */}
        <select
          className="sstv-modesel"
          value="wefax"
          aria-label={t('digi.menu.mode')}
          onChange={(e) => {
            if (e.target.value === 'sstv') window.location.hash = '#/sstv';
            else if (e.target.value !== 'wefax') void gotoTextMode(e.target.value as 'psk31' | 'psk63' | 'rtty' | 'cw' | 'ssb' | 'olivia' | 'contestia' | 'qpsk31' | 'throb');
          }}
        >
          <option value="psk31">PSK31</option>
          <option value="psk63">PSK63</option>
          <option value="qpsk31">QPSK31</option>
          <option value="throb">Throb</option>
          <option value="rtty">RTTY</option>
          <option value="cw">CW</option>
          <option value="olivia">Olivia</option>
          <option value="contestia">Contestia</option>
          <option value="ssb">SSB</option>
          <option value="sstv">SSTV</option>
          <option value="wefax">WEFAX</option>
        </select>
        <h1>{t('wefax.title')}</h1>
        {badge !== 'listening' ? (
          <span className={`sstv-badge sstv-badge--${badge} wefax-rxtag`} role="status" title={t(`wefax.state.${badge}`)} aria-label={t(`wefax.state.${badge}`)}>
            {badge === 'receiving' ? 'RX' : 'OFF'}
          </span>
        ) : null}
        {/* l'indicatore della frequenza della radio: dopo il menu del modo e prima delle stazioni */}
        <WefaxMini />
        {/* le stazioni meteo per banda e poi per frequenza: la radio va sulla portante (frequenza assegnata meno 1,9 kHz, in USB) */}
        <FreqPicker groups={groups} value={current?.id ?? chosen} onSelect={selectPreset} />
        <span className="wefax-step" role="group" aria-label={t('wefax.freq.label')}>
          <button type="button" className="glass wefax-step-btn" title={t('wefax.freq.up')} aria-label={t('wefax.freq.up')} disabled={nowIdx >= 0 && nowIdx >= ordered.length - 1} onClick={() => step(1)}>
            ▲
          </button>
          <button type="button" className="glass wefax-step-btn" title={t('wefax.freq.down')} aria-label={t('wefax.freq.down')} disabled={nowIdx === 0} onClick={() => step(-1)}>
            ▼
          </button>
        </span>
        {/* i comandi dello zoom del visore arrivano qui (stessa riga) */}
        <div id="wefax-toolbar-slot" className="wefax-toolbar-slot" />
        {data ? <AfcSql snap={data} onChanged={refresh} /> : null}
        {solo ? null : <DetachButton item="wefax" />}
      </header>
      {armed ? null : <ListenGate onListen={() => setArmed(true)} />}

      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? (
        <div className="wefax-body" ref={bodyBox}>
          <div className="wefax-main">
            {/* in alto: la cartina in arrivo */}
            <Card title={t('wefax.card.view')} className="wefax-card--view">
              <Viewer snapRows={data.rx.rows} listening={data.rx.listening} receiving={data.rx.receiving} lastId={[...(images.data ?? [])].sort((a, b) => b.time_ms - a.time_ms)[0]?.id} onSaved={refreshImages} onChanged={refresh} />
            </Card>
            {/* sotto, diviso in due alla stessa altezza: a sinistra spettro e cascata, a destra le cartine ricevute (miniature intere, un clic le apre in grande, pulsanti piccoli per scaricare e cancellare) */}
            <div className="wefax-band">
              <Card title={t('wefax.card.scope')} className="wefax-band-scope">
                <WefaxScope
                  centerHz={data.settings.center_hz}
                  shiftHz={data.settings.shift_hz}
                  afcHz={data.settings.afc ? (data.rx.afc_hz ?? 0) : 0}
                  squelch={data.settings.squelch ? { level: data.rx.signal ?? 0, threshold: data.settings.squelch_level } : undefined}
                  onCenter={(hz) => {
                    if (hz !== data.settings.center_hz) void putWefax({ ...data.settings, center_hz: hz }).then(refresh, refresh);
                  }}
                />
              </Card>
              <div className="wefax-band-hist">
                <History images={images.data ?? []} onChanged={refreshImages} live={data.rx.receiving ? data.rx.rows : null} />
              </div>
            </div>
          </div>
          {/* a destra: i comandi */}
          <div className="wefax-right">
            <Card title={t('wefax.card.rx')} className="wefax-card--rx">
              <Controls snap={data} onChanged={refresh} onListen={onListen} />
            </Card>
          </div>
        </div>
      ) : null}
    </div>
  );
}
