import { useState } from 'react';
import { postCq, postFreeText, postTxFrequency } from '../api/client';
import { replyErrorMessage } from '../lib/replyError';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Frequenza audio di partenza (Hz). */
  initialFrequency?: number;
}

const MAX_TEXT = 13;

/** Comandi di trasmissione: CQ, messaggio libero e frequenza audio. Cio' che trasmette si abilita solo dal PC. */
export function TxPanel({ txAllowed, initialFrequency = 1500 }: Props) {
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
