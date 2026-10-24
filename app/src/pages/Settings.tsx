import { useQuery } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';
import { getStationInfo, type StationInfo } from '../api/client';
import { AlertSwitch } from '../components/AlertSwitch';
import { readAlerts, saveAlerts, type AlertPrefs } from '../lib/alerts';
import { COLOR_KEYS, COLOR_LABELS, applyColors, defaultColors, readColors, saveColors, type ColorKey, type TagColors } from '../lib/colors';
import { defaultLayout, saveLayout } from '../lib/layout';
import { browserStorage } from '../lib/theme';

const TABS = ['Colori', 'Finestre', 'Avvisi sonori', 'Trasmissione', 'Log esterni', 'Collegamenti'] as const;
type Tab = (typeof TABS)[number];

/** Impostazioni di questo dispositivo e stato della stazione, una scheda per ogni funzione. */
export function Settings() {
  const [tab, setTab] = useState<Tab>('Colori');
  return (
    <div className="settings">
      <h1 className="page-title">Impostazioni</h1>
      <div className="tabs" role="tablist" aria-label="Impostazioni">
        {TABS.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            id={`tab-${name}`}
            aria-selected={tab === name}
            aria-controls="settings-panel"
            className={`glass${tab === name ? ' glass--on' : ''}`}
            onClick={() => setTab(name)}
          >
            {name}
          </button>
        ))}
      </div>
      <section className="panel settings-block" role="tabpanel" id="settings-panel" aria-label={tab}>
        <h2 className="panel-title">{tab}</h2>
        {tab === 'Colori' && <ColorsTab />}
        {tab === 'Finestre' && <WindowsTab />}
        {tab === 'Avvisi sonori' && <AlertsTab />}
        {tab === 'Trasmissione' && <InfoTab render={(i) => <TransmissionInfo info={i} />} />}
        {tab === 'Log esterni' && <InfoTab render={(i) => <ExportInfo info={i} />} />}
        {tab === 'Collegamenti' && <InfoTab render={(i) => <ConnectionsInfo info={i} />} />}
      </section>
      <a className="glass glass--link" href="#/">
        Torna alla stazione
      </a>
    </div>
  );
}

function ColorsTab() {
  const [colors, setColors] = useState<TagColors>(() => readColors(browserStorage()));
  const commit = (next: TagColors) => {
    setColors(next);
    saveColors(browserStorage(), next);
    applyColors(document.documentElement, next);
  };
  return (
    <>
      <p className="muted settings-hint">Stessi tipi di JTDX: ogni colore vale per l'etichetta e per la barra accanto al messaggio.</p>
      <div className="color-grid">
        {COLOR_KEYS.map((key: ColorKey) => (
          <label key={key} className="color-row">
            <span className="color-chip" style={{ background: colors[key], color: colors[key] }} aria-hidden="true" />
            <span className="color-name">{COLOR_LABELS[key]}</span>
            <input type="color" aria-label={COLOR_LABELS[key]} value={colors[key]} onChange={(e) => commit({ ...colors, [key]: e.target.value })} />
          </label>
        ))}
      </div>
      <button type="button" className="glass" onClick={() => commit(defaultColors())}>
        Ripristina i colori di JTDX
      </button>
    </>
  );
}

function WindowsTab() {
  const [note, setNote] = useState('');
  return (
    <>
      <p className="muted settings-hint">
        Trascina i pannelli dalla loro barra del titolo (anche nell'altra colonna o in basso), ridimensionali dal bordo inferiore e sposta il divisore tra le colonne.
      </p>
      <button
        type="button"
        className="glass"
        onClick={() => {
          saveLayout(browserStorage(), defaultLayout());
          setNote('Disposizione ripristinata: si vede riaprendo la stazione.');
        }}
      >
        Ripristina la disposizione
      </button>
      <p className="live-note" role="status">
        {note}
      </p>
    </>
  );
}

function AlertsTab() {
  const [prefs, setPrefs] = useState<AlertPrefs>(() => readAlerts(browserStorage()));
  return (
    <>
      <p className="muted settings-hint">Un suono per le novità: tre note per un paese nuovo, una sola per le altre. Partono spenti.</p>
      <AlertSwitch
        value={prefs}
        onChange={(next) => {
          saveAlerts(browserStorage(), next);
          setPrefs(next);
        }}
      />
    </>
  );
}

function InfoTab({ render }: { render(info: StationInfo): ReactNode }) {
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  if (info.isError) return <p className="notice">Non riesco a leggere lo stato della stazione.</p>;
  if (!info.data) return <p className="muted">Caricamento…</p>;
  return <>{render(info.data)}</>;
}

function Row({ name, children }: { name: string; children: ReactNode }) {
  return (
    <div className="info-row">
      <span className="muted">{name}</span>
      <span className="data">{children}</span>
    </div>
  );
}

function TransmissionInfo({ info }: { info: StationInfo }) {
  return (
    <>
      <Row name="Stato">{info.tx_allowed ? 'Trasmissione abilitata' : 'Trasmissione chiusa'}</Row>
      <p className="muted settings-hint">
        Un comando di risposta fa trasmettere JTDX davvero. Per questo Aurora parte con la trasmissione chiusa e si apre solo dal PC, all'avvio, con AURORA_ALLOW_TX=1. STOP TX funziona sempre.
      </p>
      <p className="muted settings-hint">
        Limiti dell'automazione: {info.limits.max_consecutive_cycles} risposte di fila, {info.limits.max_per_day} al giorno, spenta dopo {info.limits.dead_man_secs} s senza dispositivi.
      </p>
    </>
  );
}

function ExportInfo({ info }: { info: StationInfo }) {
  return (
    <>
      {info.export_udp.length === 0 && !info.export_file ? <p className="muted">Nessun log esterno configurato.</p> : null}
      {info.export_udp.map((a) => (
        <Row key={a} name="ADIF su UDP">
          {a}
        </Row>
      ))}
      {info.export_file ? <Row name="File ADIF">{info.export_file}</Row> : null}
      <p className="muted settings-hint">
        Ogni QSO registrato viene salvato nel log di Aurora e inviato qui. JTDX manda già i QSO a Log4OM2 con il suo server UDP secondario: non aggiungere lo stesso destinatario, sarebbero doppi. Si imposta con AURORA_EXPORT_UDP e AURORA_EXPORT_FILE.
      </p>
    </>
  );
}

function ConnectionsInfo({ info }: { info: StationInfo }) {
  return (
    <>
      <Row name="Ascolto di JTDX">{info.udp_listen}</Row>
      {info.udp_forward.map((a) => (
        <Row key={a} name="Inoltro identico a">
          {a}
        </Row>
      ))}
      <Row name="Ingresso audio">{info.audio_device ?? 'nessuno (niente cascata)'}</Row>
      <Row name="Paesi DXCC">{info.cty_loaded > 0 ? `${info.cty_loaded} paesi` : 'non caricati'}</Row>
      <Row name="Log dei QSO">{info.worked_qsos > 0 ? `${info.worked_qsos} QSO` : 'non letto'}</Row>
    </>
  );
}
