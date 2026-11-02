import { foxCalls } from '../lib/fox';
import { DxPill } from './DxPill';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getControls, postHalt, postTxMessage } from '../api/client';
import { toast } from '../lib/toast';
import type { DecodedMessage, StationStatus, TxAlert } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';
import { InputLevel } from './InputLevel';
import { OutputLevel } from './OutputLevel';
import { QsoSwitches } from './QsoSwitches';
import { replyErrorMessage } from '../lib/replyError';
import { activeTx, nextTx, txMessages } from '../lib/txMessages';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Stato del motore: stazione DX, rapporto e messaggio in corso. */
  status?: StationStatus | null;
  /** Messaggi decodificati (i piu' recenti per primi): servono a capire a che punto e' il QSO. */
  decodes?: DecodedMessage[];
}

/** Il testo di un avviso di sicurezza della trasmissione. */
export function alertText(a: TxAlert, t: ReturnType<typeof useT>): string {
  if (a.kind === 'high_swr') return t('tx.alert.highSwr', { swr: (a.value ?? 0).toFixed(1), limit: (a.limit ?? 0).toFixed(1) });
  if (a.kind === 'fox_detected') return t('tx.alert.foxDetected', { call: a.detail || '—', hz: Math.round(a.value ?? 0) });
  if (a.kind === 'hound_off') return t('tx.alert.houndOff', { call: a.detail || '—' });
  if (a.kind === 'hound_on') return t(a.detail === 'split' ? 'tx.alert.houndSplit' : 'tx.alert.hound', { hz: Math.round(a.value ?? 0) });
  if (a.kind === 'inhibited') return t('tx.alert.inhibited', { holder: a.detail || '—' });
  return t('tx.alert.unrepresentable', { text: a.detail ?? '' });
}

/** Stato del QSO: stazione DX, i sei messaggi Tx con la fase in corso e il livello audio di uscita. */
export function TxPanel({ txAllowed, status = null, decodes = [] }: Props) {
  const t = useT();
  const pendingDx = useStation((s) => s.pendingDx);
  const engineTx = useEngineTx((s) => s.tx);
  const [note, setNote] = useState('');
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const skipTx1 = controls.data?.skip_tx1 === true;
  const hound = controls.data?.hound === true;
  const messages = status
    ? txMessages({ myCall: status.de_call, myGrid: status.de_grid, dxCall: pendingDx || status.dx_call || '', report: status.report ?? '' })
    : [];
  // I testi veri e la selezione stanno nella finestra del motore: se ha gia' la stazione scelta si usano quelli (con il rapporto
  // vero e il CQ come l'hai scritto), altrimenti restano i modelli. Appena scelta con un doppio clic una stazione il motore
  // ha ancora la precedente: finche' non la conferma valgono i modelli.
  const dx = (pendingDx ? '' : status?.dx_call ?? '').toUpperCase();
  const fresh = engineTx !== null && dx !== '' && engineTx.messages[1]?.toUpperCase().includes(dx) === true;
  if (status && engineTx) {
    const cq = engineTx.messages[5]?.trim();
    if (cq) messages[5] = cq;
    if (fresh) engineTx.messages.slice(0, 5).forEach((m, i) => m.trim() !== '' && (messages[i] = m.trim()));
  }
  // Come nel motore: il pallino selezionato e' il messaggio da cui si trasmette (lo sceglie lui, avanzando da solo con AutoSeq e
  // «Salta Tx1», o lo scegli tu con un clic). Acceso forte mentre trasmette, con il bordo mentre aspetta il suo turno.
  const selected = engineTx?.selected != null ? engineTx.selected - 1 : -1;
  const transmitting = status?.transmitting === true;
  // senza la lettura del motore (altra radio, motore non raggiungibile) si ricostruisce: il testo trasmesso, poi la fase del QSO
  let active = selected >= 0 ? (transmitting ? selected : -1) : activeTx(messages, status?.tx_message);
  const sent = status?.tx_message?.trim();
  if (active >= 0 && sent) messages[active] = sent;
  let next = selected >= 0 && !transmitting ? selected : -1;
  if (status && selected < 0 && active < 0) {
    next = nextTx(decodes, status.de_call, pendingDx || status.dx_call || '');
    if (next === 0 && skipTx1) next = 1;
  }
  if (active < 0 && next < 0 && transmitting && selected >= 0) active = selected;

  /** Sceglie a mano il messaggio da cui il motore parte: si vede subito e, se il comando fallisce, torna quello di prima. */
  const choose = async (i: number) => {
    if (i === 5 && hound) return;
    if (messages[i] === '' || i === selected) return;
    useEngineTx.getState().choose(i + 1);
    try {
      await postTxMessage(i + 1);
      setNote('');
    } catch (error) {
      useEngineTx.getState().unchoose();
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="tx-panel">
      <div className="tx-main tx-split">
        <section className="tx-left tx-block" aria-label={t('ctl.sec.dx')}>
      {status && (
        <>
          <div className="tx-dx">
            {pendingDx || status.dx_call ? <DxPill status={status} /> : <strong className="data tx-dx-call">—</strong>}
            {/* accanto al DX: RX verde in ascolto; in onda TX rosso lampeggiante, ed e' anche lo STOP (un clic ferma la trasmissione) */}
            {status.transmitting ? (
              <button type="button" className="rxtx-flag rxtx-flag--stop" data-state="tx" title={t('tip.txStop')} aria-label={t('stop.button')} onClick={() => void postHalt().catch(() => toast(t('stop.fail'), 'error'))}>
                TX
              </button>
            ) : (
              <span className="rxtx-flag" data-state="rx" role="status" title={t('tip.onair')}>
                RX
              </span>
            )}
            <span className="data muted tx-freqs">
              <span>{`Tx ${status.tx_df ?? '—'} Hz`}</span> <span>{`Rx ${status.rx_df ?? '—'} Hz`}</span>
            </span>
            {/* in Hound lo si vede sempre accanto alle frequenze: chiama sopra 1000 Hz e risponde solo al Fox */}
            {status.dx_call && foxCalls(decodes).has(status.dx_call.toUpperCase()) ? (
              <span className="hound-badge hound-badge--fox" role="status" title={t('tip.foxBadge')}>
                {controls.data?.hound === true ? t('tx.foxHoundOn') : t('tx.foxHoundOff')}
              </span>
            ) : controls.data?.hound === true ? (
              <span className="hound-badge" role="status" title={t('tip.houndBadge')}>
                Hound
              </span>
            ) : null}
          </div>
          {/* gli avvisi di sicurezza della trasmissione: restano finche' il motore li tiene nello stato */}
          {status.tx_alerts && status.tx_alerts.length > 0 ? (
            <div className="tx-alerts" role="alert">
              {status.tx_alerts.map((a) => (
                <p key={a.kind} className="tx-alert" title={t('tip.tx.alert')}>
                  {alertText(a, t)}
                </p>
              ))}
            </div>
          ) : null}
          {/* Hound: il Fox ha mandato un codice OTP e il servizio ha risposto (verde verificato, rosso non valido) */}
          {status.fox_verification ? (
            <p className="tx-fox" data-ok={status.fox_verification.verified} role="status" title={t('tip.tx.fox')}>
              {t(status.fox_verification.verified ? 'tx.foxVerified' : 'tx.foxInvalid', { call: status.fox_verification.call })}
            </p>
          ) : null}
          {/* le stazioni in coda, sempre visibili: un pallino blu col posto e il nominativo */}
          {status.queue && status.queue.length > 0 ? (
            <div className="tx-queue" title={t('tip.tx.queue')}>
              <span className="tx-queue-label">{t('tx.queue')}</span>
              {status.queue.map((call, i) => (
                <span key={call} className="tx-queue-item" title={t('tip.row.queuedPos', { n: i + 1 })}>
                  <span className="queue-dot">{i + 1}</span>
                  <span className="data">{call}</span>
                </span>
              ))}
            </div>
          ) : null}
          <ol className="tx-list" role="radiogroup" aria-label={t('tx.list')} title={t('tip.tx.list')}>
            {messages.map((m, i) => (
              <li
                key={i}
                role="radio"
                tabIndex={m === '' ? -1 : 0}
                aria-checked={i === active || i === next}
                aria-disabled={m === '' || (i === 5 && hound) ? true : undefined}
                onClick={() => void choose(i)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    void choose(i);
                  }
                }}
                aria-current={i === active ? 'true' : undefined}
                data-next={i === next && m !== '' ? 'true' : undefined}
                // in Hound il Tx1 non si salta mai (WSJT-X 3.3, elide_tx1_not_allowed) e il CQ non si trasmette: niente riga barrata, Tx6 spento
                data-skipped={i === 0 && skipTx1 && !hound && m !== '' ? 'true' : undefined}
                title={i === 0 && skipTx1 && !hound ? t('tip.ctl.skip_tx1') : i === 5 && hound ? t('tip.tx.cqHound') : undefined}
                className={m === '' ? 'tx-empty' : i === 5 && hound ? 'tx-empty tx-off' : undefined}
              >
                <span className="tx-n">{i + 1}</span>
                <span className="data tx-msg">{m}</span>
                <span className="tx-step">{t(`tx.step.${i + 1}` as Key)}</span>
              </li>
            ))}
          </ol>
          {note ? <p className="live-note" role="status">{note}</p> : null}
        </>
      )}
        </section>
        <QsoSwitches txAllowed={txAllowed} />
      </div>
      <InputLevel />
      <OutputLevel txAllowed={txAllowed} />
    </div>
  );
}
