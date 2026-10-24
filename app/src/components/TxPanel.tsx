import { useState } from 'react';
import { postCq, postFreeText, postTxFrequency } from '../api/client';
import type { DecodedMessage, StationStatus } from '../api/schemas';
import { useT } from '../i18n';
import { OutputLevel } from './OutputLevel';
import { expandMacro, useMacros } from '../lib/macros';
import { replyErrorMessage } from '../lib/replyError';
import { activeTx, nextTx, txMessages } from '../lib/txMessages';

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
  const t = useT();
  const messages = status
    ? txMessages({ myCall: status.de_call, myGrid: status.de_grid, dxCall: status.dx_call ?? '', report: status.report ?? '' })
    : [];
  const active = activeTx(messages, status?.tx_message);
  const next = status && active < 0 ? nextTx(decodes, status.de_call, status.dx_call ?? '') : -1;
  const macros = useMacros((s) => s.macros);
  const [dx, setDx] = useState(false);
  const [text, setText] = useState('');
  const [hz, setHz] = useState(String(initialFrequency));
  const [note, setNote] = useState('');

  const run = async (what: string, action: () => Promise<void>) => {
    try {
      await action();
      setNote(what);
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="tx-panel">
      <div className="tx-main">
      {status && (
        <>
          <div className="tx-dx">
            <span className="muted">{t('tx.dxLabel')}</span> <strong className="data">{status.dx_call ?? '—'}</strong>
            {status.dx_grid ? <span className="data muted"> {status.dx_grid}</span> : null}
            <span className="data muted tx-freqs">
              <span>{`Tx ${status.tx_df ?? '—'} Hz`}</span> <span>{`Rx ${status.rx_df ?? '—'} Hz`}</span>
            </span>
          </div>
          <ol className="tx-list" aria-label={t('tx.list')} title={t('tip.tx.list')}>
            {messages.map((m, i) => (
              <li
                key={i}
                aria-current={i === active ? 'true' : undefined}
                data-next={i === next && m !== '' ? 'true' : undefined}
                className={m === '' ? 'tx-empty' : undefined}
              >
                <span className="tx-n">Tx{i + 1}</span>
                <span className="data">{m}</span>
              </li>
            ))}
          </ol>
        </>
      )}
      {!txAllowed && <p className="muted tx-note">{t('tx.closed')}</p>}
      <div className="tx-row">
        <button
          type="button"
          className="glass glass--on"
          disabled={!txAllowed}
          title={t('tip.tx.cq')}
          onClick={() => run(dx ? t('tx.cqDxAsked') : t('tx.cqAsked'), () => postCq(dx ? 'DX' : ''))}
        >
          {dx ? t('tx.cqDx') : t('tx.cq')}
        </button>
        <label className="tx-check">
          <input type="checkbox" checked={dx} onChange={(e) => setDx(e.target.checked)} /> {t('tx.dxCheck')}
        </label>
      </div>
      <label className="auto-field" title={t('tip.tx.free')}>
        {t('tx.free')}
        <input
          type="text"
          value={text}
          maxLength={MAX_TEXT}
          onChange={(e) => setText(e.target.value.toUpperCase().slice(0, MAX_TEXT))}
          placeholder="TNX 73"
        />
      </label>
      <div className="macro-chips" role="group" aria-label={t('macro.chips')}>
        {macros.map((m) => (
          <button
            key={m}
            type="button"
            className="glass"
            title={t('tip.macro.chip')}
            onClick={() =>
              setText(expandMacro(m, { dx: status?.dx_call ?? '', me: status?.de_call ?? '', report: status?.report ?? '' }))
            }
          >
            {m}
          </button>
        ))}
      </div>
      <div className="tx-row">
        <button
          type="button"
          className="glass"
          disabled={text.trim() === ''}
          title={t('tip.tx.free')}
          onClick={() => run(t('tx.prepared'), () => postFreeText(text.trim(), false))}
        >
          {t('tx.prepare')}
        </button>
        <button
          type="button"
          className="glass glass--on"
          disabled={!txAllowed || text.trim() === ''}
          title={t('tip.tx.free')}
          onClick={() => run(t('tx.sent'), () => postFreeText(text.trim(), true))}
        >
          {t('tx.send')}
        </button>
      </div>
      <label className="auto-field" title={t('tip.tx.freq')}>
        {t('tx.freq')}
        <input type="number" min={200} max={3000} value={hz} onChange={(e) => setHz(e.target.value)} />
      </label>
      <div className="tx-row">
        <button type="button" className="glass" title={t('tip.tx.freq')} onClick={() => run(t('tx.freqSet'), () => postTxFrequency(Number(hz)))}>
          {t('tx.set')}
        </button>
      </div>
      <p className="live-note" role="status">
        {note}
      </p>
      </div>
      <OutputLevel txAllowed={txAllowed} />
    </div>
  );
}
