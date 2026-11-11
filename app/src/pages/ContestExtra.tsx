/** Le linguette «Allenamento» e «Analisi» della pagina Contest (prototipo approvato: anteprime/contest-extra-prototipo.html). */
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getAnalisi, getRegole, type ContestStato } from '../api/contest';
import type { T } from '../i18n';
import { Simulatore, type Livello, type SimPunteggio } from '../lib/contestSim';
import { useStation } from '../state/station';

const utcH = (ms: number) => new Date(ms).toISOString().slice(11, 13);
const utc = (ms: number) => new Date(ms).toISOString().slice(11, 19);
const numero = (n: number | undefined) => (n ?? 0).toLocaleString();

/* ---------------------------------------------------------------- Analisi dopo la gara */

export function AnalisiGara({ s, t }: { s?: ContestStato; t: T }) {
  const [scelta, setScelta] = useState('');
  // l'edizione: il giorno d'inizio (`2026-11-10`) o, per i vecchi log, l'anno
  const [id, anno] = scelta ? [scelta.slice(0, scelta.lastIndexOf('|')), scelta.slice(scelta.lastIndexOf('|') + 1)] : [undefined, undefined];
  // si chiede solo con la linguetta aperta (il componente esiste solo allora) e senza ripetere: il log di un'edizione chiusa non cambia
  const q = useQuery({ queryKey: ['contest-analisi', scelta, s?.totali?.qso ?? 0], queryFn: () => getAnalisi(id, anno), retry: false, staleTime: 30_000 }).data;
  const a = q?.analisi ?? null;
  const nome = (cid: string) => s?.contest.find((c) => c.id === cid)?.nome ?? cid;
  const ore = a?.ore ?? [];
  const max = Math.max(1, ...ore.map((o) => o.qso));
  const conQso = ore.filter((o) => o.qso > 0);
  const migliore = conQso.reduce<number | null>((m, o) => (m === null || o.qso > m ? o.qso : m), null);
  const peggiore = conQso.length > 1 ? conQso.reduce((m, o) => Math.min(m, o.qso), Infinity) : null;
  const c = a?.consiglio ?? null;

  return (
    <section className="dock-panel ctc-box ctc-extra-wide">
      <header className="dock-grip dock-grip--fixed">
        <span className="dock-grip-dots" aria-hidden="true" />
        <h2>
          {t('contest.an.title')}
          {a ? ` · ${a.nome} ${a.anno}` : ''}
          <span className="r">
            <select className="ctc-input" value={scelta} onChange={(e) => setScelta(e.target.value)} aria-label={t('contest.an.edition')}>
              <option value="">{t('contest.an.current')}</option>
              {(q?.edizioni ?? []).map((e) => (
                <option key={`${e.id}|${e.anno}`} value={`${e.id}|${e.anno}`}>
                  {nome(e.id)} {e.anno}
                </option>
              ))}
            </select>
          </span>
        </h2>
      </header>
      {!a || !a.totali.qso ? (
        <p className="ctc-tip">{t('contest.an.empty')}</p>
      ) : (
        <div className="ctc-scroll">
          <div className="ctc-kpi">
            <div><small>{t('contest.an.qso')}</small><b>{numero(a.totali.qso)}</b></div>
            <div><small>{t('contest.an.mult')}</small><b className="ctc-mult">{numero(a.totali.molt)}</b></div>
            <div><small>{t('contest.an.score')}</small><b className="ctc-val">{numero(a.totali.punteggio)}</b></div>
            <div><small>{t('contest.an.uncertain')}</small><b className={a.totali.incerti ? 'ctc-warn' : undefined}>{numero(a.totali.incerti)}</b></div>
            <div><small>{t('contest.an.hours')}</small><b>{a.ore_attive.toFixed(1)}</b></div>
          </div>
          <div className="ctc-rate ctc-rate--an">
            {ore.map((o) => (
              <i
                key={o.ora_ms}
                className={o.qso === migliore ? 'best' : o.qso === peggiore ? 'low' : undefined}
                style={{ height: `${Math.max(3, (o.qso / max) * 100)}%` }}
                title={`${utcH(o.ora_ms)} UTC · ${o.qso} QSO${o.banda ? ` · ${o.banda}` : ''}`}
              />
            ))}
          </div>
          <div className="ctc-tip">{t('contest.an.rateHint')}</div>
          <table className="data">
            <thead>
              <tr>
                <th>{t('contest.an.band')}</th>
                <th className="num">QSO</th>
                <th className="num">{t('contest.an.mult')}</th>
                <th>{t('contest.an.best')}</th>
                <th>{t('contest.an.lost')}</th>
              </tr>
            </thead>
            <tbody>
              {a.bande.map((b) => {
                const top = b.rate_migliore > 0 && b.rate_migliore === Math.max(...a.bande.map((x) => x.rate_migliore));
                return (
                  <tr key={b.banda}>
                    <td>{b.banda}</td>
                    <td className="num">{b.qso}</td>
                    <td className="num ctc-mult">{b.molt}</td>
                    <td className={top ? 'ctc-val' : undefined}>{b.rate_migliore ? `${b.rate_migliore}/h${b.ora_migliore_ms !== null ? ` · ${utcH(b.ora_migliore_ms)} UTC` : ''}` : '—'}</td>
                    <td className={b.perso_min >= 20 ? 'ctc-warn' : undefined}>{b.perso_min ? t('contest.an.lostVal', { m: b.perso_min, n: b.buchi }) : '—'}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <div className={`ctc-alert ${c ? 'ctc-alert--o' : 'ctc-alert--g'}`}>
            {!c
              ? t('contest.an.tipNone')
              : c.tipo === 'banda'
                ? t('contest.an.tipBand', { h: utcH(c.ora_ms), a: c.banda_su, n: c.qso_su, b: c.banda_meglio, hb: utcH(c.ora_meglio_ms), r: c.rate_meglio })
                : t('contest.an.tipGap', { h: utc(c.ora_ms).slice(0, 5), b: c.banda, m: c.minuti })}
          </div>
        </div>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------- Allenamento */

// il simulatore resta vivo se si cambia linguetta (senza timer: al ritorno riprende dal periodo in corso)
let corrente: Simulatore | null = null;
let corrente_id = '';
const CHIAVE_ULTIMO = 'aurora.contest.allenamento.ultimo';

interface Ultimo {
  qso: number;
  minuti: number;
  rate: number;
  nil: number;
}

function leggiUltimo(): Ultimo | null {
  try {
    const v = localStorage.getItem(CHIAVE_ULTIMO);
    return v ? (JSON.parse(v) as Ultimo) : null;
  } catch {
    return null;
  }
}

function salvaUltimo(u: Ultimo) {
  try {
    localStorage.setItem(CHIAVE_ULTIMO, JSON.stringify(u));
  } catch {
    // memoria del browser non disponibile: si perde solo il riepilogo
  }
}

export function Allenamento({ s, t }: { s?: ContestStato; t: T }) {
  const status = useStation((x) => x.status);
  const elenco = s?.contest ?? [];
  const [id, setId] = useState(s?.attivo ?? corrente_id ?? elenco[0]?.id ?? '');
  const [livello, setLivello] = useState<Livello>('medio');
  const [minuti, setMinuti] = useState(30);
  const [, setGiro] = useState(0);
  const [ultimo, setUltimo] = useState<Ultimo | null>(leggiUltimo);
  const scelto = id || elenco[0]?.id || '';
  const regole = useQuery({ queryKey: ['contest-regole', scelto], queryFn: () => getRegole(scelto), enabled: !!scelto, retry: false, staleTime: 300_000 }).data;
  const sim = corrente;
  const attivo = !!sim && !sim.finito;

  // un solo timer, e solo mentre l'allenamento corre con la linguetta aperta
  useEffect(() => {
    if (!attivo) return;
    const h = window.setInterval(() => {
      const c = corrente;
      if (!c) return;
      if (c.finito) {
        const p = c.punteggio();
        const u = { qso: p.qso, minuti: Math.round((c.fine_ms - c.inizio_ms) / 60_000), rate: p.rate, nil: p.nilEvitati };
        salvaUltimo(u);
        setUltimo(u);
        setGiro((g) => g + 1);
        return;
      }
      if (c.avanza()) setGiro((g) => g + 1);
    }, 1000);
    return () => window.clearInterval(h);
  }, [attivo]);

  const avvia = () => {
    const r = regole?.regole;
    corrente = new Simulatore({
      campi: r?.scambio.ricevuto ?? ['grid4'],
      mio: status?.de_call ?? '',
      mioGrid: status?.de_grid ?? '',
      parolaCq: regole?.impostazioni.parola_cq || r?.messaggi.parola_cq || 'TEST',
      livello,
      minuti,
    });
    corrente_id = scelto;
    setGiro((g) => g + 1);
  };
  const ferma = () => {
    corrente = null;
    setGiro((g) => g + 1);
  };
  const p: SimPunteggio | null = sim ? sim.punteggio() : null;
  const righe = sim ? [...sim.righe].reverse() : [];
  const q = sim?.inQso() ?? null;
  const restano = sim ? Math.max(0, sim.fine_ms - Date.now()) : 0;
  const chip = <V,>(v: V, cur: V, label: string, set: (x: V) => void) => (
    <button type="button" className={v === cur ? 'on' : undefined} disabled={attivo} onClick={() => set(v)}>
      {label}
    </button>
  );

  return (
    <div className="ctc-extra">
      <section className="dock-panel ctc-box">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2>
            {t('contest.sim.title')} <span className="r ctc-yellow">{t('contest.sim.noTx')}</span>
          </h2>
        </header>
        <div className="ctc-scroll">
          <div className="ctc-form ctc-sim-form">
            <label>
              {t('contest.sim.contest')}
              <select className="ctc-input" value={scelto} disabled={attivo} onChange={(e) => setId(e.target.value)}>
                {elenco.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.nome}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t('contest.sim.pileup')}
              <span className="ctc-tabs ctc-tabs--flat">
                {chip<Livello>('leggero', livello, t('contest.sim.light'), setLivello)}
                {chip<Livello>('medio', livello, t('contest.sim.medium'), setLivello)}
                {chip<Livello>('fitto', livello, t('contest.sim.heavy'), setLivello)}
              </span>
            </label>
            <label>
              {t('contest.sim.duration')}
              <span className="ctc-tabs ctc-tabs--flat">
                {chip(15, minuti, t('contest.sim.min', { n: 15 }), setMinuti)}
                {chip(30, minuti, t('contest.sim.min', { n: 30 }), setMinuti)}
                {chip(60, minuti, t('contest.sim.hour'), setMinuti)}
              </span>
            </label>
          </div>
          <div className="ctc-actions">
            {attivo ? (
              <button type="button" className="ctc-btn ctc-btn--stop" onClick={ferma}>
                {t('contest.sim.stop')}
              </button>
            ) : (
              <button type="button" className="ctc-btn ctc-btn--on" disabled={!scelto || !regole} onClick={avvia}>
                {t('contest.sim.start')}
              </button>
            )}
            {attivo ? <span className="ctc-muted">{t('contest.sim.left', { n: Math.ceil(restano / 60_000) })}</span> : null}
          </div>
          <p className="ctc-tip">{t('contest.sim.hint')}</p>
          {p ? (
            <div className="ctc-kpi">
              <div><small>QSO</small><b>{p.qso}</b></div>
              <div><small>{t('contest.an.mult')}</small><b className="ctc-mult">{p.molt}</b></div>
              <div><small>{t('contest.an.score')}</small><b className="ctc-val">{numero(p.punteggio)}</b></div>
              <div><small>{t('contest.sim.rate')}</small><b>{p.rate}/h</b></div>
              <div><small>{t('contest.sim.dupes')}</small><b className={p.doppi ? 'ctc-warn' : undefined}>{p.doppi}</b></div>
              <div><small>{t('contest.sim.errors')}</small><b className={p.errori ? 'ctc-bad' : undefined}>{p.errori}</b></div>
              <div><small>{t('contest.an.uncertain')}</small><b className={p.incerti ? 'ctc-warn' : undefined}>{p.incerti}</b></div>
              <div><small>{t('contest.sim.nil')}</small><b className="ctc-val">{p.nilEvitati}</b></div>
            </div>
          ) : null}
          {sim?.finito ? <div className="ctc-alert ctc-alert--g">{t('contest.sim.done')}</div> : null}
          {ultimo && !attivo ? <div className="ctc-alert ctc-alert--g">{t('contest.sim.last', { q: ultimo.qso, m: ultimo.minuti, r: ultimo.rate, n: ultimo.nil })}</div> : null}
        </div>
      </section>

      <section className="dock-panel ctc-box ctc-dec">
        <header className="dock-grip dock-grip--fixed">
          <span className="dock-grip-dots" aria-hidden="true" />
          <h2>
            {t('contest.decodes')} <span className="badge badge--tag ctc-sim-badge">{t('contest.sim.badge')}</span>
            <span className="r">
              {q ? t('contest.sim.qso', { c: q.call }) : ''}
              <button type="button" className="ctc-btn" disabled={!attivo || !q} onClick={() => { sim?.rr73(); setGiro((g) => g + 1); }}>
                RR73
              </button>
            </span>
          </h2>
        </header>
        <div className="ctc-scroll data">
          <table>
            <thead>
              <tr>
                <th>UTC</th>
                <th className="num">dB</th>
                <th className="num">Hz</th>
                <th>{t('contest.col.msg')}</th>
                <th>{t('contest.col.new')}</th>
              </tr>
            </thead>
            <tbody>
              {righe.length === 0 ? (
                <tr>
                  <td colSpan={5} className="ctc-muted">{attivo ? t('contest.sim.waiting') : t('contest.sim.hint')}</td>
                </tr>
              ) : null}
              {righe.map((r) => {
                const cls = r.tx ? 'ctc-sim-tx' : r.call === q?.call ? 'qso' : r.doppio ? 'dup' : '';
                return (
                  <tr key={r.id} className={`ctc-r ${cls}`} onDoubleClick={() => { if (!r.tx) { sim?.rispondi(r.call); setGiro((g) => g + 1); } }}>
                    <td className="ctc-muted">{utc(r.time_ms)}</td>
                    <td className="num">{r.tx ? '' : r.snr > 0 ? `+${r.snr}` : r.snr}</td>
                    <td className="num ctc-muted">{r.tx ? '' : r.freq_hz}</td>
                    <td>{r.tx ? `${t('contest.sim.you')} › ${r.text}` : r.text}</td>
                    <td className="ctc-new">
                      {r.doppio ? <span className="badge badge--tag badge--quiet">{t('contest.dupe')}</span> : null}
                      {r.sbagliato ? <span className="badge badge--tag ctc-bad">{t('contest.sim.wrong')}</span> : null}
                      {r.nuovo ? <span className="badge badge--tag" style={{ background: 'var(--tag-new-grid)', color: 'var(--tag-new-grid-ink)' }}>{r.nuovo}</span> : null}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
