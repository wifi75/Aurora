import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, type ReactNode } from 'react';
import { getClusterSpots, getControls, getLog4om, postControl, postCq, type ClusterSpot } from '../api/client';
import type { DecodedMessage } from '../api/schemas';
import { tuneSpot } from '../components/ClusterPanel';
import { DecodeList } from '../components/DecodeList';
import { useCatExtras } from '../components/radio-display/RadioExtras';
import { displayModel } from '../lib/radioDisplay';
import { StopButton } from '../components/StopButton';
import { chooseTxMessage, useTxSequence } from '../components/TxPanel';
import { useT } from '../i18n';
import { filterSpots } from '../lib/cluster';
import { countryOfCall } from '../lib/dxCountry';
import { flagCode } from '../lib/flags';
import { BAND_NAMES, bandOf } from '../lib/format';
import { distanceKm, gridCenter } from '../lib/geo';
import { periodPosition } from '../lib/period';
import { replyErrorMessage } from '../lib/replyError';
import { browserStorage } from '../lib/theme';
import { toast } from '../lib/toast';
import { senderOf } from '../lib/wishlist';
import { useStation } from '../state/station';
import '../styles/phone.css';

/** Le bande del telecomando: quelle HF fino ai 6 m, come il selettore della radio. */
const PHONE_BANDS = BAND_NAMES.filter((b) => b !== '2m');
const PHONE_MODES = ['FT8', 'FT4'] as const;
const OPEN_KEY = 'aurora-phone-open';
/** La descrizione corta di ciascuno dei sei messaggi Tx. */
const TX_KINDS = ['phone.kind.grid', 'phone.kind.report', 'phone.kind.rreport', 'phone.kind.rr73', 'phone.kind.73', 'phone.kind.cq'] as const;

interface Props {
  /** La cascata con le barre Rx e Tx, la stessa della pagina a finestre (qui bassa). */
  waterfall: ReactNode;
  /** I messaggi ricevuti gia' filtrati, con le nostre righe Tx in mezzo (i piu' recenti per primi). */
  decodes: DecodedMessage[];
  onReply(id: number): void;
  onQueue(id: number): void;
  pickBand(band: string): void;
  pickMode(mode: string): void;
  /** La radio si comanda dal CAT: senza, banda, modo e spot non si cambiano da qui. */
  rigControl: boolean;
  /** La trasmissione e' aperta (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
}

/** Quali riquadri richiudibili sono aperti (cluster e log), ricordato sul telefono. */
function readOpen(): { cluster: boolean; log: boolean } {
  try {
    const raw = browserStorage()?.getItem(OPEN_KEY);
    const parsed = raw ? (JSON.parse(raw) as { cluster?: boolean; log?: boolean }) : {};
    return { cluster: parsed.cluster === true, log: parsed.log === true };
  } catch {
    return { cluster: false, log: false };
  }
}

/** I secondi del periodo, aggiornati quattro volte al secondo perche' la barretta scorra. */
function usePeriod(mode: string) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  return periodPosition(now, mode || 'FT8');
}

const pad2 = (n: number) => String(n).padStart(2, '0');

/**
 * La pagina Stazione sul telefono: un telecomando compatto al posto dei pannelli. Dati e comandi sono quelli della pagina a finestre
 * (li passa Live): qui cambia solo la disposizione, dall'alto in basso come nel prototipo approvato.
 */
export function PhoneLive({ waterfall, decodes, onReply, onQueue, pickBand, pickMode, rigControl, txAllowed }: Props) {
  const t = useT();
  const client = useQueryClient();
  const status = useStation((s) => s.status);
  const allDecodes = useStation((s) => s.decodes);
  const pendingBand = useStation((s) => s.pendingBand);
  const transmitting = status?.transmitting === true;
  const mode = (status?.mode ?? '').toUpperCase();
  const period = usePeriod(mode);
  const seconds = `${pad2(period.seconds)}/${pad2(Math.round(period.length))}`;
  const band = pendingBand ?? (status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null);
  const [open, setOpen] = useState(readOpen);
  const toggleBox = (box: 'cluster' | 'log') => {
    const next = { ...open, [box]: !open[box] };
    setOpen(next);
    try {
      browserStorage()?.setItem(OPEN_KEY, JSON.stringify(next));
    } catch {
      // senza archivio alla prossima visita ripartono chiusi
    }
  };

  const seq = useTxSequence(status, allDecodes);
  // potenza: le stesse azioni pwr_down/pwr_up del display della radio sul PC; senza `power_set` la radio non la cambia
  const { rig, waiting, click } = useCatExtras();
  const powerSet = rig?.extras?.power_set ?? null;
  const poW = displayModel(status).meters?.power_w ?? null;
  const [pwrOpen, setPwrOpen] = useState(false);
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const txOn = controls.data?.enable_tx === true;
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const spots = cluster.data ? filterSpots(cluster.data.spots, { bands: ['now'], modes: mode ? [mode] : [], search: '' }, band).slice(0, 5) : [];
  const log = useQuery({ queryKey: ['log4om', 3], queryFn: () => getLog4om(3), refetchInterval: 10_000, retry: false });

  const fail = (error: unknown) => toast(replyErrorMessage(error, t), 'error');

  // in trasmissione si chiede conferma: un cambio di banda o di modo interrompe il QSO in corso
  const onBand = (next: string) => {
    if (next === band) return;
    if (transmitting && !window.confirm(t('phone.confirmBand', { band: next }))) return;
    pickBand(next);
  };
  const onMode = (next: string) => {
    if (next === mode) return;
    if (transmitting && !window.confirm(t('phone.confirmMode', { mode: next }))) return;
    pickMode(next);
  };

  const cq = () => {
    toast(t('tx.cqAsked'), 'ok');
    postCq('').catch(fail);
  };
  const toggleTx = async () => {
    try {
      await postControl('enable_tx', !txOn);
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, enable_tx: !txOn }));
      await client.invalidateQueries({ queryKey: ['controls'] });
    } catch (error) {
      fail(error);
    }
  };
  const pickTx = (i: number) => chooseTxMessage(i, seq.messages, seq.selected, seq.hound).catch(fail);
  const tune = async (spot: ClusterSpot) => {
    if (!rigControl) {
      toast(t('cluster.noRig'), 'info');
      return;
    }
    try {
      await tuneSpot(spot);
    } catch (error) {
      fail(error);
    }
  };

  // «In onda»: le stazioni sentite nell'ultimo periodo decodificato, ognuna una volta, dalla piu' forte
  const lastTime = allDecodes.find((d) => senderOf(d.text) !== null)?.time_ms;
  const onAir = allDecodes
    .filter((d) => d.time_ms === lastTime && senderOf(d.text) !== null)
    .filter((d, i, all) => all.findIndex((o) => senderOf(o.text)?.call === senderOf(d.text)?.call) === i)
    .sort((a, b) => b.snr - a.snr)
    .slice(0, 6);

  // a chi si trasmette: la stazione scelta, o un CQ se non c'e' o se esce il messaggio 6
  const dxCall = (seq.pendingDx || status?.dx_call || '').toUpperCase();
  const current = seq.active >= 0 ? seq.active : seq.next;
  const isCq = dxCall === '' || current === 5;
  const dxCountry = countryOfCall(allDecodes, dxCall);
  const dxFlag = flagCode(dxCountry);
  const dxGrid = seq.pendingDx ? '' : (status?.dx_grid ?? '');
  const from = status ? gridCenter(status.de_grid) : null;
  const to = dxGrid ? gridCenter(dxGrid) : null;
  const km = from && to ? distanceKm(from, to) : null;
  const dxSnr = allDecodes.find((d) => senderOf(d.text)?.call === dxCall)?.snr;
  const dxInfo = [dxCountry, dxGrid || null, km !== null ? `${km} km` : null, dxSnr !== undefined ? String(dxSnr) : null].filter(Boolean).join(' · ');

  return (
    <div className={`phone-live${transmitting ? ' phone-live--tx' : ''}`}>
      <div className="phone-top">
        <span className="data phone-freq">{status && status.dial_hz > 0 ? (status.dial_hz / 1e6).toFixed(3) : '—'}</span>
        <span className={`phone-pill phone-pill--${transmitting ? 'tx' : 'rx'}`}>{transmitting ? t('status.tx') : t('status.rx')}</span>
        {/* banda e modo: due tendine piccole nella barra, con l'area di tocco allargata dal padding */}
        <label className="phone-sel" title={t('phone.band')}>
          <select aria-label={t('phone.band')} value={band ?? ''} disabled={!rigControl} onChange={(e) => onBand(e.target.value)}>
            {band === null ? <option value="">—</option> : null}
            {PHONE_BANDS.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </label>
        <label className="phone-sel" title={t('phone.mode')}>
          <select aria-label={t('phone.mode')} value={mode} disabled={!rigControl} onChange={(e) => onMode(e.target.value)}>
            {mode !== '' && !(PHONE_MODES as readonly string[]).includes(mode) ? <option value={mode}>{mode}</option> : null}
            {PHONE_MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </label>
        {/* la potenza: impostata in % come il display della radio, in trasmissione quella misurata (PO); un tocco apre − e + */}
        {powerSet != null || poW != null ? (
          <span className="phone-pwr-host">
            <button type="button" className="phone-pwr data" aria-expanded={pwrOpen} disabled={powerSet == null} onClick={() => setPwrOpen(!pwrOpen)}>
              {transmitting && poW != null ? `${Math.round(poW)} W` : powerSet != null ? `PWR ${Math.round(powerSet * 100)}%` : `${Math.round(poW ?? 0)} W`}
            </button>
            {pwrOpen && powerSet != null ? (
              <span className="phone-pwr-pop" role="group" aria-label="PWR">
                <button type="button" aria-label={t('rx.act.pwrDown')} disabled={waiting !== null} onClick={() => void click('pwr_down')}>
                  −
                </button>
                <span className="data">{Math.round(powerSet * 100)}%</span>
                <button type="button" aria-label={t('rx.act.pwrUp')} disabled={waiting !== null} onClick={() => void click('pwr_up')}>
                  +
                </button>
              </span>
            ) : null}
          </span>
        ) : null}
        <span className="data phone-secs">{seconds}</span>
      </div>
      <div className="phone-prog" aria-hidden="true">
        <i style={{ width: `${Math.round(period.fraction * 100)}%` }} />
      </div>

      <div className="phone-wf">{waterfall}</div>

      {/* in ricezione: chi si sente adesso */}
      {transmitting ? null : (
        <div className="phone-now">
          <span className="muted">{t('phone.onAir')}</span>
          {onAir.length > 0 ? (
            onAir.map((d) => {
              const flag = flagCode(d.country);
              return (
                <span key={d.id} className="phone-now-item">
                  {flag ? <span className={`fi fi-${flag} phone-flag`} aria-hidden="true" /> : null}
                  <span className="data phone-now-call">{senderOf(d.text)?.call}</span>
                  <span className="data muted">{d.snr}</span>
                  {d.country ? (
                    <span className="phone-country" title={d.country}>
                      {d.country}
                    </span>
                  ) : null}
                </span>
              );
            })
          ) : (
            <span className="muted">{t('phone.nobody')}</span>
          )}
        </div>
      )}

      {/* i sei messaggi Tx, sempre visibili come sul PC: un tocco sceglie il prossimo; in trasmissione il riquadro diventa rosso */}
      <section className={`phone-txbox${transmitting ? ' phone-txbox--tx' : ''}`} aria-live="polite">
        <div className="phone-txh">
          <span>{transmitting ? `${t('phone.transmitting')} · ${status?.tx_df ?? '—'} Hz` : `${t('phone.txMessages')} · ${status?.tx_df ?? '—'} Hz`}</span>
          {transmitting ? <span className="data">{seconds}</span> : null}
        </div>
        {isCq ? (
          <div className="phone-dxline">
            <b>{t('phone.cqCall')}</b>
          </div>
        ) : (
          <div className="phone-dxline">
            {transmitting ? <span>{t('phone.calling')}</span> : <span>DX</span>}
            <b className="data">{dxCall}</b>
            {dxFlag ? <span className={`fi fi-${dxFlag} phone-flag`} aria-hidden="true" /> : null}
            <span className="phone-dxinfo">{dxInfo}</span>
          </div>
        )}
        <ol className="phone-txgrid">
          {seq.messages.map((m, i) => {
            const done = transmitting && current >= 0 && current < 5 && i < current;
            const cur = i === current || (!transmitting && current < 0 && i === seq.selected);
            return (
              <li key={i}>
                <button type="button" className={`phone-txl${cur ? ' phone-txl--cur' : ''}${done ? ' phone-txl--done' : ''}`} disabled={m === ''} onClick={() => void pickTx(i)}>
                  <b>{i + 1}</b>
                  <span className="data">{m || '—'}</span>
                  <em>{t(TX_KINDS[i] ?? 'phone.kind.cq')}</em>
                </button>
              </li>
            );
          })}
        </ol>
        {transmitting ? (
          <div className="phone-txbar" aria-hidden="true">
            <i style={{ width: `${Math.round(period.fraction * 100)}%` }} />
          </div>
        ) : null}
      </section>

      <div className="phone-list">
        <DecodeList decodes={decodes} myGrid={status?.de_grid} myCall={status?.de_call} dxCall={status?.dx_call} dxGrid={status?.dx_grid} onReply={onReply} onQueue={onQueue} tapReply forceFlag />
      </div>

      <section className="phone-box">
        <button type="button" className="phone-bh" aria-expanded={open.cluster} onClick={() => toggleBox('cluster')}>
          <span>{t('phone.cluster', { band: band ?? '—', mode: mode || '' })}</span>
          <span aria-hidden="true">{open.cluster ? '▾' : '▸'}</span>
        </button>
        {open.cluster ? (
          spots.length === 0 ? (
            <p className="muted phone-empty">{t('phone.noSpots')}</p>
          ) : (
            spots.map((s) => {
              const flag = flagCode(s.country);
              return (
                <button key={s.id} type="button" className="phone-row" onClick={() => void tune(s)}>
                  <span className="data phone-row-t">{(s.freq_khz / 1000).toFixed(3)}</span>
                  {flag ? <span className={`fi fi-${flag} phone-flag`} aria-hidden="true" /> : <span className="phone-flag" />}
                  <span className="data phone-row-m">{s.dx_call}</span>
                  <span className="muted">{s.time_utc.slice(0, 2)}:{s.time_utc.slice(2, 4)}</span>
                </button>
              );
            })
          )
        ) : null}
      </section>

      <section className="phone-box">
        <button type="button" className="phone-bh" aria-expanded={open.log} onClick={() => toggleBox('log')}>
          <span>{t('phone.log')}</span>
          <span aria-hidden="true">{open.log ? '▾' : '▸'}</span>
        </button>
        {open.log ? (
          log.isError ? (
            <p className="muted phone-empty">{t('log.unavailable')}</p>
          ) : log.data && log.data.qsos.length > 0 ? (
            log.data.qsos.slice(0, 3).map((q, i) => (
              <div key={`${q.date}-${q.call}-${i}`} className="phone-row phone-row--static">
                <span className="data phone-row-t">{q.date.slice(11, 16)}</span>
                <span className="data phone-row-m">{`${q.call} · ${q.band} ${q.mode}`}</span>
                <span className="data muted">{[q.rst_sent, q.rst_rcvd].filter(Boolean).join(' / ')}</span>
              </div>
            ))
          ) : (
            <p className="muted phone-empty">{log.data ? t('log.empty') : '…'}</p>
          )
        ) : null}
      </section>

      <div className="phone-bar">
        <button type="button" className="phone-btn phone-btn--go" disabled={!txAllowed} title={t('tip.tx.cq')} onClick={cq}>
          CQ
        </button>
        <button type="button" className={`phone-btn${txOn ? ' phone-btn--on' : ''}`} aria-pressed={txOn} disabled={!txAllowed || controls.isError} onClick={() => void toggleTx()}>
          {txOn ? t('phone.txOn') : t('ctl.enable_tx')}
        </button>
        <StopButton variant="phone" />
      </div>
    </div>
  );
}
