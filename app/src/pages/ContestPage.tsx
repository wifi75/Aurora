import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { getControls, postControl, postCq, postFrequency, postHalt, postMode, postReply, postTxMessage, postTxText } from '../api/client';
import {
  attivaContest,
  azzeraLog,
  disattivaContest,
  esporta,
  getCalendario,
  getContest,
  getContestLog,
  getConsiglio,
  getMappa,
  getRegole,
  modificaQso,
  salvaImpostazioni,
  valuta,
  type ContestStato,
  type Esportazione,
  type Impostazioni,
  type LogQso,
  type Regole,
  type Categoria,
  type Valore,
  type ContestRef,
} from '../api/contest';
import type { DecodedMessage } from '../api/schemas';
import { localeOf, useLang, useT, type Key, type T } from '../i18n';
import { saveText } from '../lib/foxLog';
import { toast } from '../lib/toast';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';
import { WaterfallStack } from '../components/WaterfallStack';
import { DragHandle, useDragHeight } from '../lib/dragHeight';
import { flagCode } from '../lib/flags';
import { formatDistance, usePrefs } from '../lib/prefs';
import { chiamabile, decodificheVisibili, ordinaRighe } from '../lib/contestOrder';
import { askConfirm } from '../components/ConfirmHost';
import { TxQuickControls } from '../components/TxQuickControls';
import { FreqPicker } from '../components/FreqPicker';
import { Allenamento, AnalisiGara } from './ContestExtra';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from '../components/MovableColumns';
import '../styles/contest.css';

const ATTIVITA = ['standard', 'na-vhf', 'arrl-digi', 'ww-digi', 'eu-vhf', 'field-day', 'roundup'] as const;

/** Durata leggibile: «3d 4h», «6h 12m», «12m». */
function durata(ms: number): string {
  const m = Math.max(0, Math.round(ms / 60_000));
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m % 60}m`;
  return `${m % 60}m`;
}

const utc = (ms: number) => new Date(ms).toISOString().slice(11, 19);
const utcHm = (ms: number) => new Date(ms).toISOString().slice(11, 16);
const isGrid = (s: string) => /^[A-R]{2}\d{2}([A-X]{2})?$/i.test(s) && s.toUpperCase() !== 'RR73';
const numero = (n: number | undefined) => (n ?? 0).toLocaleString();

/** Chi trasmette un messaggio decodificato e il suo locatore (se c'e'). */
function mittente(text: string): { call: string; grid: string; to: string } | null {
  const w = text.replace(/[<>]/g, '').trim().toUpperCase().split(/\s+/);
  if (w.length < 2) return null;
  if (w[0] === 'CQ' || w[0] === 'QRZ') {
    const rest = w.slice(1);
    const call = rest.length >= 2 && /^[A-Z]{1,4}$/.test(rest[0] ?? '') && !/\d/.test(rest[0] ?? '') ? rest[1] : rest[0];
    const last = rest[rest.length - 1] ?? '';
    return call ? { call, grid: isGrid(last) && last !== call ? last : '', to: 'CQ' } : null;
  }
  const last = w[w.length - 1] ?? '';
  return { call: w[1] ?? '', grid: isGrid(last) ? last : '', to: w[0] ?? '' };
}

type Riga = { d: DecodedMessage; call: string; grid: string; v?: Valore };

function Chip({ label, value, tip, tone }: { label: string; value: React.ReactNode; tip?: string; tone?: string }) {
  return (
    <span className={`ctc-chip${tone ? ` ctc-chip--${tone}` : ''}`} title={tip}>
      <small>{label}</small>
      <b>{value}</b>
    </span>
  );
}

export function ContestPage() {
  const t = useT();
  const qc = useQueryClient();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const stato = useQuery({ queryKey: ['contest'], queryFn: getContest, refetchInterval: 5000, retry: false });
  const switches = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false }).data;
  const [scelto, setScelto] = useState('');
  const [modale, setModale] = useState<'' | 'regole' | 'calendario' | 'export' | 'impostazioni'>('');
  // linguetta della pagina: in gara, allenamento (simulatore senza trasmettere) o analisi dopo la gara
  const [vista, setVista] = useState<'gara' | 'allena' | 'analisi'>('gara');
  const s = stato.data;
  const attivo = s?.attivo ?? null;
  const refresh = () => void qc.invalidateQueries({ queryKey: ['contest'] });

  const errore = (e: unknown) => toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error');
  const attiva = (id: string) =>
    attivaContest(id)
      .then((r) => {
        toast(t('contest.activated', { c: s?.contest.find((c) => c.id === id)?.nome ?? id }), 'info');
        r.avvisi.forEach((a) => toast(a, 'info'));
        refresh();
      })
      .catch(errore);
  const fine = () =>
    disattivaContest()
      .then(() => {
        toast(t('contest.ended'), 'info');
        refresh();
      })
      .catch(errore);

  if (stato.isError) return <section className="ctc"><p className="ctc-note">{t('contest.unavailable')}</p></section>;

  return (
    <section className="ctc">
      <div className="ctc-score" role="region" aria-label={t('contest.title')}>
        <button
          type="button"
          role="switch"
          aria-checked={!!attivo}
          className={attivo ? 'ctc-toggle ctc-toggle--on' : 'ctc-toggle'}
          disabled={!attivo && !scelto}
          title={t(attivo ? 'tip.contest.toggleOff' : 'tip.contest.toggleOn')}
          onClick={() => (attivo ? void fine() : void attiva(scelto))}
        >
          <span className="ctc-toggle-knob" aria-hidden="true" />
          {t(attivo ? 'contest.toggleOn' : 'contest.toggleOff')}
        </button>
        <SceltaContest elenco={s?.contest ?? []} valore={attivo ?? scelto} t={t} onScegli={(id) => (attivo ? void attiva(id) : setScelto(id))} />
        <DateContest c={(s?.contest ?? []).find((c) => c.id === (attivo ?? scelto))} finestra={attivo ? s?.finestra : undefined} t={t} regole={attivo ? s?.regole : undefined} scambio={s?.scambio_inviato ?? ''} />
        {attivo && s ? (
          <BarraPunteggio s={s} t={t} />
        ) : (
          <>
            <span className="ctc-muted">{t('contest.none')}</span>
            <button type="button" className="ctc-btn" style={{ marginLeft: 'auto' }} title={t('tip.contest.calendar')} onClick={() => setModale('calendario')}>
              {t('contest.calendar')}
            </button>
          </>
        )}
      </div>
      {/* contest spento: niente schede, RX/TX e frequenza (si vedono solo con un contest acceso) */}
      {attivo ? (
      <div className="ctc-views" role="tablist">
        <span className="ctc-views-tabs">
        {(['gara', 'allena', 'analisi'] as const).map((v) => (
          <button key={v} type="button" role="tab" aria-selected={vista === v} className={vista === v ? 'on' : undefined} onClick={() => setVista(v)}>
            {t(v === 'gara' ? 'contest.tab.race' : v === 'allena' ? 'contest.training' : 'contest.analysis')}
          </button>
        ))}
        </span>
        {/* il selettore delle frequenze della pagina Stazione, sulla riga delle schede, con le sole bande ammesse dal regolamento */}
        <SelettoreBanda bande={attivo ? (s?.regole?.bande.ammesse ?? []) : []} t={t} />
      </div>
      ) : null}
      {s?.errori_regole?.length ? <p className="ctc-note ctc-bad">{s.errori_regole.join(' · ')}</p> : null}

      {vista === 'allena' ? <Allenamento s={s} t={t} /> : null}
      {vista === 'analisi' ? <AnalisiGara s={s} t={t} /> : null}
      {vista === 'gara' && attivo && s?.regole ? (
        <Corpo s={s} t={t} vai={setVista} decodes={decodes} dx={status?.dx_call ?? null} dxGrid={status?.dx_grid ?? null} transmitting={!!status?.transmitting} switches={switches} apri={setModale} fine={() => void fine()} />
      ) : null}

      {modale === 'regole' && s?.regole ? <ModaleRegole r={s.regole} t={t} chiudi={() => setModale('')} /> : null}
      {modale === 'calendario' ? <ModaleCalendario t={t} chiudi={() => setModale('')} prepara={(id) => { setModale(''); void attiva(id); }} /> : null}
      {modale === 'export' && attivo ? <ModaleExport t={t} s={s} chiudi={() => setModale('')} /> : null}
      {modale === 'impostazioni' && attivo ? <ModaleImpostazioni id={attivo} t={t} chiudi={() => { setModale(''); refresh(); }} /> : null}
      {/* la pagina e' pubblicata come beta: nastro di traverso nell'angolo, non blocca i clic */}
      <div className="ctc-beta" aria-hidden="true">
        <span>BETA</span>
      </div>
    </section>
  );
}

/** La sigla e la spiegazione di una categoria nella lingua scelta (l'italiano dal campo `nome`, le altre dall'inglese). */
const sigla = (c: Categoria) => c.sigla || c.id;
const spiegazione = (c: Categoria, lang: string) => (lang === 'it' ? c.nome : c.en || c.nome);

/**
 * La tessera «Categoria» della barra: solo la sigla («SO-LP», «2A»). Il clic apre un pannello nello stile del menu dei contest con la
 * scelta della categoria del regolamento, gli operatori (riga OPERATORS del Cabrillo), e potenza, Assisted e classe del Field Day solo
 * se il regolamento li prevede. Le scelte si salvano con le impostazioni del contest.
 */
function TesseraCategoria({ s, t }: { s: ContestStato; t: T }) {
  const lang = useLang((x) => x.lang);
  const qc = useQueryClient();
  const [aperto, setAperto] = useState(false);
  const [imp, setImp] = useState<Impostazioni | null>(null);
  const radice = useRef<HTMLDivElement>(null);
  const r = s.regole;
  const id = s.attivo ?? '';
  useEffect(() => {
    if (!aperto) return;
    // si parte dalle impostazioni salvate (con l'anagrafica comune gia' riempita dal servizio)
    getRegole(id)
      .then((d) => setImp(d.impostazioni))
      .catch(() => setImp(s.impostazioni ?? null));
    const fuori = (e: PointerEvent) => {
      if (!radice.current?.contains(e.target as Node)) setAperto(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setAperto(false);
    };
    document.addEventListener('pointerdown', fuori);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('pointerdown', fuori);
      document.removeEventListener('keydown', esc);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aperto, id]);
  if (!r) return <Chip label={t('contest.category')} value={s.categoria || '—'} />;
  const opz = r.opzioni_categoria ?? { potenze: [], assistenza: false, classe_fd: false };
  const cat = imp ? r.categorie.find((c) => c.id === imp.categoria) : undefined;
  const multi = cat?.operatore === 'MULTI-OP';
  const call = s.stazione?.call ?? '';
  const set = <K extends keyof Impostazioni>(k: K, v: Impostazioni[K]) => imp && setImp({ ...imp, [k]: v });
  const salva = () => {
    if (!imp) return;
    // chi opera da solo: l'operatore e' il nominativo della stazione se il campo e' rimasto vuoto
    const pronto = { ...imp, operatori: imp.operatori.trim() || (multi ? '' : imp.call_contest || call) };
    salvaImpostazioni(id, pronto)
      .then(() => {
        toast(t('contest.set.saved'), 'info');
        setAperto(false);
        void qc.invalidateQueries({ queryKey: ['contest'] });
        void qc.invalidateQueries({ queryKey: ['contest-regole', id] });
      })
      .catch((e: unknown) => toast(String(e), 'error'));
  };
  return (
    <div className="ctc-pick ctc-cat" ref={radice}>
      <button type="button" className="ctc-chip ctc-cat-btn" aria-haspopup="dialog" aria-expanded={aperto} title={`${s.categoria_nome || ''}\n${t('tip.contest.cat.open')}`.trim()} onClick={() => setAperto(!aperto)}>
        <small>{t('contest.category')}</small>
        <b>{s.categoria || '—'}</b>
      </button>
      {aperto && imp ? (
        <div className="ctc-pick-pop ctc-cat-pop" role="dialog" aria-label={t('contest.category')}>
          <div className="ctc-pick-group">{t('contest.category')}</div>
          <div role="radiogroup" aria-label={t('contest.category')}>
            {r.categorie.map((c) => (
              <label key={c.id} className={`ctc-pick-item ctc-cat-item${c.id === imp.categoria ? ' is-on' : ''}`} title={`${c.operatore} · ${c.potenza} · ${c.trasmettitori}`}>
                <input type="radio" name="ctc-cat" checked={c.id === imp.categoria} onChange={() => set('categoria', c.id)} />
                <b className="ctc-cat-sigla">{sigla(c)}</b>
                <span className="ctc-cat-desc">{spiegazione(c, lang)}</span>
              </label>
            ))}
          </div>
          {opz.classe_fd ? (
            <label className="ctc-cat-row" title={t('tip.contest.cat.tx')}>
              <span>{t('contest.cat.tx')}</span>
              <input className="ctc-input" type="number" min={1} max={99} value={imp.trasmettitori_n || 1} onChange={(e) => set('trasmettitori_n', Math.max(1, Math.min(99, Number(e.target.value) || 1)))} />
              <b className="ctc-cat-sigla">{`${imp.trasmettitori_n || 1}${cat ? sigla(cat) : ''}`}</b>
            </label>
          ) : null}
          <label className="ctc-cat-row ctc-cat-col" title={t('tip.contest.cat.operators')}>
            <span>{multi ? t('contest.cat.operatorsMulti') : t('contest.set.operators')}</span>
            <input className="ctc-input" value={imp.operatori} placeholder={multi ? 'IU3ABC IU3XYZ' : imp.call_contest || call} onChange={(e) => set('operatori', e.target.value.toUpperCase())} />
            {multi && !imp.operatori.trim() ? <small className="ctc-warn">{t('contest.cat.operatorsMissing')}</small> : null}
          </label>
          {opz.potenze.length ? (
            <label className="ctc-cat-row" title={t('tip.contest.cat.power')}>
              <span>{t('contest.cat.power')}</span>
              <select className="ctc-input" value={imp.potenza || cat?.potenza || ''} onChange={(e) => set('potenza', e.target.value)}>
                {opz.potenze.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          {opz.assistenza ? (
            <label className="ctc-cat-row" title={t('tip.contest.cat.assisted')}>
              <span>{t('contest.set.assisted')}</span>
              <select className="ctc-input" value={imp.assistenza || cat?.assistenza || 'NON-ASSISTED'} onChange={(e) => set('assistenza', e.target.value)}>
                <option value="NON-ASSISTED">{t('contest.cat.no')}</option>
                <option value="ASSISTED">{t('contest.cat.yes')}</option>
              </select>
            </label>
          ) : null}
          <div className="ctc-actions">
            <button type="button" className="ctc-btn ctc-btn--on" onClick={salva}>
              {t('contest.set.save')}
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** La cascata sotto le Decodifiche: alta circa 60 px (piccola), con la maniglia sopra per regolarla (ricordata, al massimo 420 px per le Decodifiche). */
function Cascata() {
  const t = useT();
  const wf = useDragHeight('aurora-contest-wf-height-v3', 60, 420, 40);
  return (
    <section className="dock-panel ctc-box ctc-wf-box" aria-label={t('fox.waterfall')}>
      {/* solo la cascata, compatta: niente barra del titolo ne' barra degli strumenti (si regolano dalla pagina Stazione) */}
      <DragHandle label={t('fox.resize')} className="ctc-wf-handle" onPointerDown={wf.onPointerDownInverse} />
      <div className="ctc-wf" style={{ height: wf.height }}>
        <WaterfallStack />
      </div>
    </section>
  );
}

/** Il colore di un'etichetta come in Attivita' di banda (variabili --tag-* del tema). */
const tag = (k: string) => ({ background: `var(--tag-${k})`, color: `var(--tag-${k}-ink)` });

/** Colonne delle Decodifiche e del Log del contest: si spostano e si allargano trascinando i titoli, come nel DX Cluster. */
const DEC_COLUMNS = ['utc', 'db', 'hz', 'msg', 'country', 'km', 'new', 'value'] as const;
type DecCol = (typeof DEC_COLUMNS)[number];
/** Larghezze di partenza in em (a 13 px sono le misure di prima in pixel); Paese prende lo spazio che avanza (al minimo 2,5 em, come prima
 *  poteva stringersi del tutto: la tabella non diventa piu' larga del riquadro). */
const DEC_EM: Record<DecCol, number> = { utc: 6.5, db: 3.3, hz: 3.9, msg: 15.8, country: 2.5, km: 5.6, new: 7.1, value: 5.7 };
const DEC_COLS_KEY = 'aurora.contest.dec.cols';
const DEC_LABEL: Partial<Record<DecCol, Key>> = { utc: 'contest.col.utc', msg: 'contest.col.msg', country: 'contest.col.country', new: 'contest.col.new', value: 'contest.col.value' };
const DEC_NUM = new Set<DecCol>(['db', 'hz', 'km', 'value']);
const LOG_COLUMNS = ['utc', 'band', 'call', 'rcvd', 'points', 'state'] as const;
type LogCol = (typeof LOG_COLUMNS)[number];
/** Larghezze di partenza in em, strette perche' il riquadro del log e' piccolo; Ricevuto prende lo spazio che avanza. */
const LOG_EM: Record<LogCol, number> = { utc: 4.2, band: 3.8, call: 6.5, rcvd: 2.5, points: 5.5, state: 5.5 };
const LOG_COLS_KEY = 'aurora.contest.log.cols';
const LOG_LABEL: Record<LogCol, Key> = { utc: 'contest.col.utc', band: 'contest.col.band', call: 'contest.col.call', rcvd: 'contest.col.rcvd', points: 'contest.col.points', state: 'contest.col.state' };

/** Il messaggio con il nominativo di chi trasmette a pastiglia, come la stazione DX nella pagina Stazione. */
function pastiglia(text: string, call: string) {
  return text.split(/(\s+)/).map((p, i) => (call && p.toUpperCase() === call.toUpperCase() ? <mark key={i} className="dx-call">{p}</mark> : p));
}

const GRUPPI = ['hf', 'fd', 'vhf', 'other'] as const;
const gruppoDi = (c: ContestRef): (typeof GRUPPI)[number] =>
  c.id === 'personalizzato' || !c.finestra ? 'other' : c.famiglia === 'hf' ? 'hf' : c.famiglia === 'field-day' ? 'fd' : 'vhf';
const GIORNO = 86_400_000;
/** Pallino di stato: verde in corso, arancio entro 7 giorni, grigio lontano. */
const statoDi = (c: ContestRef, ora: number) => (!c.finestra ? 'off' : ora >= c.finestra.inizio_ms ? 'on' : c.finestra.inizio_ms - ora <= 7 * GIORNO ? 'soon' : 'off');

/**
 * Il menu dei contest, nello stile del menu delle frequenze della Stazione: contest raggruppati con titoletti, nome senza anno, a destra
 * la data della prossima edizione e un pallino di stato. Frecce, Invio, Esc e clic fuori per chiudere.
 */
function SceltaContest({ elenco, valore, t, onScegli }: { elenco: ContestRef[]; valore: string; t: T; onScegli(id: string): void }) {
  const lang = useLang((x) => x.lang);
  const [aperto, setAperto] = useState(false);
  const [att, setAtt] = useState(0);
  const radice = useRef<HTMLDivElement>(null);
  const ora = Date.now();
  // in ogni categoria per data di inizio, dal piu' vicino; senza data in fondo (a pari data, l'ordine dell'elenco)
  const perInizio = (g: string) =>
    elenco
      .filter((c) => gruppoDi(c) === g)
      .sort((x, y) => (x.finestra?.inizio_ms ?? Number.MAX_SAFE_INTEGER) - (y.finestra?.inizio_ms ?? Number.MAX_SAFE_INTEGER));
  const ordinati = GRUPPI.flatMap((g) => perInizio(g));
  const giorno = new Intl.DateTimeFormat(localeOf(lang), { day: 'numeric', month: 'short', timeZone: 'UTC' });
  const data = (c: ContestRef) => (c.finestra ? giorno.formatRange(c.finestra.inizio_ms, c.finestra.fine_ms - 60_000) : '');
  const scelto = elenco.find((c) => c.id === valore);
  useEffect(() => {
    if (!aperto) return;
    setAtt(Math.max(0, ordinati.findIndex((c) => c.id === valore)));
    const fuori = (e: PointerEvent) => {
      if (!radice.current?.contains(e.target as Node)) setAperto(false);
    };
    document.addEventListener('pointerdown', fuori);
    return () => document.removeEventListener('pointerdown', fuori);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aperto]);
  useEffect(() => {
    if (aperto) radice.current?.querySelector(`[data-i="${att}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [aperto, att]);
  const scegli = (id: string) => {
    setAperto(false);
    onScegli(id);
  };
  const tasti = (e: React.KeyboardEvent) => {
    if (!aperto) {
      if (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        setAperto(true);
      }
      return;
    }
    if (e.key === 'Escape') setAperto(false);
    else if (e.key === 'ArrowDown') setAtt((i) => Math.min(ordinati.length - 1, i + 1));
    else if (e.key === 'ArrowUp') setAtt((i) => Math.max(0, i - 1));
    else if (e.key === 'Enter' && ordinati[att]) scegli(ordinati[att].id);
    else return;
    e.preventDefault();
  };
  let i = -1;
  return (
    <div className="ctc-pick" ref={radice} onKeyDown={tasti}>
      <button type="button" className="ctc-chip ctc-chip--sel ctc-pick-btn" aria-haspopup="listbox" aria-expanded={aperto} aria-label={t('contest.choose')} title={t('tip.contest.choose')} onClick={() => setAperto(!aperto)}>
        {scelto ? (
          <>
            <span className={`ctc-dot ctc-dot--${statoDi(scelto, ora)}`} aria-hidden="true" />
            <span className="ctc-pick-name">{scelto.nome}</span>
            <span className="ctc-pick-date">{data(scelto)}</span>
          </>
        ) : (
          <span className="ctc-pick-name">{t('contest.choose')}</span>
        )}
        <span className="ctc-pick-caret" aria-hidden="true">▾</span>
      </button>
      {aperto ? (
        <div className="ctc-pick-pop" role="listbox" aria-label={t('contest.choose')}>
          {GRUPPI.map((g) => {
            const voci = perInizio(g);
            if (voci.length === 0) return null;
            return (
              <div key={g} role="group" aria-label={t(`contest.group.${g}`)}>
                <div className="ctc-pick-group">{t(`contest.group.${g}`)}</div>
                {voci.map((c) => {
                  i += 1;
                  const n = i;
                  return (
                    <div key={c.id} data-i={n} role="option" aria-selected={c.id === valore} className={`ctc-pick-item${c.id === valore ? ' is-on' : ''}${n === att ? ' is-act' : ''}`} onPointerEnter={() => setAtt(n)} onClick={() => scegli(c.id)}>
                      <span className={`ctc-dot ctc-dot--${statoDi(c, ora)}`} aria-hidden="true" />
                      <span className="ctc-pick-name">{c.nome}</span>
                      <span className="ctc-pick-date">{data(c)}</span>
                    </div>
                  );
                })}
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

/**
 * Le date del contest scelto, compatte («29–30 ago 2026»), calcolate ogni anno dalla regola del calendario. Il clic apre un piccolo
 * pannello con orari UTC e locali, durata, ore massime e pausa minima, link al regolamento. «?» se le date vanno ricontrollate.
 */
function DateContest({ c, finestra, t, regole, scambio = '' }: { c: ContestRef | undefined; finestra?: { inizio_ms: number; fine_ms: number } | null; t: T; regole?: Regole | null; scambio?: string }) {
  const mio = useStation((x) => x.status?.de_call ?? '');
  const loc = useStation((x) => x.status?.de_grid?.slice(0, 4) ?? '');
  const lang = useLang((x) => x.lang);
  const [aperto, setAperto] = useState(false);
  const f = finestra ?? c?.finestra;
  if (!c || !f) return null;
  // la fine e' esclusa: l'ultimo minuto di gara e' un minuto prima
  const fine = f.fine_ms - 60_000;
  const giorno = new Intl.DateTimeFormat(localeOf(lang), { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' });
  const quando = (tz?: string) => new Intl.DateTimeFormat(localeOf(lang), { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit', timeZone: tz });
  const ore = (f.fine_ms - f.inizio_ms) / 3_600_000;
  return (
    <span className="ctc-dates">
      <button type="button" className="ctc-chip ctc-chip--phase" title={t('contest.dates.tip')} aria-expanded={aperto} onClick={() => setAperto(!aperto)}>
        <span aria-label={giorno.formatRange(f.inizio_ms, fine)}>{t('contest.how')}</span>
        {c.da_verificare ? <sup className="ctc-warn" title={t('contest.dates.verify')}>?</sup> : null}
      </button>
      {aperto ? (
        <div className="ctc-dates-pop" role="dialog" onMouseLeave={() => setAperto(false)}>
          <b>{c.nome}</b>
          {regole ? (
            <>
              {/* come funziona, in breve: il riassunto del regolamento e un QSO d'esempio con quello che Aurora trasmette */}
              <span className="ctc-how-text">{lang === 'it' ? regole.sintesi.it : regole.sintesi.en || regole.sintesi.it}</span>
              <span className="ctc-how-title">{t('contest.how.tx')}</span>
              <code className="ctc-how-qso">
                {`CQ ${regole.messaggi.parola_cq ? regole.messaggi.parola_cq + ' ' : ''}${mio || 'IU3ABC'} ${loc || 'JN55'}`}
                <br />
                {`DL1ABC ${mio || 'IU3ABC'} ${(scambio || 'dB').replace(/dB/, '-10')}`}
                <br />
                {`DL1ABC ${mio || 'IU3ABC'} ${regole.messaggi.chiusura || 'RR73'}`}
              </code>
            </>
          ) : null}
          <span>{t('contest.dates.utc')}: {quando('UTC').format(f.inizio_ms)} → {quando('UTC').format(fine)}</span>
          <span>{t('contest.dates.local')}: {quando().format(f.inizio_ms)} → {quando().format(fine)}</span>
          <span>{t('contest.dates.duration')}: {Math.round(ore * 10) / 10} h</span>
          {c.ore_massime ? <span>{t('contest.dates.maxHours')}: {c.ore_massime} h</span> : null}
          {c.pausa_minima_min ? <span>{t('contest.dates.pause')}: {c.pausa_minima_min} min</span> : null}
          {c.da_verificare ? <span className="ctc-warn">? {t('contest.dates.verify')}</span> : null}
          {c.url_regole ? <a href={c.url_regole} target="_blank" rel="noreferrer">{t('contest.dates.rules')}</a> : null}
        </div>
      ) : null}
    </span>
  );
}

function BarraPunteggio({ s, t }: { s: ContestStato; t: T }) {
  const tot = s.totali;
  const ora = s.ora_ms ?? Date.now();
  const f = s.finestra;
  const fase = s.fase ?? 'sempre';
  const tempo = fase === 'prima' && f ? durata(f.inizio_ms - ora) : fase === 'in-corso' && f ? durata(f.fine_ms - ora) : '';
  return (
    <>
      <span className={`ctc-chip ctc-chip--phase ctc-phase--${fase}`} title={t('tip.contest.phase')}>
        ● {t(`contest.phase.${fase}` as 'contest.phase.prima', { t: tempo })}
      </span>
      <Chip label={t('contest.qso')} value={numero(tot?.qso)} />
      <Chip label={t('contest.mult')} value={numero(tot?.molt)} />
      <Chip label={t('contest.score')} value={numero(tot?.punteggio)} />
      <Chip label={t('contest.rate10')} value={`${s.rate10 ?? 0}/h`} tip={t('tip.contest.rate')} />
      <Chip label={t('contest.rate60')} value={`${s.rate60 ?? 0}/h`} tip={t('tip.contest.rate')} />
      <Chip label={t('contest.forecast')} value={<span className="ctc-warn">{numero(s.previsto)}</span>} tip={t('tip.contest.forecast')} tone="warn" />
      <Chip label={t('contest.sent')} value={<span className="data">{s.scambio_inviato || '—'}</span>} />
      <TesseraCategoria s={s} t={t} />
      {s.ore_massime ? <Chip label={t('contest.hours')} value={`${(s.ore_usate ?? 0).toFixed(1)} / ${s.ore_massime}`} /> : null}
      {s.banda_ok === false ? <span className="ctc-chip ctc-bad">{t('contest.badBand', { b: s.banda ?? '' })}</span> : null}
      {s.modo_ok === false ? <span className="ctc-chip ctc-bad">{t('contest.badMode', { m: s.modo ?? '' })}</span> : null}
      {/* Fox/Hound: vietato da tutti i regolamenti, il blocco resta ma la scritta non serve (la dice «Come funziona») */}
    </>
  );
}

/** Il selettore delle frequenze (lo stesso della pagina Stazione) con le sole bande del contest. */
function SelettoreBanda({ bande, t }: { bande: readonly string[]; t: T }) {
  const dial = useStation((x) => x.status?.dial_hz ?? 0);
  const mode = useStation((x) => x.status?.mode ?? '');
  const tx = useStation((x) => x.status?.transmitting ?? false);
  return (
    <div className="ctc-freq" title={t('tip.contest.freq')}>
      {/* RX verde in ascolto, TX rosso in trasmissione: un clic sul TX ferma subito la trasmissione (come nel pannello Trasmissione) */}
      {tx ? (
        <button type="button" className="rxtx-flag rxtx-flag--stop" data-state="tx" title={t('tip.txStop')} aria-label={t('stop.button')} onClick={() => void postHalt().catch(() => toast(t('stop.fail'), 'error'))}>
          TX
        </button>
      ) : (
        // clic su RX: libera il QSO in corso (come «Canc DX»: nominativo e locatore del DX, e la stazione scelta)
        <button
          type="button"
          className="rxtx-flag"
          data-state="rx"
          title={t('tip.contest.rxClear')}
          onClick={() => {
            useStation.getState().setPendingDx(null);
            void postControl('clear_dx').catch(() => undefined);
          }}
        >
          RX
        </button>
      )}
      <FreqPicker
        bands={bande.length ? bande : undefined}
        dialHz={dial}
        mode={mode}
        locked={tx}
        onPick={(hz) => void postFrequency(hz).catch((e: unknown) => toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error'))}
      />
    </div>
  );
}

/** La tabella delle Decodifiche: colonne che si spostano e si allargano trascinando i titoli; doppio clic su una riga per chiamare. */
export function TabellaDecodifiche({ righe, dx, stella, nonContest, rispondi, t }: { righe: Riga[]; dx: string | null; stella: string | undefined; nonContest: ReadonlySet<string>; rispondi: (x: Riga) => void; t: T }) {
  const prefs = usePrefs((p) => p.prefs);
  const cols = useMovableColumns({ storageKey: DEC_COLS_KEY, columns: DEC_COLUMNS, defaultEm: DEC_EM, elastic: 'country' });
  return (
    <>
      <table ref={cols.tableRef} className="ctc-dec-table mv-grid" style={cols.tableStyle}>
        <MovableColGroup cols={cols} />
        <thead onContextMenu={cols.openMenu}>
          <tr>
            {cols.visible.map((c, i) => {
              const label = c === 'db' ? 'dB' : c === 'hz' ? 'Hz' : c === 'km' ? prefs.distanceUnit : t(DEC_LABEL[c] as Key);
              return (
                <MovableTh key={c} cols={cols} col={c} index={i} className={`ctc-c--${c}${DEC_NUM.has(c) ? ' num' : ''}`}>
                  {label}
                </MovableTh>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {righe.length === 0 ? (
            <tr>
              <td colSpan={cols.visible.length} className="ctc-muted">{t('contest.noDecodes')}</td>
            </tr>
          ) : null}
          {righe.map(({ d, call, v }) => {
            const inQso = call === dx;
            const cls = inQso ? 'qso' : v?.doppio ? 'dup' : call === stella ? 'top' : '';
            // novita' come le etichette di Attivita' di banda: i colori delle etichette (--tag-*) del tema
            const novita = inQso ? (
              <span className="badge badge--tag" style={tag('worked')}>{t('contest.inQso')}</span>
            ) : v?.doppio ? (
              // doppio su questa banda; se altrove vale ancora, le bande dove serve
              <span className="badge badge--tag badge--quiet" title={v.bande_utili.length ? t('contest.dupeUseful', { b: v.bande_utili.join(' ') }) : t('contest.dupeAll')}>
                {t('contest.dupe')}{v.bande_utili.length ? ` → ${v.bande_utili.join(' ')}` : ''}
              </span>
            ) : nonContest.has(call) ? (
              <span className="badge badge--tag badge--quiet">{t('contest.nonContest')}</span>
            ) : v?.fuori_regola ? (
              <span className="badge badge--tag ctc-bad">{t('contest.outOfRules')}</span>
            ) : v?.molt_nuovi.length ? (
              v.molt_nuovi.map((m) => (
                <span key={m} className="badge badge--tag" style={tag(/^[A-R]{2}\d{2}$/i.test(m) ? 'new-grid' : 'new-dxcc')}>
                  {m}
                </span>
              ))
            ) : null;
            const code = prefs.showFlag ? flagCode(d.country ?? null) : null;
            // ogni cella sa a quale colonna appartiene: le colonne cambiano posto e le celle le seguono
            const cella = (c: DecCol) => {
              switch (c) {
                case 'utc':
                  return <td key={c} className="ctc-c--utc ctc-muted">{utc(d.time_ms)}</td>;
                case 'db':
                  return <td key={c} className="ctc-c--db num">{d.snr > 0 ? `+${d.snr}` : d.snr}</td>;
                case 'hz':
                  return <td key={c} className="ctc-c--hz num ctc-muted">{d.freq_hz}</td>;
                case 'msg':
                  return <td key={c} className="ctc-c--msg">{pastiglia(d.text, call)}</td>;
                case 'country':
                  return (
                    <td key={c} className="ctc-c--country ctc-country">
                      {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null} {d.country ?? ''}
                    </td>
                  );
                case 'km':
                  return <td key={c} className="ctc-c--km num ctc-muted">{v && v.km > 0 ? formatDistance(v.km, prefs.distanceUnit) : ''}</td>;
                case 'new':
                  return <td key={c} className="ctc-c--new ctc-new-cell"><span className="ctc-new">{novita}</span></td>;
                case 'value':
                  return <td key={c} className="ctc-c--value num">{v && v.punti > 0 ? <span className="badge badge--tag ctc-pill">{(call === stella ? '★ ' : '') + t('contest.pt', { n: v.punti })}</span> : v ? t('contest.pt', { n: v.punti }) : ''}</td>;
              }
            };
            return (
              <tr key={d.id} className={`ctc-r ${cls}`} onDoubleClick={() => void rispondi({ d, call, grid: '', v })} title={v?.bande_utili.length ? v.bande_utili.join(' ') : undefined}>
                {cols.visible.map(cella)}
              </tr>
            );
          })}
        </tbody>
      </table>
      <ColumnsMenu cols={cols} />
    </>
  );
}

function Corpo({ s, t, vai, decodes, dx, dxGrid, transmitting, switches, apri, fine }: { s: ContestStato; t: T; vai: (v: 'allena' | 'analisi') => void; decodes: DecodedMessage[]; dx: string | null; dxGrid: string | null; transmitting: boolean; switches?: Partial<Record<string, boolean>>; apri: (m: 'regole' | 'calendario' | 'export' | 'impostazioni') => void; fine: () => void }) {
  const r = s.regole as Regole;
  const [campo, setCampo] = useState('');
  const ordine = usePrefs((p) => p.prefs.contestOrder);
  const soloCq = usePrefs((p) => p.prefs.contestOnlyCq);
  const mioCall = useStation((x) => x.status?.de_call ?? '');
  const modo = useStation((x) => x.status?.mode ?? '');
  // «Valore»: solo gli ultimi periodi; «Ora»: tutte, in ordine di tempo
  const recenti = useMemo(() => decodificheVisibili(decodes, ordine, modo), [decodes, ordine, modo]);
  const righe0: Riga[] = useMemo(
    () =>
      recenti
        .map((d) => {
          const m = mittente(d.text);
          return m && m.call ? { d, call: m.call, grid: m.grid } : null;
        })
        .filter((x): x is Riga => x !== null),
    [recenti],
  );
  const chiave = righe0.map((x) => `${x.call}:${x.grid}`).join(',');
  const valori = useQuery({
    queryKey: ['contest-valuta', chiave, s.totali?.qso ?? 0],
    queryFn: () => valuta(Array.from(new Map(righe0.map((x) => [x.call, { call: x.call, grid: x.grid }])).values())),
    enabled: righe0.length > 0,
    retry: false,
  }).data;
  const perCall = useMemo(() => new Map((valori?.valori ?? []).map((v) => [v.call, v])), [valori]);
  const righe = useMemo(() => {
    const tutte = ordinaRighe(righe0.map((x) => ({ ...x, v: perCall.get(x.call) })), ordine, mioCall);
    // «Solo CQ»: solo chi chiama CQ (o QRZ) e chi chiama me, i QSO fra altre stazioni spariscono
    const out = soloCq ? tutte.filter((x) => chiamabile(x.d.text, mioCall)) : tutte;
    return campo ? out.filter((x) => x.grid.toUpperCase().startsWith(campo)) : out;
  }, [righe0, perCall, campo, ordine, soloCq, mioCall]);
  // doppio clic su un doppio: si chiede prima (in gara un doppio non vale punti)
  const rispondi = async (x: Riga) => {
    if (x.v?.doppio && !(await askConfirm(t('contest.dupeAsk', { c: x.call })))) return;
    void postReply(x.d.id, true);
  };
  // «Ora»: la riga nuova e' in fondo e l'elenco segue, a meno che chi usa non sia salito a leggere (come Attivita' di banda)
  const scrollRef = useRef<HTMLDivElement>(null);
  const alFondo = useRef(true);
  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (ordine === 'ora' && el && alFondo.current) el.scrollTop = el.scrollHeight;
  }, [righe, ordine]);
  useEffect(() => {
    alFondo.current = true;
  }, [ordine]);
  const nonContest = new Set(s.non_contest ?? []);
  // la stella (stazione consigliata) vale solo per chi si sente adesso: in «Ora» l'elenco ha anche le decodifiche vecchie
  const idAdesso = new Set(decodificheVisibili(righe.map((x) => x.d), 'valore', modo).map((d) => d.id));
  const migliori = righe.filter((x) => idAdesso.has(x.d.id) && x.v && !x.v.doppio && !x.v.fuori_regola && x.call !== dx && x.d.text.startsWith('CQ'));
  const stella = migliori[0]?.call;
  const tx = useEngineTx((e) => e.tx);
  const autoseq = !!switches?.autoseq;
  // Auto CQ: il CQ del contest ripetuto a ogni periodo finche' qualcuno risponde (messaggio 6 scelto e Abilita Tx acceso); F1 lo avvia,
  // Esc ferma subito la trasmissione (come in N1MM e JTDX)
  const autoCq = !!switches?.enable_tx && tx?.selected === 6;
  const avviaCq = async () => {
    try {
      await postCq(s.sequenza?.parola_cq ?? '', true);
      await postControl('enable_tx', true);
    } catch (e) {
      toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error');
    }
  };
  const fermaCq = async () => {
    await Promise.allSettled([postControl('enable_tx', false), postHalt()]);
  };
  const avviaRef = useRef(avviaCq);
  avviaRef.current = avviaCq;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // mentre si scrive in un campo o con una finestra aperta i tasti F restano a loro
      const el = e.target instanceof HTMLElement ? e.target : null;
      if (el && (el.isContentEditable || el.closest('input, select, textarea, [role="dialog"], dialog, .modal'))) return;
      if (document.querySelector('dialog[open], [role="dialog"][aria-modal="true"], [role="alertdialog"]')) return;
      // F2, F3, F4: scelgono i messaggi 2 (scambio), 3 (R + scambio) e 4 (chiusura), come il clic sulla riga
      const n = { F2: 2, F3: 3, F4: 4 }[e.key];
      if (n) {
        e.preventDefault();
        void postTxMessage(n).catch(() => undefined);
        return;
      }
      if (e.key !== 'F1') return;
      e.preventDefault();
      void avviaRef.current();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  const txOn = !!switches?.enable_tx;
  const dxRiga = dx ? righe.find((x) => x.call === dx) : undefined;
  const mio = useStation((x) => x.status?.de_call ?? '');
  // il DX non fa il contest: per non perdere il QSO lo si chiude con i messaggi normali (rapporto, R+rapporto, RR73), scritti nei Tx 2-4
  const dxFuori = !!dx && nonContest.has(dx);
  const chiudiNormale = async (n: 2 | 3 | 4) => {
    if (!dx || !mio) return;
    const snr = dxRiga?.d.snr ?? -10;
    const rpt = `${snr >= 0 ? '+' : '-'}${String(Math.min(99, Math.abs(snr))).padStart(2, '0')}`;
    try {
      if (autoseq) await postControl('autoseq', false);
      await postTxText(2, `${dx} ${mio} ${rpt}`);
      await postTxText(3, `${dx} ${mio} R${rpt}`);
      await postTxText(4, `${dx} ${mio} RR73`);
      await postTxMessage(n);
    } catch (e) {
      toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error');
    }
  };
  const banda = s.banda ?? '';

  return (
    <main className="ctc-main">
      {/* colonna sinistra: Decodifiche sopra e la cascata sotto, larga uguale, con la maniglia in mezzo */}
      <div className="ctc-left">
      <section className="dock-panel ctc-box ctc-dec">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2 title={t('tip.contest.decodes')}>
          {t(ordine === 'ora' ? 'contest.decodes.time' : 'contest.decodes')}{' '}
          <span className="dock-seg" role="group" aria-label={t('contest.order')} onPointerDown={(e) => e.stopPropagation()}>
            <span className="ctc-muted">{t('contest.order')}:</span>
            {(['valore', 'ora'] as const).map((o) => (
              <button key={o} type="button" className={ordine === o ? 'dock-mini dock-mini--on' : 'dock-mini'} aria-pressed={ordine === o} title={t(o === 'valore' ? 'tip.contest.order.value' : 'tip.contest.order.time')} onClick={() => usePrefs.getState().set({ contestOrder: o })}>
                {t(o === 'valore' ? 'contest.order.value' : 'contest.order.time')}
              </button>
            ))}
          </span>{' '}
          <button type="button" className={soloCq ? 'dock-mini dock-mini--on' : 'dock-mini'} aria-pressed={soloCq} title={t('tip.contest.onlyCq')} onPointerDown={(e) => e.stopPropagation()} onClick={() => usePrefs.getState().set({ contestOnlyCq: !soloCq })}>
            {t('contest.onlyCq')}
          </button>{' '}
          <span className="r">{campo ? <button type="button" className="ctc-link" onClick={() => setCampo('')}>{campo} ✕</button> : t('contest.decodesHint')}</span>
        </h2>
        </header>
        <div className="ctc-scroll data" ref={scrollRef} onScroll={(e) => { const el = e.currentTarget; alFondo.current = el.scrollHeight - el.clientHeight - el.scrollTop < 8; }}>
          <TabellaDecodifiche righe={righe} dx={dx} stella={stella} nonContest={nonContest} rispondi={(x) => void rispondi(x)} t={t} />
        </div>
      </section>
      <Cascata />
      </div>

      <section className="dock-panel ctc-box ctc-qso">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2>
          {t('contest.qsoNow')} <span className="r">{t('contest.seqHint')}</span>
        </h2>
        </header>
        {dx ? (
          <div className="ctc-dx">
            <span className="ctc-pill data">
              {dx} {dxGrid ?? ''}
            </span>
            <span className="ctc-muted">
              {dxRiga?.d.country ?? ''}
              {dxRiga?.v ? ` · ${Math.round(dxRiga.v.km).toLocaleString()} km · ${t('contest.pt', { n: dxRiga.v.punti })}` : ''}
            </span>
          </div>
        ) : (
          <p className="ctc-tip">{t('contest.idle')}</p>
        )}
        {dxFuori ? (
          <div className="ctc-alert ctc-alert--o ctc-normal">
            <span>{t('contest.normal.title', { c: dx ?? '' })}</span>
            <span className="ctc-actions">
              <button type="button" className="ctc-btn" onClick={() => void chiudiNormale(2)}>{t('contest.normal.rpt')}</button>
              <button type="button" className="ctc-btn" onClick={() => void chiudiNormale(3)}>{t('contest.normal.rrpt')}</button>
              <button type="button" className="ctc-btn" onClick={() => void chiudiNormale(4)}>RR73</button>
            </span>
          </div>
        ) : null}
        <div className="ctc-steps data">
          {[
            [2, 'contest.step.exch'],
            [3, 'contest.step.rexch'],
            [4, 'contest.step.close'],
            [6, 'contest.step.cq'],
          ].map(([n, k]) => {
            const testo = tx?.messages[(n as number) - 1] ?? '';
            const ora = tx?.selected === n;
            return (
              <div key={n} className={ora ? 'now' : undefined} onClick={() => void postTxMessage(n as number)} role="button" tabIndex={0}>
                <span>
                  {n} · {testo || '—'}
                </span>
                <kbd className="ctc-kbd ctc-kbd--row">{n === 6 ? 'F1' : `F${n}`}</kbd>
                <span className={ora && transmitting ? 'ctc-val' : 'ctc-muted'}>{ora && transmitting ? t('contest.onAir') : t(k as 'contest.step.exch')}</span>
              </div>
            );
          })}
        </div>
        <div className="ctc-actions">
          <button type="button" className={`ctc-btn${autoseq ? ' ctc-btn--on' : ''}`} aria-pressed={autoseq} title={t('tip.contest.autoseq')} onClick={() => void postControl('autoseq', !autoseq)}>
            {t('contest.autoseq')}
          </button>
          <button type="button" className={`ctc-btn ctc-btn--tx${txOn ? ' ctc-btn--on' : ''}`} aria-pressed={txOn} onClick={() => void postControl('enable_tx', !txOn)}>
            {t('contest.enableTx')}
          </button>
          <button type="button" className={`ctc-btn ctc-btn--tx${autoCq ? ' ctc-btn--on' : ''}`} aria-pressed={autoCq} title={t('tip.contest.autoCq')} onClick={() => void (autoCq ? fermaCq() : avviaCq())}>
            {t('contest.autoCq')} <kbd className="ctc-kbd">F1</kbd>
          </button>
          <button type="button" className="ctc-btn" disabled={!dx} title={t('tip.contest.rr73')} onClick={() => void postTxMessage(4)}>
            {t('contest.rr73')}
          </button>
          {/* Tx/Rx Split: lo stesso controllo della Stazione e della Fox */}
          <TxQuickControls splitOnly inline />
          <button type="button" className="ctc-btn ctc-btn--stop" title={t('tip.txStop')} onClick={() => void postHalt()}>
            STOP TX <kbd className="ctc-kbd">Esc</kbd>
          </button>
        </div>
        {migliori.length ? (
          <div className="ctc-tip" title={t('tip.contest.next')}>
            {t('contest.next')}{' '}
            {migliori.slice(0, 3).map((x, i) => (
              <span key={x.call} className="ctc-next" onDoubleClick={() => void postReply(x.d.id, true)}>
                {i > 0 ? ' → ' : ''}
                <b>{x.call}</b> ({x.call === stella ? '★ ' : ''}
                {t('contest.pt', { n: x.v?.punti ?? 0 })}
                {x.v?.molt_nuovi.length ? `, ${x.v.molt_nuovi.join('+')}` : ''}
                {x.d.snr < -18 ? ', -' + Math.abs(x.d.snr) + ' dB' : ''})
              </span>
            ))}
          </div>
        ) : null}
      </section>

      {/* colonna destra in alto: consiglio di banda e modo sopra, mappa dei moltiplicatori sotto */}
      <div className="ctc-stack">
        <ConsiglioBanda t={t} />
        <MappaMoltiplicatori r={r} t={t} banda={banda} decodes={decodes} scegli={setCampo} />
        <section className="dock-panel ctc-box ctc-tipsbox">
          <header className="dock-grip dock-grip--fixed">
            <span className="dock-grip-dots" aria-hidden="true" />
            <h2>{t('contest.tips')}</h2>
          </header>
          <Avvisi s={s} t={t} />
          <PerBanda s={s} t={t} />
          <div className="ctc-rate">
            {(s.fase === 'prima' ? (s.per_ora ?? []).map(() => 0) : (s.per_ora ?? [])).map((n, i, all) => {
              const max = Math.max(1, ...all);
              return <i key={i} style={{ height: `${Math.max(3, (n / max) * 100)}%` }} title={`${n}/h`} />;
            })}
          </div>
          <div className="ctc-tip">{t('contest.rateHint')}</div>
          <div className="ctc-actions">
            <button type="button" className="ctc-btn" title={t('tip.contest.rules')} onClick={() => apri('regole')}>
              {t('contest.rules')}
            </button>
            <button type="button" className="ctc-btn" title={t('tip.contest.calendar')} onClick={() => apri('calendario')}>
              {t('contest.calendar')}
            </button>
            <button type="button" className="ctc-btn" title={t('tip.contest.export')} onClick={() => apri('export')}>
              {t('contest.export')}
            </button>
            <button type="button" className="ctc-btn" title={t('tip.contest.settings')} onClick={() => apri('impostazioni')}>
              {t('contest.settings')}
            </button>
            <button type="button" className="ctc-btn" onClick={() => vai('allena')}>
              {t('contest.training')}
            </button>
            <button type="button" className="ctc-btn" onClick={() => vai('analisi')}>
              {t('contest.analysis')}
            </button>
            <button type="button" className="ctc-btn ctc-btn--stop" title={t('tip.contest.end')} onClick={fine}>
              {t('contest.end')}
            </button>
          </div>
        </section>
      </div>

      <LogContest t={t} qso={s.totali?.qso ?? 0} />

    </main>
  );
}

/** QSO, punti e moltiplicatori di ogni banda, sotto il punteggio (dai totali gia' calcolati). Solo con piu' di una banda lavorata. */
function PerBanda({ s, t }: { s: ContestStato; t: T }) {
  const bande = s.totali?.per_banda ?? [];
  if (bande.length < 2) return null;
  return (
    <table className="ctc-band-tbl data" aria-label={t('contest.perBand')}>
      <thead>
        <tr>
          <th>{t('contest.perBand')}</th>
          <th className="num">{t('contest.qso')}</th>
          <th className="num">{t('contest.points')}</th>
          <th className="num">{t('contest.mult')}</th>
        </tr>
      </thead>
      <tbody>
        {bande.map((b) => (
          <tr key={b.banda} className={b.banda === s.banda ? 'now' : undefined}>
            <td>{b.banda}</td>
            <td className="num">{numero(b.qso)}</td>
            <td className="num">{numero(b.punti)}</td>
            <td className="num">{numero(b.molt)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Avvisi({ s, t }: { s: ContestStato; t: T }) {
  const out: { tono: 'g' | 'o' | 'r'; testo: string }[] = [];
  const ora = s.ora_ms ?? Date.now();
  if (s.fase === 'prima' && s.finestra) out.push({ tono: 'o', testo: t('contest.alert.before', { t: durata(s.finestra.inizio_ms - ora) }) });
  if (s.fase === 'finito') out.push({ tono: 'g', testo: t('contest.alert.after') });
  if (s.banda_ok === false) out.push({ tono: 'r', testo: t('contest.badBand', { b: s.banda ?? '' }) });
  if (s.modo_ok === false) out.push({ tono: 'r', testo: t('contest.badMode', { m: s.modo ?? '' }) });
  if (s.totali?.incerti) out.push({ tono: 'o', testo: t('contest.alert.uncertain', { n: s.totali.incerti }) });
  for (const c of (s.non_contest ?? []).slice(-2)) out.push({ tono: 'o', testo: t('contest.alert.nonContest', { c }) });
  if (s.ore_massime) out.push({ tono: (s.ore_usate ?? 0) > s.ore_massime - 1 ? 'r' : 'g', testo: t('contest.alert.hours', { u: (s.ore_usate ?? 0).toFixed(1), m: s.ore_massime, p: s.regole?.calendario.pausa_minima_min ?? 60 }) });
  if (s.regole?.restrizioni.avvio_umano) out.push({ tono: 'o', testo: t('contest.alert.human') });
  if (out.length === 0) out.push({ tono: 'g', testo: t('contest.alert.ok') });
  return (
    <div className="ctc-alerts">
      {out.map((a, i) => (
        <div key={i} className={`ctc-alert ctc-alert--${a.tono}`}>
          {a.testo}
        </div>
      ))}
    </div>
  );
}

/** Il riquadro del moltiplicatore come rettangolo in gradi (lon, lat in basso a sinistra, larghezza, altezza): campo (20x10) o quadrato (2x1). */
function rettangolo(k: string): { x: number; y: number; w: number; h: number } | null {
  const g = k.toUpperCase();
  if (!/^[A-R]{2}(\d{2})?$/.test(g)) return null;
  const x = (g.charCodeAt(0) - 65) * 20 - 180;
  const y = (g.charCodeAt(1) - 65) * 10 - 90;
  if (g.length === 2) return { x, y, w: 20, h: 10 };
  return { x: x + Number(g[2]) * 2, y: y + Number(g[3]), w: 2, h: 1 };
}

/** Consiglio di banda e modo: per ogni banda ammessa rate stimato e moltiplicatori nuovi in ascolto, la banda migliore e FT8/FT4. */
function ConsiglioBanda({ t }: { t: T }) {
  const q = useQuery({ queryKey: ['contest-consiglio'], queryFn: getConsiglio, refetchInterval: 15_000, retry: false }).data;
  const [ignora, setIgnora] = useState('');
  const [invio, setInvio] = useState(false);
  if (!q) return null;
  const best = q.bande.find((b) => b.banda === q.migliore);
  const chiave = best ? `${best.banda}|${best.modo}` : '';
  const qui = q.bande.find((b) => b.banda === q.banda);
  const resta = !!best && best.banda === q.banda && best.modo.toUpperCase() === q.modo.toUpperCase();
  const max = Math.max(1, ...q.bande.map((b) => b.punteggio));
  const porta = async (dove = best) => {
    if (!dove || dove.khz == null) return;
    setInvio(true);
    try {
      // prima il modo (la radio va sulla frequenza del modo nella banda di adesso), poi la frequenza del consiglio
      if (dove.modo.toUpperCase() !== q.modo.toUpperCase()) await postMode(dove.modo);
      await postFrequency(Math.round(dove.khz * 1000));
      toast(t('contest.adv.done', { b: dove.banda, m: dove.modo }), 'info');
    } catch (e) {
      toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error');
    } finally {
      setInvio(false);
    }
  };
  return (
    <section className="dock-panel ctc-box ctc-adv">
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2 title={t('tip.contest.adv')}>
          {t('contest.adv')} <span className="r">{t('contest.adv.every')}</span>
        </h2>
      </header>
      <div className="ctc-adv-in">
        {!best ? (
          <p className="ctc-tip">{t('contest.adv.noData')}</p>
        ) : resta ? (
          <div className="ctc-adv-big ctc-val">{t('contest.adv.stay', { b: best.banda, m: best.modo })}</div>
        ) : (
          <div className="ctc-adv-big ctc-val">{t('contest.adv.go', { b: best.banda, m: best.modo })}</div>
        )}
        {best ? (
          <div className="ctc-tip">
            {t('contest.adv.why', { b: best.banda, n: best.molt_nuovi.length, r: best.rate_stimato })}
            {qui && qui.banda !== best.banda ? ' ' + t('contest.adv.here', { b: qui.banda, r: qui.rate_stimato }) : ''}
            {best.modo.toUpperCase() === 'FT4' ? ' ' + t('contest.adv.ft4', { n: best.affollamento }) : ''}
          </div>
        ) : null}
        <div className="ctc-adv-bands data">
          {q.bande.map((b) => (
            <div
              key={b.banda}
              className={`ctc-adv-row${b.khz != null && b.banda !== q.banda ? ' ctc-adv-row--go' : ''}${b.banda === q.banda ? ' ctc-adv-row--here' : ''}`}
              title={b.khz != null && b.banda !== q.banda ? t('contest.adv.click', { b: b.banda, m: b.modo }) : b.molt_nuovi.join(' ') || undefined}
              onClick={() => {
                if (!invio && b.khz != null && b.banda !== q.banda) void porta(b);
              }}
            >
              <b>{b.banda}</b>
              <div className="ctc-adv-bar">
                <i className={b.banda === q.migliore ? 'best' : undefined} style={{ width: `${b.dati ? Math.max(4, (b.punteggio / max) * 100) : 0}%` }} />
              </div>
              <span className={b.banda === q.migliore ? 'ctc-val' : b.dati ? undefined : 'ctc-muted'}>
                {b.dati ? t('contest.adv.row', { r: b.rate_stimato, n: b.molt_nuovi.length }) : '—'}
              </span>
            </div>
          ))}
        </div>
        {q.cambi_banda_limite != null ? (
          <div className={`ctc-alert ctc-alert--${(q.cambi_banda_ora ?? 0) >= q.cambi_banda_limite ? 'r' : 'o'}`}>
            {t('contest.adv.changes', { n: q.cambi_banda_ora ?? 0, m: q.cambi_banda_limite })}
          </div>
        ) : null}
        {best && !resta && ignora !== chiave ? (
          <div className="ctc-actions">
            <button type="button" className="ctc-btn ctc-btn--on" disabled={invio || best.khz == null} title={best.khz != null ? `${best.khz} kHz` : undefined} onClick={() => void porta()}>
              {t('contest.adv.move', { b: best.banda, m: best.modo })}
            </button>
            <button type="button" className="ctc-btn" onClick={() => setIgnora(chiave)}>
              {t('contest.adv.ignore')}
            </button>
          </div>
        ) : null}
      </div>
    </section>
  );
}

/** Mappa dei moltiplicatori di una banda: lavorati, nuovi e in ascolto (arancio), sentiti non nuovi. Clic = chiama la stazione. */
function MappaMoltiplicatori({ r, t, banda, decodes, scegli }: { r: Regole; t: T; banda: string; decodes: DecodedMessage[]; scegli: (c: string) => void }) {
  const [scelta, setScelta] = useState(banda || r.bande.ammesse[0] || '');
  useEffect(() => {
    if (banda && r.bande.ammesse.includes(banda)) setScelta(banda);
  }, [banda, r.bande.ammesse]);
  const m = useQuery({ queryKey: ['contest-mappa', scelta], queryFn: () => getMappa(scelta), refetchInterval: 15_000, retry: false, enabled: r.moltiplicatori.length > 0 && !!scelta }).data;
  // l'ultima decodifica di ogni nominativo: serve il suo numero per rispondere (lo stesso comando del doppio clic sulle decodifiche)
  const ultima = useMemo(() => {
    const out = new Map<string, number>();
    for (const d of decodes) {
      const x = mittente(d.text);
      if (x?.call) out.set(x.call, d.id);
    }
    return out;
  }, [decodes]);
  const chiama = (call: string) => {
    const id = ultima.get(call);
    if (id == null) {
      toast(t('contest.map.gone', { c: call }), 'info');
      return;
    }
    void postReply(id, true).catch((e: unknown) => toast(t('contest.err', { e: e instanceof Error ? e.message : String(e) }), 'error'));
  };
  const tipo = m?.tipo ?? r.moltiplicatori[0]?.tipo ?? '';
  const quad = tipo === 'grid4';
  const chiaveDi = (g: string) => (g ? (quad ? g.slice(0, 4) : g.slice(0, 2)).toUpperCase() : '');
  // i riquadri: per ognuno lo stato e la stazione migliore da chiamare (prima chi da' un moltiplicatore nuovo, poi il segnale piu' forte)
  const celle = useMemo(() => {
    const out = new Map<string, { stato: 'f' | 'n' | 'h'; call?: string; snr?: number }>();
    if (!m?.griglia) return out;
    for (const v of m.lavorati) out.set(v.toUpperCase(), { stato: 'f' });
    const sentiti = [...m.sentiti].filter((s) => s.grid && !s.doppio).sort((a, b) => b.molt_nuovi.length - a.molt_nuovi.length || b.snr - a.snr);
    for (const s of sentiti) {
      const k = chiaveDi(s.grid);
      const c = out.get(k);
      const nuovo = s.molt_nuovi.length > 0;
      if (!c) out.set(k, { stato: nuovo ? 'n' : 'h', call: s.call, snr: s.snr });
      else if (!c.call) out.set(k, { stato: nuovo ? 'n' : 'f', call: s.call, snr: s.snr });
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [m, quad]);
  const nuovi = useMemo(() => (m?.sentiti ?? []).filter((s) => s.molt_nuovi.length > 0).sort((a, b) => b.snr - a.snr), [m]);
  // i moltiplicatori gia' presi sulle altre bande: un prefisso preso su 80 m vale ancora su 40 m (colore a parte)
  const altre = r.bande.ammesse.filter((b) => b !== scelta);
  const altrove = useQuery({
    queryKey: ['contest-mappa-altre', altre.join(',')],
    queryFn: async () => new Set((await Promise.all(altre.map((b) => getMappa(b).catch(() => null)))).flatMap((x) => (x?.lavorati ?? []).map((v) => v.toUpperCase()))),
    refetchInterval: 30_000,
    retry: false,
    enabled: r.moltiplicatori.length > 0 && altre.length > 0,
  }).data;
  // il prefisso WPX di un nominativo (fino all'ultima cifra del prefisso): per le stazioni che non danno un moltiplicatore nuovo
  const prefisso = (call: string) => (call.split('/').sort((a, b) => b.length - a.length)[0] ?? call).match(/^[A-Z0-9]*?\d+/)?.[0] ?? call;
  // tutte le stazioni sentite, a tessere colorate: nuovo ovunque, nuovo qui ma preso su un'altra banda, gia' preso qui, gia' lavorata
  const tessere = useMemo(() => {
    const out = (m?.sentiti ?? []).map((s) => {
      const chiave = (s.molt_nuovi[0] ?? (tipo === 'prefisso' ? prefisso(s.call) : '')).toUpperCase();
      const stato: 'nuovo' | 'altra' | 'preso' | 'doppio' = s.doppio ? 'doppio' : s.molt_nuovi.length > 0 ? (altrove?.has(chiave) ? 'altra' : 'nuovo') : 'preso';
      return { s, chiave, stato };
    });
    const ordine = { nuovo: 0, altra: 1, preso: 2, doppio: 3 };
    return out.sort((a, b) => ordine[a.stato] - ordine[b.stato] || b.s.snr - a.s.snr);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [m, altrove, tipo]);
  // il riquadro della mappa: tutti i riquadri e la stazione, con un margine; almeno 40x20 gradi
  const vista = useMemo(() => {
    const rr = [...celle.keys()].map(rettangolo).filter((x): x is NonNullable<typeof x> => !!x);
    const mio = m?.mio_grid ? rettangolo(m.mio_grid.slice(0, 4)) : null;
    if (mio) rr.push(mio);
    if (rr.length === 0) return { x: -180, y: -90, w: 360, h: 180, mio };
    let x0 = Math.min(...rr.map((q) => q.x));
    let x1 = Math.max(...rr.map((q) => q.x + q.w));
    let y0 = Math.min(...rr.map((q) => q.y));
    let y1 = Math.max(...rr.map((q) => q.y + q.h));
    const pad = quad ? 2 : 10;
    x0 -= pad;
    x1 += pad;
    y0 -= pad / 2;
    y1 += pad / 2;
    if (x1 - x0 < 40) [x0, x1] = [(x0 + x1) / 2 - 20, (x0 + x1) / 2 + 20];
    if (y1 - y0 < 20) [y0, y1] = [(y0 + y1) / 2 - 10, (y0 + y1) / 2 + 10];
    return { x: x0, y: y0, w: x1 - x0, h: y1 - y0, mio };
  }, [celle, m?.mio_grid, quad]);
  return (
    <section className="dock-panel ctc-box ctc-map-box">
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2 title={t('tip.contest.map')}>
          {t('contest.map')}
          {/* legenda nella barra del titolo: solo i quadratini dei colori, il significato passando col mouse */}
          <span className="ctc-mkeys" aria-label={t('contest.map')}>
            <i className="k-nuovo" title={t('contest.mt.nuovo')} />
            <i className="k-altra" title={t('contest.mt.altra')} />
            <i className="k-preso" title={t('contest.mt.preso')} />
            <i className="k-doppio" title={t('contest.mt.doppio')} />
            <i className="k-cq" title="CQ" />
          </span>
          <span className="r">{scelta} · {tipo || '—'}</span>
        </h2>
      </header>
      {r.moltiplicatori.length === 0 ? (
        <p className="ctc-tip">{t('contest.noMult')}</p>
      ) : (
        <>
          <div className="ctc-tabs" role="tablist">
            {r.bande.ammesse.map((b) => (
              <button key={b} type="button" role="tab" aria-selected={b === scelta} className={b === scelta ? 'on' : undefined} onClick={() => setScelta(b)}>
                {b.replace('m', '')}
              </button>
            ))}
          </div>
          {m?.griglia ? (
            <>
              <svg className="ctc-map" viewBox={`${vista.x} ${-(vista.y + vista.h)} ${vista.w} ${vista.h}`} preserveAspectRatio="xMidYMid meet" role="img" aria-label={t('contest.map')}>
                {[...celle.entries()].map(([k, c]) => {
                  const q = rettangolo(k);
                  if (!q) return null;
                  const tip = c.call ? `${k} · ${c.call} ${c.snr ?? ''} dB${c.stato === 'n' ? ' · ' + t('contest.map.new') : ''}` : k;
                  return (
                    <rect
                      key={k}
                      className={`ctc-cell ctc-cell--${c.stato}${c.call ? ' is-call' : ''}`}
                      x={q.x + q.w * 0.06}
                      y={-(q.y + q.h) + q.h * 0.08}
                      width={q.w * 0.88}
                      height={q.h * 0.84}
                      onClick={() => (c.call ? chiama(c.call) : scegli(k.slice(0, 2)))}
                    >
                      <title>{tip}</title>
                    </rect>
                  );
                })}
                {vista.mio ? <circle className="ctc-map-me" cx={vista.mio.x + vista.mio.w / 2} cy={-(vista.mio.y + vista.mio.h / 2)} r={quad ? 0.6 : 3} /> : null}
              </svg>
              <div className="ctc-legend">
                <span className="ctc-sq f" /> {t('contest.map.done')} &nbsp; <span className="ctc-sq n" /> {t('contest.map.newHeard')} &nbsp; <span className="ctc-sq h" /> {t('contest.map.heard')} · {t('contest.multTaken', { b: scelta, n: m.lavorati.length })}
              </div>
            </>
          ) : (
            <div className="ctc-scroll data">
              {/* a tessere su piu' colonne, colorate: viola nuovo ovunque, arancio nuovo qui ma preso su un'altra banda, verde gia' preso qui */}
              {tessere.length === 0 ? (
                <p className="ctc-muted">{t('contest.map.noneNew')}</p>
              ) : (
                <div className="ctc-mtiles">
                  {tessere.map(({ s, chiave, stato }) => (
                    <button
                      type="button"
                      key={s.call}
                      className={`ctc-mtile ctc-mtile--${stato}${s.cq ? ' ctc-mtile--cq' : ''}`}
                      title={`${t('tip.contest.mapCall')} · ${chiave} · ${t(`contest.mt.${stato}` as 'contest.mt.nuovo')} · ${s.snr} dB${s.cq ? ' · CQ' : ''}`}
                      onClick={() => chiama(s.call)}
                    >
                      <span className="ctc-mult">{chiave}</span>
                      <b>{s.call}</b>
                    </button>
                  ))}
                </div>
              )}
              <div className="ctc-legend">{t('contest.multTaken', { b: scelta, n: m?.lavorati.length ?? 0 })}</div>
            </div>
          )}
          {nuovi.length > 0 && m?.griglia ? (
            <div className="ctc-alert ctc-alert--g">{t('contest.map.summary', { b: scelta, n: new Set(nuovi.flatMap((s) => s.molt_nuovi)).size, l: [...new Set(nuovi.flatMap((s) => s.molt_nuovi))].slice(0, 6).join(', ') })}</div>
          ) : null}
        </>
      )}
    </section>
  );
}

export function LogContest({ t, qso }: { t: T; qso: number }) {
  const qc = useQueryClient();
  const log = useQuery({ queryKey: ['contest-log', qso], queryFn: getContestLog, refetchInterval: 10_000, retry: false }).data;
  const [modifica, setModifica] = useState<{ n: number; testo: string } | null>(null);
  // tasto destro su una riga: menu piccolo per segnarla valida, incerta o «non in contest» (0 punti, fuori dal Cabrillo)
  const [menu, setMenu] = useState<{ n: number; x: number; y: number } | null>(null);
  const cols = useMovableColumns({ storageKey: LOG_COLS_KEY, columns: LOG_COLUMNS, defaultEm: LOG_EM, elastic: 'rcvd' });
  useEffect(() => {
    if (!menu) return;
    const chiudi = () => setMenu(null);
    window.addEventListener('click', chiudi);
    window.addEventListener('scroll', chiudi, true);
    return () => {
      window.removeEventListener('click', chiudi);
      window.removeEventListener('scroll', chiudi, true);
    };
  }, [menu]);
  const righe = [...(log?.qso ?? [])].reverse();
  const incerti = righe.filter((q) => q.stato === 'incerto').length;
  const aggiorna = () => {
    void qc.invalidateQueries({ queryKey: ['contest-log'] });
    void qc.invalidateQueries({ queryKey: ['contest'] });
  };
  const salva = (n: number, patch: Parameters<typeof modificaQso>[1]) => modificaQso(n, patch).then(aggiorna).catch((e: unknown) => toast(String(e), 'error'));
  // ogni cella sa a quale colonna appartiene: le colonne cambiano posto e le celle le seguono
  const cella = (c: LogCol, q: LogQso) => {
    switch (c) {
      case 'utc':
        return <td key={c} className="ctc-c--utc">{utcHm(q.time_ms)}</td>;
      case 'band':
        return <td key={c} className="ctc-c--band">{q.band}</td>;
      case 'call':
        return <td key={c} className="ctc-c--call">{q.call}</td>;
      case 'rcvd':
        return (
          <td key={c} onClick={() => setModifica({ n: q.n, testo: q.exch_rcvd || q.grid })} className={`ctc-c--rcvd${q.scambio_ok ? '' : ' ctc-bad'}`} title={q.scambio_nota || (q.modificato ? t('contest.edited') : undefined)}>
            {modifica?.n === q.n ? (
              <input
                className="ctc-input"
                autoFocus
                value={modifica.testo}
                onChange={(e) => setModifica({ n: q.n, testo: e.target.value })}
                onBlur={() => setModifica(null)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    void salva(q.n, { exch_rcvd: modifica.testo });
                    setModifica(null);
                  }
                  if (e.key === 'Escape') setModifica(null);
                }}
              />
            ) : (
              <>
                {q.exch_rcvd || q.grid || '—'}
                {q.modificato ? ' ✎' : ''}
              </>
            )}
          </td>
        );
      case 'points':
        return (
          <td key={c} className={`ctc-c--points num${q.molt_nuovi.length ? ' ctc-val' : ''}`}>
            {q.doppio ? t('contest.dupe') : `${q.punti}${q.molt_nuovi.length ? ` · ${t('contest.multShort')}` : ''}${q.stato === 'incerto' ? ` · ${t('contest.state.incerto')}` : ''}`}
          </td>
        );
      case 'state':
        return (
          <td key={c} className="ctc-c--state">
            <select className="ctc-mini" aria-label={t('contest.state.valido')} value={q.stato} onChange={(e) => void salva(q.n, { stato: e.target.value as LogQso['stato'] })}>
              {(['valido', 'incerto', 'errore'] as const).map((x) => (
                <option key={x} value={x}>
                  {t(`contest.state.${x}`)}
                </option>
              ))}
            </select>
          </td>
        );
    }
  };
  return (
    <section className="dock-panel ctc-box ctc-logbox">
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2 title={t('tip.contest.log')}>
        {t('contest.log')}{' '}
        <span className="r">
          {t('contest.logCount', { n: righe.length, u: incerti })}{' '}
          <button type="button" className="ctc-link" title={t('tip.contest.resetLog')} onClick={() => window.confirm(t('contest.resetConfirm')) && void azzeraLog().then(aggiorna)}>
            {t('contest.resetLog')}
          </button>
        </span>
      </h2>
        </header>
      <div className="ctc-scroll data">
        <table ref={cols.tableRef} className="ctc-log-table mv-grid" style={cols.tableStyle}>
          <MovableColGroup cols={cols} />
          <thead onContextMenu={cols.openMenu}>
            <tr>
              {cols.visible.map((c, i) => (
                <MovableTh key={c} cols={cols} col={c} index={i} className={`ctc-c--${c}${c === 'points' ? ' num' : ''}`}>
                  {t(LOG_LABEL[c])}
                </MovableTh>
              ))}
            </tr>
          </thead>
          <tbody>
            {righe.map((q: LogQso) => (
              <tr
                key={q.n}
                className={q.doppio ? 'dup' : q.stato === 'incerto' ? 'warn' : q.stato === 'errore' ? 'dup' : undefined}
                title={t('tip.contest.rowMenu')}
                onContextMenu={(e) => {
                  e.preventDefault();
                  cols.closeMenu();
                  setMenu({ n: q.n, x: e.clientX, y: e.clientY });
                }}
              >
                {cols.visible.map((c) => cella(c, q))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {menu ? (
        <div className="ctc-menu" role="menu" style={{ left: menu.x, top: menu.y }}>
          <button type="button" role="menuitem" onClick={() => void salva(menu.n, { stato: 'valido' })}>{t('contest.state.valido')}</button>
          <button type="button" role="menuitem" onClick={() => void salva(menu.n, { stato: 'incerto' })}>{t('contest.state.incerto')}</button>
          <button type="button" role="menuitem" className="ctc-bad" onClick={() => void salva(menu.n, { stato: 'errore' })}>{t('contest.notInContest')}</button>
        </div>
      ) : null}
      <ColumnsMenu cols={cols} />
    </section>
  );
}

function Modale({ titolo, chiudi, children, t }: { titolo: string; chiudi: () => void; children: React.ReactNode; t: T }) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === 'Escape' && chiudi();
    window.addEventListener('keydown', k);
    return () => window.removeEventListener('keydown', k);
  }, [chiudi]);
  return (
    <div className="ctc-modal" role="dialog" aria-modal="true" aria-label={titolo} onClick={chiudi}>
      <div className="ctc-modal-box" onClick={(e) => e.stopPropagation()}>
        <header>
          <h2>{titolo}</h2>
          <button type="button" className="ctc-btn" onClick={chiudi}>
            {t('contest.close')}
          </button>
        </header>
        <div className="ctc-modal-body">{children}</div>
      </div>
    </div>
  );
}

function ModaleRegole({ r, t, chiudi }: { r: Regole; t: T; chiudi: () => void }) {
  const lang = useLang((s) => s.lang);
  const testo = lang === 'it' ? r.sintesi.it : r.sintesi.en || r.sintesi.it;
  return (
    <Modale titolo={`${r.contest.nome} · ${r.contest.sponsor}`} chiudi={chiudi} t={t}>
      <p>{testo}</p>
      <dl className="ctc-dl">
        <dt>{t('contest.rules.bands')}</dt>
        <dd>{r.bande.ammesse.join(', ')}</dd>
        <dt>{t('contest.rules.modes')}</dt>
        <dd>{r.modi.ammessi.join(', ')}</dd>
        <dt>{t('contest.rules.messages')}</dt>
        <dd>
          {t(`contest.act.${r.messaggi.attivita}` as 'contest.act.standard')}
          {r.messaggi.parola_cq ? ` · CQ ${r.messaggi.parola_cq}` : ''} · {r.messaggi.chiusura}
        </dd>
        <dt>{t('contest.rules.exchange')}</dt>
        <dd className="data">{r.scambio.inviato.join(' + ')}</dd>
        <dt>{t('contest.rules.format')}</dt>
        <dd>
          {r.export.formato.toUpperCase()} {r.export.cabrillo_contest ? `(CONTEST: ${r.export.cabrillo_contest})` : ''} {r.export.scadenza ? `· ${r.export.scadenza}` : ''}
        </dd>
      </dl>
      <p className="ctc-muted">{t('contest.rules.verified', { d: r.contest.verificato_il, y: r.contest.edizione })}</p>
      {r.contest.da_verificare.length ? <p className="ctc-warn">{t('contest.rules.check', { f: r.contest.da_verificare.join(', ') })}</p> : null}
      {r.contest.url_regole ? (
        <p>
          <a href={r.contest.url_regole} target="_blank" rel="noreferrer">
            {t('contest.rules.official')} ↗
          </a>
        </p>
      ) : null}
    </Modale>
  );
}

function ModaleCalendario({ t, chiudi, prepara }: { t: T; chiudi: () => void; prepara: (id: string) => void }) {
  const cal = useQuery({ queryKey: ['contest-calendario'], queryFn: getCalendario, retry: false }).data;
  const ora = Date.now();
  return (
    <Modale titolo={t('contest.cal.title')} chiudi={chiudi} t={t}>
      {cal && cal.contest.length === 0 ? <p>{t('contest.cal.empty')}</p> : null}
      <table className="ctc-cal data">
        <tbody>
          {(cal?.contest ?? []).map((c) => (
            <tr key={`${c.id}-${c.inizio_ms}`}>
              <td>{new Date(c.inizio_ms).toISOString().slice(0, 16).replace('T', ' ')}Z</td>
              <td>{c.nome}</td>
              <td className="ctc-muted">{c.inizio_ms > ora ? durata(c.inizio_ms - ora) : t('contest.phase.in-corso', { t: durata(c.fine_ms - ora) })}{c.da_verificare ? ` · ${t('contest.cal.check')}` : ''}</td>
              <td>
                <button type="button" className="ctc-btn" onClick={() => prepara(c.id)}>
                  {t('contest.cal.prepare')}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Modale>
  );
}

function ModaleExport({ t, s, chiudi }: { t: T; s?: ContestStato; chiudi: () => void }) {
  const [formato, setFormato] = useState(s?.regole?.export.formato ?? 'cabrillo');
  const [banda, setBanda] = useState('');
  const [e, setE] = useState<Esportazione | null>(null);
  useEffect(() => {
    esporta(formato, banda || undefined)
      .then(setE)
      .catch((x: unknown) => toast(String(x), 'error'));
  }, [formato, banda]);
  return (
    <Modale titolo={t('contest.exp.title')} chiudi={chiudi} t={t}>
      <div className="ctc-actions">
        <label>
          {t('contest.exp.format')}{' '}
          <select className="ctc-input" value={formato} onChange={(x) => setFormato(x.target.value)}>
            <option value="cabrillo">Cabrillo 3.0</option>
            <option value="edi">EDI (REG1TEST)</option>
            <option value="adif">ADIF</option>
          </select>
        </label>
        {formato === 'edi' ? (
          <select className="ctc-input" value={banda} onChange={(x) => setBanda(x.target.value)}>
            <option value="">—</option>
            {(s?.regole?.bande.ammesse ?? []).map((b) => (
              <option key={b}>{b}</option>
            ))}
          </select>
        ) : null}
        <button type="button" className="ctc-btn ctc-btn--on" disabled={!e} onClick={() => e && saveText(e.nome, e.testo)}>
          {t('contest.exp.save')}
        </button>
        {e?.invio_url ? (
          <a className="ctc-btn" href={e.invio_url} target="_blank" rel="noreferrer">
            {t('contest.exp.send')} ↗
          </a>
        ) : null}
      </div>
      {e ? (
        e.problemi.length ? (
          <div className="ctc-alert ctc-alert--o">
            <b>{t('contest.exp.problems')}</b>
            <ul>
              {e.problemi.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          </div>
        ) : (
          <div className="ctc-alert ctc-alert--g">{t('contest.exp.ok')}</div>
        )
      ) : null}
      <pre className="ctc-pre">{e?.testo ?? ''}</pre>
    </Modale>
  );
}

function ModaleImpostazioni({ id, t, chiudi }: { id: string; t: T; chiudi: () => void }) {
  const dati = useQuery({ queryKey: ['contest-regole', id], queryFn: () => getRegole(id), retry: false }).data;
  const [imp, setImp] = useState<Impostazioni | null>(null);
  useEffect(() => {
    if (dati && !imp) setImp(dati.impostazioni);
  }, [dati, imp]);
  if (!dati || !imp) return null;
  const r = dati.regole;
  const set = <K extends keyof Impostazioni>(k: K, v: Impostazioni[K]) => setImp({ ...imp, [k]: v });
  const testo = (k: keyof Impostazioni, label: string, tip?: string) => (
    <label title={tip}>
      {label}
      <input className="ctc-input" value={String(imp[k] ?? '')} onChange={(e) => set(k, e.target.value as never)} />
    </label>
  );
  const dataOra = (ms: number | null) => (ms ? new Date(ms).toISOString().slice(0, 16) : '');
  const daDataOra = (v: string) => (v ? Date.parse(`${v}:00Z`) : null);
  const salva = () =>
    salvaImpostazioni(id, imp)
      .then(() => {
        toast(t('contest.set.saved'), 'info');
        chiudi();
      })
      .catch((e: unknown) => toast(String(e), 'error'));
  const freq = Object.entries(r.bande.frequenze);
  return (
    <Modale titolo={`${t('contest.settings')} · ${r.contest.nome}`} chiudi={chiudi} t={t}>
      <div className="ctc-form">
        <label title={t('tip.contest.set.activity')}>
          {t('contest.set.activity')}
          <select className="ctc-input" value={imp.attivita || r.messaggi.attivita} onChange={(e) => set('attivita', e.target.value === r.messaggi.attivita ? '' : e.target.value)}>
            {ATTIVITA.map((a) => (
              <option key={a} value={a}>
                {t(`contest.act.${a}`)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t('contest.set.cqWord')}
          <input className="ctc-input" placeholder={r.messaggi.parola_cq} value={imp.parola_cq} onChange={(e) => set('parola_cq', e.target.value)} />
        </label>
        {testo('scambio', t('contest.set.exchange'), t('tip.contest.set.exchange'))}
        <label>
          {t('contest.set.serial')}
          <input className="ctc-input" type="number" min={1} max={9999} value={imp.seriale_partenza} onChange={(e) => set('seriale_partenza', Math.max(1, Number(e.target.value) || 1))} />
        </label>
        <label>
          {t('contest.category')}
          <select className="ctc-input" value={imp.categoria} onChange={(e) => set('categoria', e.target.value)}>
            {r.categorie.map((c) => (
              <option key={c.id} value={c.id}>
                {sigla(c)} — {c.nome}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t('contest.set.mode')}
          <select className="ctc-input" value={imp.modo} onChange={(e) => set('modo', e.target.value)}>
            {r.modi.ammessi.map((m) => (
              <option key={m}>{m}</option>
            ))}
          </select>
        </label>
        <label>
          {t('contest.set.power')}
          <input className="ctc-input" type="number" min={0} value={imp.potenza_w} onChange={(e) => set('potenza_w', Number(e.target.value) || 0)} />
        </label>
        {r.punteggio.formula.includes('bonus') ? (
          <label>
            {t('contest.set.bonus')}
            <input className="ctc-input" type="number" min={0} value={imp.bonus} onChange={(e) => set('bonus', Number(e.target.value) || 0)} />
          </label>
        ) : null}
        <label className="ctc-check">
          <input type="checkbox" checked={imp.hold_tx} onChange={(e) => set('hold_tx', e.target.checked)} /> {t('contest.set.holdTx')}
        </label>
        <label className="ctc-check">
          <input type="checkbox" checked={imp.rrr} onChange={(e) => set('rrr', e.target.checked)} /> {t('contest.set.rrr')}
        </label>
        {testo('call_contest', t('contest.set.callContest'), t('tip.contest.set.callContest'))}
        {testo('grid6', t('contest.set.grid6'))}
        {testo('antenna', t('contest.set.antenna'), t('tip.contest.set.antenna'))}
        {testo('operatori', t('contest.set.operators'))}
        {testo('club', t('contest.set.club'))}
        {testo('nome', t('contest.set.name'), t('tip.contest.set.shared'))}
        {testo('email', t('contest.set.email'), t('tip.contest.set.shared'))}
        {testo('location', t('contest.set.location'))}
        {testo('assistenza', t('contest.set.assisted'))}
        {testo('cabrillo_contest', t('contest.set.cabrillo'))}
        {testo('categoria_tempo', t('contest.set.catTime'), t('tip.contest.set.catTime'))}
        {testo('categoria_overlay', t('contest.set.catOverlay'), t('tip.contest.set.catOverlay'))}
        <label title={t('tip.contest.set.dates')}>
          {t('contest.set.start')}
          <input className="ctc-input" type="datetime-local" value={dataOra(imp.inizio_ms)} onChange={(e) => set('inizio_ms', daDataOra(e.target.value))} />
        </label>
        <label title={t('tip.contest.set.dates')}>
          {t('contest.set.end')}
          <input className="ctc-input" type="datetime-local" value={dataOra(imp.fine_ms)} onChange={(e) => set('fine_ms', daDataOra(e.target.value))} />
        </label>
        <label className="ctc-wide" title={t('tip.contest.set.shared')}>
          {t('contest.set.address')}
          <textarea className="ctc-input" rows={2} value={imp.indirizzo ?? ''} onChange={(e) => set('indirizzo', e.target.value)} />
        </label>
        <label className="ctc-wide">
          {t('contest.set.soapbox')}
          <textarea className="ctc-input" rows={2} value={imp.soapbox} onChange={(e) => set('soapbox', e.target.value)} />
        </label>
      </div>
      {freq.length ? (
        <div className="ctc-freqs" title={t('tip.contest.set.freqs')}>
          <b>{t('contest.set.freqs')}</b>
          {freq.map(([b, modi]) =>
            Object.entries(modi).map(([m, khz]) => (
              <button key={`${b}${m}`} type="button" className="ctc-mini" onClick={() => void postFrequency(Math.round(khz * 1000))}>
                {b} {m} {khz}
              </button>
            )),
          )}
        </div>
      ) : null}
      <div className="ctc-actions">
        <button type="button" className="ctc-btn ctc-btn--on" onClick={() => void salva()}>
          {t('contest.set.save')}
        </button>
      </div>
    </Modale>
  );
}
