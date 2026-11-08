import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { getControls, postControl, postCq, postFrequency, postHalt, postReply, postTxMessage } from '../api/client';
import {
  attivaContest,
  azzeraLog,
  disattivaContest,
  esporta,
  getCalendario,
  getContest,
  getContestLog,
  getLavorati,
  getRegole,
  modificaQso,
  salvaImpostazioni,
  valuta,
  type ContestStato,
  type Esportazione,
  type Impostazioni,
  type LogQso,
  type Regole,
  type Valore,
} from '../api/contest';
import type { DecodedMessage } from '../api/schemas';
import { useLang, useT, type T } from '../i18n';
import { saveText } from '../lib/foxLog';
import { toast } from '../lib/toast';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';
import { WaterfallStack } from '../components/WaterfallStack';
import { WaterfallToolbar } from '../components/WaterfallToolbar';
import { DragHandle, useDragHeight } from '../lib/dragHeight';
import { flagCode } from '../lib/flags';
import { formatDistance, usePrefs } from '../lib/prefs';
import '../styles/contest.css';

const ATTIVITA = ['standard', 'na-vhf', 'arrl-digi', 'ww-digi', 'eu-vhf', 'field-day', 'roundup'] as const;
const LETTERE = 'ABCDEFGHIJKLMNOPQR';

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

/** Il valore per l'ordinamento: i doppi in fondo, poi punti e moltiplicatori nuovi. */
const peso = (v?: Valore) => (!v ? 0 : v.doppio || v.fuori_regola ? -1 : v.punti + 10 * v.molt_nuovi.length);

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
        <select className="ctc-chip ctc-chip--sel" aria-label={t('contest.choose')} title={t('tip.contest.choose')} value={attivo ?? scelto} onChange={(e) => (attivo ? void attiva(e.target.value) : setScelto(e.target.value))}>
          {attivo ? null : <option value="">{t('contest.choose')} ▾</option>}
          {(s?.contest ?? []).map((c) => (
            <option key={c.id} value={c.id}>
              {c.nome} {c.edizione}
            </option>
          ))}
        </select>
        {attivo && s ? (
          <BarraPunteggio s={s} t={t} />
        ) : (
          <>
            <button type="button" className="ctc-btn ctc-btn--on" disabled={!scelto} title={t('tip.contest.activate')} onClick={() => void attiva(scelto)}>
              {t('contest.activate')}
            </button>
            <span className="ctc-muted">{t('contest.none')}</span>
            <button type="button" className="ctc-btn" style={{ marginLeft: 'auto' }} title={t('tip.contest.calendar')} onClick={() => setModale('calendario')}>
              {t('contest.calendar')}
            </button>
          </>
        )}
      </div>
      {s?.errori_regole?.length ? <p className="ctc-note ctc-bad">{s.errori_regole.join(' · ')}</p> : null}

      {attivo && s?.regole ? (
        <Corpo s={s} t={t} decodes={decodes} dx={status?.dx_call ?? null} dxGrid={status?.dx_grid ?? null} transmitting={!!status?.transmitting} switches={switches} apri={setModale} fine={() => void fine()} />
      ) : null}

      {/* la cascata a tutta larghezza sotto i riquadri, come nella Fox: barre Rx/Tx trascinabili, clic che sposta la Rx */}
      <Cascata />

      {modale === 'regole' && s?.regole ? <ModaleRegole r={s.regole} t={t} chiudi={() => setModale('')} /> : null}
      {modale === 'calendario' ? <ModaleCalendario t={t} chiudi={() => setModale('')} prepara={(id) => { setModale(''); void attiva(id); }} /> : null}
      {modale === 'export' && attivo ? <ModaleExport t={t} s={s} chiudi={() => setModale('')} /> : null}
      {modale === 'impostazioni' && attivo ? <ModaleImpostazioni id={attivo} t={t} chiudi={() => { setModale(''); refresh(); }} /> : null}
    </section>
  );
}

/** La fascia della cascata: alta circa 160 px, con la maniglia sopra per regolarla (ricordata, al massimo 420 px per le Decodifiche). */
function Cascata() {
  const t = useT();
  const wf = useDragHeight('aurora-contest-wf-height', 160, 420);
  return (
    <section className="dock-panel ctc-box ctc-wf-box" aria-label={t('fox.waterfall')}>
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2>{t('fox.waterfall')}</h2>
      </header>
      <DragHandle label={t('fox.resize')} className="ctc-wf-handle" onPointerDown={wf.onPointerDownInverse} />
      <WaterfallToolbar />
      <div className="ctc-wf" style={{ height: wf.height }}>
        <WaterfallStack />
      </div>
    </section>
  );
}

/** Il colore di un'etichetta come in Attivita' di banda (variabili --tag-* del tema). */
const tag = (k: string) => ({ background: `var(--tag-${k})`, color: `var(--tag-${k}-ink)` });

/** Il messaggio con il nominativo di chi trasmette a pastiglia, come la stazione DX nella pagina Stazione. */
function pastiglia(text: string, call: string) {
  return text.split(/(\s+)/).map((p, i) => (call && p.toUpperCase() === call.toUpperCase() ? <mark key={i} className="dx-call">{p}</mark> : p));
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
      <Chip label={t('contest.category')} value={s.categoria || '—'} />
      {s.ore_massime ? <Chip label={t('contest.hours')} value={`${(s.ore_usate ?? 0).toFixed(1)} / ${s.ore_massime}`} /> : null}
      {s.banda_ok === false ? <span className="ctc-chip ctc-bad">{t('contest.badBand', { b: s.banda ?? '' })}</span> : null}
      {s.modo_ok === false ? <span className="ctc-chip ctc-bad">{t('contest.badMode', { m: s.modo ?? '' })}</span> : null}
      {s.fox_vietato ? (
        <span className="ctc-chip ctc-bad" style={{ marginLeft: 'auto' }} title={t('tip.contest.foxBlocked')}>
          {t('contest.foxBlocked')}
        </span>
      ) : null}
    </>
  );
}

function Corpo({ s, t, decodes, dx, dxGrid, transmitting, switches, apri, fine }: { s: ContestStato; t: T; decodes: DecodedMessage[]; dx: string | null; dxGrid: string | null; transmitting: boolean; switches?: Partial<Record<string, boolean>>; apri: (m: 'regole' | 'calendario' | 'export' | 'impostazioni') => void; fine: () => void }) {
  const r = s.regole as Regole;
  const [campo, setCampo] = useState('');
  // le decodifiche degli ultimi due periodi (al piu' 45 s prima dell'ultima)
  const recenti = useMemo(() => {
    const top = decodes.reduce((m, d) => Math.max(m, d.time_ms), 0);
    return decodes.filter((d) => top - d.time_ms <= 45_000 || d.time_ms > top);
  }, [decodes]);
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
    const out = righe0.map((x) => ({ ...x, v: perCall.get(x.call) }));
    out.sort((a, b) => peso(b.v) - peso(a.v) || b.d.snr - a.d.snr);
    return campo ? out.filter((x) => x.grid.toUpperCase().startsWith(campo)) : out;
  }, [righe0, perCall, campo]);
  const nonContest = new Set(s.non_contest ?? []);
  const migliori = righe.filter((x) => x.v && !x.v.doppio && !x.v.fuori_regola && x.call !== dx && x.d.text.startsWith('CQ'));
  const stella = migliori[0]?.call;
  const prefs = usePrefs((p) => p.prefs);
  const tx = useEngineTx((e) => e.tx);
  const autoseq = !!switches?.autoseq;
  const txOn = !!switches?.enable_tx;
  const dxRiga = dx ? righe.find((x) => x.call === dx) : undefined;
  const banda = s.banda ?? '';

  return (
    <main className="ctc-main">
      <section className="dock-panel ctc-box ctc-dec">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2 title={t('tip.contest.decodes')}>
          {t('contest.decodes')} <span className="r">{campo ? <button type="button" className="ctc-link" onClick={() => setCampo('')}>{campo} ✕</button> : t('contest.decodesHint')}</span>
        </h2>
        </header>
        <div className="ctc-scroll data">
          <table>
            <thead>
              <tr>
                <th>{t('contest.col.utc')}</th>
                <th className="num">dB</th>
                <th className="num">Hz</th>
                <th>{t('contest.col.msg')}</th>
                <th>{t('contest.col.country')}</th>
                <th className="num">{prefs.distanceUnit}</th>
                <th>{t('contest.col.new')}</th>
                <th className="num">{t('contest.col.value')}</th>
              </tr>
            </thead>
            <tbody>
              {righe.length === 0 ? (
                <tr>
                  <td colSpan={8} className="ctc-muted">{t('contest.noDecodes')}</td>
                </tr>
              ) : null}
              {righe.map(({ d, call, v }) => {
                const inQso = call === dx;
                const cls = inQso ? 'qso' : v?.doppio ? 'dup' : call === stella ? 'top' : '';
                // novita' come le etichette di Attivita' di banda: i colori delle etichette (--tag-*) del tema
                const novita = inQso ? (
                  <span className="badge badge--tag" style={tag('worked')}>{t('contest.inQso')}</span>
                ) : v?.doppio ? (
                  <span className="badge badge--tag badge--quiet">{t('contest.dupe')}</span>
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
                return (
                  <tr key={d.id} className={`ctc-r ${cls}`} onDoubleClick={() => void postReply(d.id)} title={v?.bande_utili.length ? v.bande_utili.join(' ') : undefined}>
                    <td className="ctc-muted">{utc(d.time_ms)}</td>
                    <td className="num">{d.snr > 0 ? `+${d.snr}` : d.snr}</td>
                    <td className="num ctc-muted">{d.freq_hz}</td>
                    <td>{pastiglia(d.text, call)}</td>
                    <td className="ctc-country">
                      {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null} {d.country ?? ''}
                    </td>
                    <td className="num ctc-muted">{v && v.km > 0 ? formatDistance(v.km, prefs.distanceUnit) : ''}</td>
                    <td className="ctc-new">{novita}</td>
                    <td className="num">{v && v.punti > 0 ? <span className="badge badge--tag ctc-pill">{(call === stella ? '★ ' : '') + t('contest.pt', { n: v.punti })}</span> : v ? t('contest.pt', { n: v.punti }) : ''}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>

      <section className="dock-panel ctc-box">
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
                <span className={ora && transmitting ? 'ctc-val' : 'ctc-muted'}>{ora && transmitting ? t('contest.onAir') : t(k as 'contest.step.exch')}</span>
              </div>
            );
          })}
        </div>
        <div className="ctc-actions">
          <button type="button" className={`ctc-btn${autoseq ? ' ctc-btn--on' : ''}`} aria-pressed={autoseq} title={t('tip.contest.autoseq')} onClick={() => void postControl('autoseq', !autoseq)}>
            ● {t('contest.autoseq')}
          </button>
          <button type="button" className={`ctc-btn${txOn ? ' ctc-btn--on' : ''}`} aria-pressed={txOn} onClick={() => void postControl('enable_tx', !txOn)}>
            ● {t('contest.enableTx')}
          </button>
          <button type="button" className="ctc-btn" title={t('tip.contest.cq')} onClick={() => void postCq(s.sequenza?.parola_cq ?? '')}>
            {t('contest.cq')}
          </button>
          <button type="button" className="ctc-btn" disabled={!dx} title={t('tip.contest.rr73')} onClick={() => void postTxMessage(4)}>
            {t('contest.rr73')}
          </button>
          <button type="button" className="ctc-btn ctc-btn--stop" onClick={() => void postHalt()}>
            STOP TX
          </button>
        </div>
        {migliori.length ? (
          <div className="ctc-tip" title={t('tip.contest.next')}>
            {t('contest.next')}{' '}
            {migliori.slice(0, 3).map((x, i) => (
              <span key={x.call} className="ctc-next" onDoubleClick={() => void postReply(x.d.id)}>
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

      <Moltiplicatori r={r} t={t} banda={banda} righe={righe0} scegli={setCampo} />

      <LogContest t={t} qso={s.totali?.qso ?? 0} />

      <section className="dock-panel ctc-box">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2>{t('contest.tips')}</h2>
        </header>
        <Avvisi s={s} t={t} />
        <div className="ctc-rate">
          {(s.per_ora ?? []).map((n, i, all) => {
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
          <button type="button" className="ctc-btn" disabled title={t('contest.soon')}>
            {t('contest.training')}
          </button>
          <button type="button" className="ctc-btn" disabled title={t('contest.soon')}>
            {t('contest.analysis')}
          </button>
          <button type="button" className="ctc-btn ctc-btn--stop" title={t('tip.contest.end')} onClick={fine}>
            {t('contest.end')}
          </button>
        </div>
      </section>
    </main>
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

function Moltiplicatori({ r, t, banda, righe, scegli }: { r: Regole; t: T; banda: string; righe: Riga[]; scegli: (c: string) => void }) {
  const [scelta, setScelta] = useState(banda || r.bande.ammesse[0] || '');
  useEffect(() => {
    if (banda && r.bande.ammesse.includes(banda)) setScelta(banda);
  }, [banda, r.bande.ammesse]);
  const lav = useQuery({ queryKey: ['contest-lavorati', scelta], queryFn: () => getLavorati(scelta), refetchInterval: 15_000, retry: false, enabled: r.moltiplicatori.length > 0 }).data;
  const tipo = r.moltiplicatori[0]?.tipo ?? '';
  const valori = lav?.moltiplicatori[0]?.valori ?? [];
  const campiFatti = new Set(valori.map((v) => v.slice(0, 2).toUpperCase()));
  const inOnda = new Set(righe.filter((x) => x.grid).map((x) => x.grid.slice(0, 2).toUpperCase()));
  const aGriglia = tipo === 'campo2' || tipo === 'grid4';
  return (
    <section className="dock-panel ctc-box">
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2 title={t('tip.contest.missing')}>
        {t('contest.missing')} <span className="r">{scelta} · {tipo}</span>
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
          {aGriglia ? (
            <div className="ctc-grid">
              {Array.from({ length: 18 * 18 }, (_, i) => {
                const c = `${LETTERE[i % 18]}${LETTERE[17 - Math.floor(i / 18)]}`;
                const cls = campiFatti.has(c) ? 'f' : inOnda.has(c) ? 'a' : '';
                return <i key={c} className={cls} title={c} onClick={() => scegli(c)} />;
              })}
            </div>
          ) : (
            <div className="ctc-tip data">{valori.join(' ') || '—'}</div>
          )}
          <div className="ctc-legend">
            <span className="ctc-sq f" /> {t('contest.legend.done')} &nbsp; <span className="ctc-sq a" /> {t('contest.legend.onair')} &nbsp; <span className="ctc-sq" /> {t('contest.legend.missing')} · {t('contest.multTaken', { b: scelta, n: valori.length })}
          </div>
        </>
      )}
    </section>
  );
}

function LogContest({ t, qso }: { t: T; qso: number }) {
  const qc = useQueryClient();
  const log = useQuery({ queryKey: ['contest-log', qso], queryFn: getContestLog, refetchInterval: 10_000, retry: false }).data;
  const [modifica, setModifica] = useState<{ n: number; testo: string } | null>(null);
  const righe = [...(log?.qso ?? [])].reverse();
  const incerti = righe.filter((q) => q.stato === 'incerto').length;
  const aggiorna = () => {
    void qc.invalidateQueries({ queryKey: ['contest-log'] });
    void qc.invalidateQueries({ queryKey: ['contest'] });
  };
  const salva = (n: number, patch: Parameters<typeof modificaQso>[1]) => modificaQso(n, patch).then(aggiorna).catch((e: unknown) => toast(String(e), 'error'));
  return (
    <section className="dock-panel ctc-box">
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
        <table>
          <thead>
            <tr>
              <th>{t('contest.col.utc')}</th>
              <th>{t('contest.col.band')}</th>
              <th>{t('contest.col.call')}</th>
              <th>{t('contest.col.rcvd')}</th>
              <th className="num">{t('contest.col.points')}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {righe.map((q: LogQso) => (
              <tr key={q.n} className={q.doppio ? 'dup' : q.stato === 'incerto' ? 'warn' : q.stato === 'errore' ? 'dup' : undefined}>
                <td>{utcHm(q.time_ms)}</td>
                <td>{q.band}</td>
                <td>{q.call}</td>
                <td onClick={() => setModifica({ n: q.n, testo: q.exch_rcvd || q.grid })} className={q.scambio_ok ? undefined : 'ctc-bad'} title={q.modificato ? t('contest.edited') : undefined}>
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
                <td className={`num${q.molt_nuovi.length ? ' ctc-val' : ''}`}>
                  {q.doppio ? t('contest.dupe') : `${q.punti}${q.molt_nuovi.length ? ` · ${t('contest.multShort')}` : ''}${q.stato === 'incerto' ? ` · ${t('contest.state.incerto')}` : ''}`}
                </td>
                <td>
                  <select className="ctc-mini" aria-label={t('contest.state.valido')} value={q.stato} onChange={(e) => void salva(q.n, { stato: e.target.value as LogQso['stato'] })}>
                    {(['valido', 'incerto', 'errore'] as const).map((x) => (
                      <option key={x} value={x}>
                        {t(`contest.state.${x}`)}
                      </option>
                    ))}
                  </select>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
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
                {c.nome}
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
        {testo('grid6', t('contest.set.grid6'))}
        {testo('operatori', t('contest.set.operators'))}
        {testo('club', t('contest.set.club'))}
        {testo('nome', t('contest.set.name'))}
        {testo('location', t('contest.set.location'))}
        {testo('assistenza', t('contest.set.assisted'))}
        {testo('cabrillo_contest', t('contest.set.cabrillo'))}
        <label title={t('tip.contest.set.dates')}>
          {t('contest.set.start')}
          <input className="ctc-input" type="datetime-local" value={dataOra(imp.inizio_ms)} onChange={(e) => set('inizio_ms', daDataOra(e.target.value))} />
        </label>
        <label title={t('tip.contest.set.dates')}>
          {t('contest.set.end')}
          <input className="ctc-input" type="datetime-local" value={dataOra(imp.fine_ms)} onChange={(e) => set('fine_ms', daDataOra(e.target.value))} />
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
