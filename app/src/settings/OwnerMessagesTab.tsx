import { useCallback, useEffect, useState } from 'react';
import { askConfirm } from '../components/ConfirmHost';
import { showAgentPopup } from '../components/AgentPopup';
import { Logo } from '../components/Logo';
import { useT } from '../i18n';
import { useLang } from '../i18n/lang';
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
  stile?: string;
  /** Installazioni che hanno mostrato la cartolina (dalla 0.11.182) e installazioni attive che potevano riceverlo. */
  visti?: number;
  raggiungibili?: number;
}

/** Modelli pronti, gia' scritti in tutte le lingue: riempiono il modulo (si possono poi cambiare). Date: da oggi per qualche giorno. */
type L6 = Record<Lang, string>;
/** Temi grafici della cartolina (lo stesso nome del modello). */
const THEMES = ['natale', 'anno', 'pasqua', 'aggiorna', 'servizio'] as const;
type Theme = (typeof THEMES)[number] | '';

const TEMPLATES: readonly { id: Theme; days: number; festive: boolean; title: L6; text: L6; before?: 'current' }[] = [
  {
    id: 'natale', days: 3, festive: true,
    title: { it: 'Buon Natale!', en: 'Merry Christmas!', de: 'Frohe Weihnachten!', fr: 'Joyeux Noël !', es: '¡Feliz Navidad!', pt: 'Feliz Natal!' },
    text: {
      it: 'Auguri di buon Natale a te e alla tua famiglia, e buoni DX!',
      en: 'Merry Christmas to you and your family, and good DX!',
      de: 'Frohe Weihnachten dir und deiner Familie, und gute DX!',
      fr: 'Joyeux Noël à toi et à ta famille, et bons DX !',
      es: '¡Feliz Navidad a ti y a tu familia, y buenos DX!',
      pt: 'Feliz Natal para ti e para a tua família, e bons DX!',
    },
  },
  {
    id: 'anno', days: 2, festive: true,
    title: { it: 'Buon anno!', en: 'Happy New Year!', de: 'Frohes neues Jahr!', fr: 'Bonne année !', es: '¡Feliz año nuevo!', pt: 'Feliz ano novo!' },
    text: {
      it: 'Felice anno nuovo: che sia pieno di paesi nuovi e di bande aperte!',
      en: 'Happy New Year: may it be full of new countries and open bands!',
      de: 'Frohes neues Jahr: möge es voller neuer Länder und offener Bänder sein!',
      fr: 'Bonne année : qu\'elle soit pleine de nouveaux pays et de bandes ouvertes !',
      es: '¡Feliz año nuevo: que esté lleno de países nuevos y bandas abiertas!',
      pt: 'Feliz ano novo: que seja cheio de países novos e bandas abertas!',
    },
  },
  {
    id: 'pasqua', days: 2, festive: true,
    title: { it: 'Buona Pasqua!', en: 'Happy Easter!', de: 'Frohe Ostern!', fr: 'Joyeuses Pâques !', es: '¡Feliz Pascua!', pt: 'Feliz Páscoa!' },
    text: {
      it: 'Auguri di buona Pasqua e buoni collegamenti!',
      en: 'Happy Easter and good contacts!',
      de: 'Frohe Ostern und gute Verbindungen!',
      fr: 'Joyeuses Pâques et bons contacts !',
      es: '¡Feliz Pascua y buenos contactos!',
      pt: 'Feliz Páscoa e bons contactos!',
    },
  },
  {
    id: 'aggiorna', days: 7, festive: false, before: 'current',
    title: { it: 'Aggiorna Aurora', en: 'Update Aurora', de: 'Aurora aktualisieren', fr: 'Mets Aurora à jour', es: 'Actualiza Aurora', pt: 'Atualiza a Aurora' },
    text: {
      it: 'La tua versione ha un problema corretto nella nuova: aggiorna da Impostazioni → Aggiornamenti.',
      en: 'Your version has a problem fixed in the new one: update from Settings → Updates.',
      de: 'Deine Version hat ein Problem, das in der neuen behoben ist: aktualisiere über Einstellungen → Updates.',
      fr: 'Ta version a un problème corrigé dans la nouvelle : mets à jour depuis Réglages → Mises à jour.',
      es: 'Tu versión tiene un problema corregido en la nueva: actualiza desde Ajustes → Actualizaciones.',
      pt: 'A tua versão tem um problema corrigido na nova: atualiza em Definições → Atualizações.',
    },
  },
  {
    id: 'servizio', days: 1, festive: false,
    title: { it: 'Avviso di servizio', en: 'Service notice', de: 'Servicehinweis', fr: 'Avis de service', es: 'Aviso de servicio', pt: 'Aviso de serviço' },
    text: {
      it: 'Oggi alcuni servizi online di Aurora potrebbero non rispondere per qualche ora.',
      en: 'Today some of Aurora\'s online services may not respond for a few hours.',
      de: 'Heute könnten einige Online-Dienste von Aurora für ein paar Stunden nicht antworten.',
      fr: 'Aujourd\'hui certains services en ligne d\'Aurora pourraient ne pas répondre pendant quelques heures.',
      es: 'Hoy algunos servicios en línea de Aurora podrían no responder durante unas horas.',
      pt: 'Hoje alguns serviços online da Aurora podem não responder durante algumas horas.',
    },
  },
];

declare const __AURORA_UI_VERSION__: string;

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
  // i nomi dei modelli nella lingua dell'interfaccia (i testi del messaggio restano in tutte e sei)
  const uiLang = useLang((st) => st.lang) as Lang;
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
  const [theme, setTheme] = useState<Theme>('');
  const [busy, setBusy] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [langsOpen, setLangsOpen] = useState(false);
  // lingua della cartolina d'anteprima (chi non ha la sua lingua vede l'inglese o l'italiano, come nell'agente)
  const [shown, setShown] = useState<Lang>('it');

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

  const applyTemplate = (id: string) => {
    const m = TEMPLATES.find((x) => x.id === id);
    if (!m) return;
    setTitle({ ...m.title });
    setText({ ...m.text });
    setFestive(m.festive);
    setTheme(m.id);
    setFrom(localInput(new Date()));
    setTo(localInput(new Date(Date.now() + m.days * 86_400_000)));
    setBefore(m.before === 'current' && typeof __AURORA_UI_VERSION__ === 'string' ? __AURORA_UI_VERSION__ : '');
  };

  // come la vedranno gli utenti: la stessa cartolina con il suono, solo su questo PC (non si pubblica niente)
  const preview = () =>
    showAgentPopup(
      { kind: festive ? '✦ IU3WJQ' : 'IU3WJQ', title: title.it ?? '', text: text.it ?? '', primary: link ? t('agent.msg.open') : t('upd.poster.ok'), secondary: link ? t('upd.poster.ok') : '', festive, theme },
      12,
    );

  // traduzione automatica (servizio dei messaggi, solo con il segreto): dall'italiano alle altre cinque lingue, da ricontrollare
  const translate = async () => {
    const others = LANGS.filter((l) => l !== 'it');
    const already = others.some((l) => (title[l] ?? '').trim() || (text[l] ?? '').trim());
    if (already && !(await askConfirm(t('own.translateAsk')))) return;
    setTranslating(true);
    setNote('');
    try {
      const r = await fetch(`${BASE}/traduci`, { method: 'POST', headers: headers(), body: JSON.stringify({ titolo: title.it ?? '', testo: text.it ?? '' }) });
      if (r.status === 401) {
        setNote(t('own.badToken'));
        return;
      }
      const j = (await r.json()) as { titolo?: Partial<Record<Lang, string>>; testo?: Partial<Record<Lang, string>> };
      const got = others.filter((l) => j.titolo?.[l] || j.testo?.[l]);
      if (got.length === 0) {
        setNote(t('own.translateFail'));
        return;
      }
      setTitle((cur) => ({ ...cur, ...Object.fromEntries(got.map((l) => [l, j.titolo?.[l] ?? cur[l] ?? ''])) }));
      setText((cur) => ({ ...cur, ...Object.fromEntries(got.map((l) => [l, j.testo?.[l] ?? cur[l] ?? ''])) }));
      setLangsOpen(true);
      setNote(t('own.translated', { n: got.length }));
    } catch {
      setNote(t('own.offline'));
    } finally {
      setTranslating(false);
    }
  };

  const duplicate = (m: Msg) => {
    setTitle(m.titolo);
    setText(m.testo);
    setFestive(m.tema === 'festa');
    setTheme((m.stile ?? '') as Theme);
    setBefore(m.prima_di);
    setLink(m.link);
    setFrom(localInput(new Date()));
    setTo(localInput(new Date(Date.now() + 3 * 86_400_000)));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const publish = async () => {
    // arriva a tutte le installazioni: si chiede conferma, con a chi va e fino a quando
    const who = before ? t('own.whoOld', { v: before }) : t('own.whoAll');
    if (!(await askConfirm(t('own.publishAsk', { title: title.it ?? '', who, until: new Date(to).toLocaleString() })))) return;
    setBusy(true);
    try {
      const r = await fetch(BASE, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify({ titolo: title, testo: text, dal: new Date(from).toISOString(), al: new Date(to).toISOString(), prima_di: before, link, tema: festive ? 'festa' : 'normale', stile: theme }),
      });
      const j = (await r.json().catch(() => ({}))) as { error?: string };
      if (r.ok) {
        setNote(t('own.published'));
        setTitle({});
        setText({});
        setTheme('');
        setFestive(false);
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
  const pick = (m: Partial<Record<Lang, string>>) => (m[shown] ?? '').trim() || (m.en ?? '').trim() || (m.it ?? '').trim();
  const cardTitle = pick(title);
  const cardText = pick(text);
  const who = before ? t('own.whoOld', { v: before }) : t('own.whoAll');
  const when = (v: string) => (v ? new Date(v).toLocaleString([], { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' }) : '');
  return (
    <Page className="own-page">
      <Group title={t('own.token')} className="own-g-token">
        <div className="own-row">
          <input className="agent-input own-token own-input" type="password" autoComplete="off" value={token} placeholder="ADMIN_TOKEN" onChange={(e) => saveToken(e.target.value)} />
          <span className="muted own-hint">{t('own.tokenHint')}</span>
        </div>
      </Group>

      <Group title={t('own.new')} className="own-g-new">
        <div className="own-form">
          <div className="own-templates">
            <span className="muted">{t('own.templates')}</span>
            {TEMPLATES.map((m) => (
              <button key={m.id} type="button" className="glass chip" onClick={() => applyTemplate(m.id)}>
                {m.title[uiLang] ?? m.title.it}
              </button>
            ))}
          </div>
          <label>
            <span>
              {t('own.title')} <small className="muted">{(title.it ?? '').length}/120</small>
            </span>
            <input className="agent-input own-wide" maxLength={120} value={title.it ?? ''} onChange={(e) => setTitle({ ...title, it: e.target.value })} />
          </label>
          <label>
            <span>
              {t('own.text')} <small className="muted">{(text.it ?? '').length}/1200</small>
            </span>
            <textarea className="own-text" maxLength={1200} rows={3} value={text.it ?? ''} onChange={(e) => setText({ ...text, it: e.target.value })} />
          </label>
          <div className="own-translate">
            <button type="button" className="glass" disabled={translating || token.length < 16 || !(title.it ?? '').trim()} onClick={() => void translate()} title={t('own.translateTip')}>
              {translating ? t('own.translating') : t('own.translate')}
            </button>
            <span className="muted">{t('own.translateHint')}</span>
          </div>
          <details className="own-langs" open={langsOpen} onToggle={(e) => setLangsOpen((e.currentTarget as HTMLDetailsElement).open)}>
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
            <label>
              <span>{t('own.theme')}</span>
              <select className="agent-input own-select" value={theme} onChange={(e) => setTheme(e.target.value as Theme)}>
                <option value="">{t('own.theme.none')}</option>
                {THEMES.map((th) => (
                  <option key={th} value={th}>
                    {t(`own.theme.${th}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <label>
            <span>{t('own.link')}</span>
            <input className="agent-input own-wide" placeholder="https://" value={link} onChange={(e) => setLink(e.target.value)} />
          </label>
          <div className="own-actions">
            <button type="button" className="glass own-publish" disabled={busy || token.length < 16 || !(title.it ?? '').trim() || !(text.it ?? '').trim()} onClick={() => void publish()}>
              {t('own.publish')}
            </button>
            {note ? <span className="muted">{note}</span> : null}
          </div>
        </div>
      </Group>

      <Group title={t('own.previewTitle')} className="own-g-preview">
        <div className="own-preview">
          <div className="own-preview-langs" role="group" aria-label={t('own.otherLangs')}>
            {LANGS.map((l) => (
              <button key={l} type="button" className={`glass chip${shown === l ? ' own-chip--on' : ''}`} aria-pressed={shown === l} onClick={() => setShown(l)}>
                {l.toUpperCase()}
                {l !== 'it' && !(title[l] ?? '').trim() ? <small> ↺</small> : null}
              </button>
            ))}
          </div>
          {cardTitle ? (
            <div className={`agent-card agent-card--static${festive ? ' agent-card--festa' : ''}${theme ? ` agent-card--t-${theme}` : ''}`}>
              <div className="agent-card-band">
                {theme ? <i className="agent-card-deco" aria-hidden="true" /> : null}
                <Logo size={46} animated className="agent-card-logo" />
                <div className="agent-card-brand">
                  <b>AURORA</b>
                  <span>{festive ? '✦ IU3WJQ' : 'IU3WJQ'}</span>
                </div>
                <span className="agent-card-x" aria-hidden="true">
                  ×
                </span>
              </div>
              <div className="agent-card-body">
                <h3>{cardTitle}</h3>
                <p>{cardText}</p>
                <div className="agent-card-actions">
                  {link ? <span className="agent-card-btn">{t('upd.poster.ok')}</span> : null}
                  <span className="agent-card-btn agent-card-btn--main">{link ? t('agent.msg.open') : t('upd.poster.ok')}</span>
                </div>
              </div>
            </div>
          ) : (
            <p className="own-preview-empty muted">{t('own.previewEmpty')}</p>
          )}
          <p className="own-preview-who muted">{t('own.previewWho', { who, from: when(from), to: when(to) })}</p>
          <button type="button" className="glass" disabled={!(title.it ?? '').trim()} onClick={preview}>
            {t('own.previewSound')}
          </button>
        </div>
      </Group>

      <Group title={t('own.list')} className="own-g-list">
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
                      {m.stile ? ` · ${t(`own.theme.${m.stile}` as 'own.theme.natale')}` : ''}
                    </small>
                    {m.raggiungibili !== undefined ? (
                      <small className="own-reach" title={t('own.reachTip')}>
                        {t('own.reach', { seen: m.visti ?? 0, reach: m.raggiungibili })}
                      </small>
                    ) : null}
                  </span>
                  <span className="agent-opt-ctl">
                    <button type="button" className="glass" onClick={() => duplicate(m)}>
                      {t('own.duplicate')}
                    </button>
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
