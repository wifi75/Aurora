import { DxPill } from './DxPill';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getControls, postTxMessage } from '../api/client';
import type { DecodedMessage, StationStatus } from '../api/schemas';
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

/** Stato del QSO: stazione DX, i sei messaggi Tx con la fase in corso e il livello audio di uscita. */
export function TxPanel({ txAllowed, status = null, decodes = [] }: Props) {
  const t = useT();
  const pendingDx = useStation((s) => s.pendingDx);
  const engineTx = useEngineTx((s) => s.tx);
  const [note, setNote] = useState('');
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const skipTx1 = controls.data?.skip_tx1 === true;
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
            <span className="data muted tx-freqs">
              <span>{`Tx ${status.tx_df ?? '—'} Hz`}</span> <span>{`Rx ${status.rx_df ?? '—'} Hz`}</span>
            </span>
          </div>
          <ol className="tx-list" role="radiogroup" aria-label={t('tx.list')} title={t('tip.tx.list')}>
            {messages.map((m, i) => (
              <li
                key={i}
                role="radio"
                tabIndex={m === '' ? -1 : 0}
                aria-checked={i === active || i === next}
                aria-disabled={m === '' ? true : undefined}
                onClick={() => void choose(i)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    void choose(i);
                  }
                }}
                aria-current={i === active ? 'true' : undefined}
                data-next={i === next && m !== '' ? 'true' : undefined}
                data-skipped={i === 0 && skipTx1 && m !== '' ? 'true' : undefined}
                title={i === 0 && skipTx1 ? t('tip.ctl.skip_tx1') : undefined}
                className={m === '' ? 'tx-empty' : undefined}
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
