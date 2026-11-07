import { useCallback, useEffect, useState } from 'react';
import { askConfirm } from '../components/ConfirmHost';
import { useT } from '../i18n';
import { Help } from './common';
import { Group, Notes, Page } from './layout';

/** Il servizio dei messaggi (lo stesso che li manda ad Aurora e all'agente). */
const BASE = 'https://aurora-rapporti.tizianocassone.workers.dev/v1/messaggi';
/** Il segreto resta solo su questo PC: chiave senza il prefisso «aurora-», quindi non va nel file delle impostazioni ne' al telefono. */
const TOKEN_KEY = 'auroraOwnerToken';

const LANGS = ['it', 'en', 'de', 'fr', 'es', 'pt'] as const;
type Lang = (typeof LANGS)[number];

interface Msg {
  id: string;
  titolo: Partial<Record<Lang, string>>;
  testo: Partial<Record<Lang, string>>;
  dal: string;
  al: string;
  prima_di: string;
  link: string;
  tema: 'normale' | 'festa';
}

function readToken(): string {
  try {
    return localStorage.getItem(TOKEN_KEY) ?? '';
  } catch {
    return '';
  }
}

/** `datetime-local` vuole l'ora del PC senza fuso. */
function localInput(d: Date): string {
  return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}

/**
 * «Messaggi agli utenti» (solo per l'autore): scrive i messaggi che arrivano a tutte le installazioni come cartolina con il suono.
 * Le operazioni chiedono il segreto dell'amministratore, che resta su questo PC.
 */
export function OwnerMessagesTab() {
  const t = useT();
  const [token, setToken] = useState(readToken);
  const [list, setList] = useState<Msg[] | null>(null);
  const [note, setNote] = useState('');
  const [title, setTitle] = useState<Partial<Record<Lang, string>>>({});
  const [text, setText] = useState<Partial<Record<Lang, string>>>({});
  const [from, setFrom] = useState(() => localInput(new Date()));
  const [to, setTo] = useState(() => localInput(new Date(Date.now() + 3 * 86_400_000)));
  const [before, setBefore] = useState('');
  const [link, setLink] = useState('');
  const [festive, setFestive] = useState(false);
  const [busy, setBusy] = useState(false);

  const headers = useCallback(() => ({ Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }), [token]);

  const load = useCallback(async () => {
    if (token.length < 16) return;
    try {
      const r = await fetch(`${BASE}/tutti`, { headers: headers() });
      if (r.status === 401) {
        setNote(t('own.badToken'));
        setList(null);
        return;
      }
      const j = (await r.json()) as { messaggi: Msg[] };
      setList(j.messaggi);
      setNote('');
    } catch {
      setNote(t('own.offline'));
    }
  }, [token, headers, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const saveToken = (v: string) => {
    setToken(v);
    try {
      localStorage.setItem(TOKEN_KEY, v);
    } catch {
      // senza archivio il segreto vale finche' la finestra resta aperta
    }
  };

  const publish = async () => {
    setBusy(true);
    try {
      const r = await fetch(BASE, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify({ titolo: title, testo: text, dal: new Date(from).toISOString(), al: new Date(to).toISOString(), prima_di: before, link, tema: festive ? 'festa' : 'normale' }),
      });
      const j = (await r.json().catch(() => ({}))) as { error?: string };
      if (r.ok) {
        setNote(t('own.published'));
        setTitle({});
        setText({});
        void load();
      } else {
        setNote(r.status === 401 ? t('own.badToken') : (j.error ?? `${r.status}`));
      }
    } catch {
      setNote(t('own.offline'));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (m: Msg) => {
    if (!(await askConfirm(t('own.removeAsk', { title: m.titolo.it ?? m.titolo.en ?? '' })))) return;
    await fetch(`${BASE}/${m.id}`, { method: 'DELETE', headers: headers() }).catch(() => undefined);
    void load();
  };

  const now = new Date().toISOString();
  return (
    <Page>
      <Group title={t('own.token')} span={2}>
        <div className="own-row">
          <input className="agent-input own-token" type="password" autoComplete="off" value={token} placeholder="ADMIN_TOKEN" onChange={(e) => saveToken(e.target.value)} />
          <span className="muted own-hint">{t('own.tokenHint')}</span>
        </div>
      </Group>

      <Group title={t('own.new')} span={2}>
        <div className="own-form">
          <label>
            <span>{t('own.title')}</span>
            <input className="agent-input own-wide" maxLength={120} value={title.it ?? ''} onChange={(e) => setTitle({ ...title, it: e.target.value })} />
          </label>
          <label>
            <span>{t('own.text')}</span>
            <textarea className="own-text" maxLength={1200} rows={3} value={text.it ?? ''} onChange={(e) => setText({ ...text, it: e.target.value })} />
          </label>
          <details className="own-langs">
            <summary className="muted">{t('own.otherLangs')}</summary>
            {LANGS.filter((l) => l !== 'it').map((l) => (
              <div key={l} className="own-lang">
                <b>{l.toUpperCase()}</b>
                <input className="agent-input own-wide" placeholder={t('own.title')} value={title[l] ?? ''} onChange={(e) => setTitle({ ...title, [l]: e.target.value })} />
                <textarea className="own-text" rows={2} placeholder={t('own.text')} value={text[l] ?? ''} onChange={(e) => setText({ ...text, [l]: e.target.value })} />
              </div>
            ))}
          </details>
          <div className="own-grid">
            <label>
              <span>{t('own.from')}</span>
              <input className="agent-input" type="datetime-local" value={from} onChange={(e) => setFrom(e.target.value)} />
            </label>
            <label>
              <span>{t('own.to')}</span>
              <input className="agent-input" type="datetime-local" value={to} onChange={(e) => setTo(e.target.value)} />
            </label>
            <label>
              <span>{t('own.before')}</span>
              <input className="agent-input" placeholder="0.11.169" value={before} onChange={(e) => setBefore(e.target.value)} />
            </label>
            <label className="own-check">
              <input type="checkbox" checked={festive} onChange={(e) => setFestive(e.target.checked)} />
              <span>{t('own.festive')}</span>
            </label>
          </div>
          <label>
            <span>{t('own.link')}</span>
            <input className="agent-input own-wide" placeholder="https://" value={link} onChange={(e) => setLink(e.target.value)} />
          </label>
          <div className="own-actions">
            <button type="button" className="glass" disabled={busy || token.length < 16 || !(title.it ?? '').trim() || !(text.it ?? '').trim()} onClick={() => void publish()}>
              {t('own.publish')}
            </button>
            {note ? <span className="muted">{note}</span> : null}
          </div>
        </div>
      </Group>

      <Group title={t('own.list')} span={2}>
        {list === null ? (
          <p className="muted">{t('own.needToken')}</p>
        ) : list.length === 0 ? (
          <p className="muted">{t('own.none')}</p>
        ) : (
          <div className="agent-opts">
            {list.map((m) => {
              const state = m.dal <= now && m.al >= now ? 'on' : m.al < now ? 'done' : 'wait';
              return (
                <div key={m.id} className="agent-opt">
                  <span>
                    <b>
                      {m.titolo.it ?? m.titolo.en} <span className={`own-state own-state--${state}`}>{t(`own.state.${state}`)}</span>
                    </b>
                    <small className="muted">
                      {new Date(m.dal).toLocaleString()} – {new Date(m.al).toLocaleString()}
                      {m.prima_di ? ` · < ${m.prima_di}` : ''}
                      {m.tema === 'festa' ? ' · ✦' : ''}
                    </small>
                  </span>
                  <span className="agent-opt-ctl">
                    <button type="button" className="glass" onClick={() => void remove(m)}>
                      {t('own.remove')}
                    </button>
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </Group>

      <Notes>
        <Help>{t('own.intro')}</Help>
      </Notes>
    </Page>
  );
}
