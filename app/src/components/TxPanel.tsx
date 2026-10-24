import { useState } from 'react';
import { postCq, postFreeText, postTxFrequency } from '../api/client';
import type { StationStatus } from '../api/schemas';
import type { DecodedMessage } from '../api/schemas';
import { activeTx, nextTx, txMessages } from '../lib/txMessages';
import { replyErrorMessage } from '../lib/replyError';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Frequenza audio di partenza (Hz). */
  initialFrequency?: number;
  /** Stato di JTDX: stazione DX, rapporto e messaggio in corso. */
  status?: StationStatus | null;
  /** Messaggi decodificati (i piu' recenti per primi): servono a capire a che punto e' il QSO. */
  decodes?: DecodedMessage[];
}

const MAX_TEXT = 13;

/** Comandi di trasmissione: CQ, messaggio libero e frequenza audio. Cio' che trasmette si abilita solo dal PC. */
export function TxPanel({ txAllowed, initialFrequency = 1500, status = null, decodes = [] }: Props) {
  const messages = status
    ? txMessages({ myCall: status.de_call, myGrid: status.de_grid, dxCall: status.dx_call ?? '', report: status.report ?? '' })
    : [];
  const active = activeTx(messages, status?.tx_message);
  const next = status && active < 0 ? nextTx(decodes, status.de_call, status.dx_call ?? '') : -1;
  const [dx, setDx] = useState(false);
  const [text, setText] = useState('');
  const [hz, setHz] = useState(String(initialFrequency));
  const [note, setNote] = useState('');

  const run = async (what: string, action: () => Promise<void>) => {
    try {
      await action();
      setNote(what);
    } catch (error) {
      setNote(replyErrorMessage(error));
    }
  };

  return (
    <div className="tx-panel">
      {status && (
        <>
          <div className="tx-dx">
            <span className="muted">DX</span> <strong className="data">{status.dx_call ?? '—'}</strong>
            {status.dx_grid ? <span className="data muted"> {status.dx_grid}</span> : null}
            <span className="data muted tx-freqs">
              <span>{`Tx ${status.tx_df ?? '—'} Hz`}</span> <span>{`Rx ${status.rx_df ?? '—'} Hz`}</span>
            </span>
          </div>
          <ol className="tx-list" aria-label="Messaggi Tx1-Tx6">
            {messages.map((m, i) => (
              <li key={i} aria-current={i === active ? 'true' : undefined} data-next={i === next && m !== '' ? 'true' : undefined} className={m === '' ? 'tx-empty' : undefined}>
                <span className="tx-n">Tx{i + 1}</span>
                <span className="data">{m}</span>
              </li>
            ))}
          </ol>
        </>
      )}
      {!txAllowed && (
        <p className="muted tx-note">
          Trasmissione chiusa: Aurora si avvia con AURORA_ALLOW_TX=0. Si apre solo dal PC, all'avvio.
        </p>
      )}
      <div className="tx-row">
        <button type="button" className="glass glass--on" disabled={!txAllowed} onClick={() => run(dx ? 'CQ DX chiesto' : 'CQ chiesto', () => postCq(dx ? 'DX' : ''))}>
          {dx ? 'CQ DX' : 'CQ'}
        </button>
        <label className="tx-check">
          <input type="checkbox" checked={dx} onChange={(e) => setDx(e.target.checked)} /> DX
        </label>
      </div>
      <label className="auto-field">
        Messaggio libero
        <input
          type="text"
          value={text}
          maxLength={MAX_TEXT}
          onChange={(e) => setText(e.target.value.toUpperCase().slice(0, MAX_TEXT))}
          placeholder="TNX 73"
        />
      </label>
      <div className="tx-row">
        <button type="button" className="glass" disabled={text.trim() === ''} onClick={() => run('Messaggio preparato', () => postFreeText(text.trim(), false))}>
          Prepara
        </button>
        <button type="button" className="glass glass--on" disabled={!txAllowed || text.trim() === ''} onClick={() => run('Messaggio inviato', () => postFreeText(text.trim(), true))}>
          Invia
        </button>
      </div>
      <label className="auto-field">
        Frequenza di trasmissione (Hz)
        <input type="number" min={200} max={3000} value={hz} onChange={(e) => setHz(e.target.value)} />
      </label>
      <div className="tx-row">
        <button type="button" className="glass" onClick={() => run('Frequenza impostata', () => postTxFrequency(Number(hz)))}>
          Imposta
        </button>
      </div>
      <p className="live-note" role="status">
        {note}
      </p>
    </div>
  );
}
