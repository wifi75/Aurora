/**
 * Messaggi del proprietario a tutte le installazioni (auguri, avvisi, «aggiorna subito»): li scrive solo da Aurora sul suo PC, con il
 * segreto dell'amministratore; Aurora (e l'agente nel tray) li legge ogni ora da GET /v1/messaggi e li mostra una volta sola.
 * Nessun dato di chi li legge: la richiesta porta solo la versione del programma, per i messaggi rivolti alle versioni vecchie.
 */

const LINGUE = ['it', 'en', 'de', 'fr', 'es', 'pt'] as const;
type Lingua = (typeof LINGUE)[number];

export interface Messaggio {
  id: string;
  titolo: Partial<Record<Lingua, string>>;
  testo: Partial<Record<Lingua, string>>;
  /** Da quando a quando si mostra (ISO, UTC). */
  dal: string;
  al: string;
  /** Solo alle versioni piu' vecchie di questa (vuoto: a tutti). */
  prima_di: string;
  link: string;
  /** Aspetto della cartolina: normale o festa (auguri). */
  tema: 'normale' | 'festa';
  /** Tema grafico della cartolina (dalla 0.11.179): natale, anno, pasqua, aggiorna, servizio; vuoto = normale. Le versioni vecchie lo ignorano. */
  stile: Stile;
  creato: string;
}

const STILI = ['natale', 'anno', 'pasqua', 'aggiorna', 'servizio'] as const;
type Stile = (typeof STILI)[number] | '';
function stile(v: unknown): Stile {
  return typeof v === 'string' && (STILI as readonly string[]).includes(v) ? (v as Stile) : '';
}

function versione(v: string): number[] {
  return v.split('.').map((x) => Number.parseInt(x, 10) || 0);
}

/** `a` e' piu' vecchia di `b`. */
function piuVecchia(a: string, b: string): boolean {
  const x = versione(a);
  const y = versione(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d < 0;
  }
  return false;
}

function testi(v: unknown, max: number): Partial<Record<Lingua, string>> {
  const out: Partial<Record<Lingua, string>> = {};
  if (typeof v !== 'object' || v === null) return out;
  for (const l of LINGUE) {
    const t = (v as Record<string, unknown>)[l];
    if (typeof t === 'string' && t.trim() !== '') out[l] = t.replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, ' ').trim().slice(0, max);
  }
  return out;
}

function data(v: unknown): string | null {
  if (typeof v !== 'string' || v === '') return null;
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

export function daRiga(r: Record<string, unknown>): Messaggio {
  return {
    id: String(r.id),
    titolo: JSON.parse(String(r.titolo || '{}')) as Messaggio['titolo'],
    testo: JSON.parse(String(r.testo || '{}')) as Messaggio['testo'],
    dal: String(r.dal),
    al: String(r.al),
    prima_di: String(r.prima_di ?? ''),
    link: String(r.link ?? ''),
    tema: r.tema === 'festa' ? 'festa' : 'normale',
    stile: stile(r.stile),
    creato: String(r.creato),
  };
}

/** I messaggi da mostrare adesso a una certa versione (pubblico). */
export async function attivi(db: D1Database, ver: string): Promise<Messaggio[]> {
  const ora = new Date().toISOString();
  const { results } = await db.prepare('SELECT * FROM messaggi WHERE dal <= ?1 AND al >= ?1 ORDER BY dal DESC LIMIT 20').bind(ora).all();
  return results.map(daRiga).filter((m) => m.prima_di === '' || ver === '' || piuVecchia(ver, m.prima_di));
}

/** Le versioni che sanno ricevere i messaggi (la 0.11.168 e' la prima). */
const PRIMA_CON_MESSAGGI = '0.11.168';

/**
 * Tutti i messaggi (amministratore), con due numeri per ognuno: `visti` = installazioni che hanno davvero mostrato la cartolina
 * (dalla 0.11.182, codice anonimo dell'installazione), `raggiungibili` = installazioni attive nei giorni del messaggio con una versione
 * che lo riceve (dalla tabella `attivi`, cosi' vale anche per le versioni che non dicono «visto»).
 */
export async function tutti(db: D1Database): Promise<(Messaggio & { visti: number; raggiungibili: number })[]> {
  const { results } = await db.prepare('SELECT * FROM messaggi ORDER BY dal DESC LIMIT 200').all();
  const lista = results.map(daRiga);
  if (lista.length === 0) return [];
  const visti = new Map<string, number>();
  const v = await db.prepare('SELECT messaggio, COUNT(*) AS n FROM messaggi_visti GROUP BY messaggio').all<{ messaggio: string; n: number }>();
  for (const r of v.results) visti.set(r.messaggio, r.n);
  const dal = lista.reduce((min, m) => (m.dal < min ? m.dal : min), lista[0].dal).slice(0, 10);
  const a = await db
    .prepare('SELECT installazione, versione, giorno FROM attivi WHERE giorno >= ?1')
    .bind(dal)
    .all<{ installazione: string; versione: string; giorno: string }>();
  return lista.map((m) => {
    const da = m.dal.slice(0, 10);
    const a2 = m.al.slice(0, 10);
    const chi = new Set<string>();
    for (const r of a.results) {
      if (r.giorno < da || r.giorno > a2) continue;
      if (piuVecchia(r.versione, PRIMA_CON_MESSAGGI)) continue;
      if (m.prima_di !== '' && !piuVecchia(r.versione, m.prima_di)) continue;
      chi.add(r.installazione);
    }
    return { ...m, visti: visti.get(m.id) ?? 0, raggiungibili: chi.size };
  });
}

/** Un'installazione ha mostrato la cartolina di un messaggio: una riga sola per messaggio e installazione (codice anonimo). */
export async function visto(db: D1Database, id: string, installazione: string): Promise<boolean> {
  if (!/^[0-9a-f]{8}$/.test(id) || !/^[0-9a-f-]{36}$/.test(installazione)) return false;
  const esiste = await db.prepare('SELECT 1 FROM messaggi WHERE id = ?1').bind(id).first();
  if (!esiste) return false;
  await db
    .prepare('INSERT OR IGNORE INTO messaggi_visti (messaggio, installazione, giorno) VALUES (?1, ?2, ?3)')
    .bind(id, installazione, new Date().toISOString().slice(0, 10))
    .run();
  return true;
}

/** Crea un messaggio dal corpo JSON (amministratore). Restituisce l'errore in chiaro se manca qualcosa. */
export async function crea(db: D1Database, corpo: unknown): Promise<Messaggio | string> {
  if (typeof corpo !== 'object' || corpo === null) return 'corpo non valido';
  const c = corpo as Record<string, unknown>;
  const titolo = testi(c.titolo, 120);
  const testo = testi(c.testo, 1200);
  if (!titolo.it && !titolo.en) return 'serve almeno il titolo in italiano o in inglese';
  if (!testo.it && !testo.en) return 'serve almeno il testo in italiano o in inglese';
  const dal = data(c.dal) ?? new Date().toISOString();
  const al = data(c.al);
  if (!al || al <= dal) return 'la data di fine deve venire dopo quella di inizio';
  const prima = typeof c.prima_di === 'string' && /^\d+(\.\d+){0,3}$/.test(c.prima_di.trim()) ? c.prima_di.trim() : '';
  const link = typeof c.link === 'string' && /^https:\/\/[^\s"'<>]{3,300}$/.test(c.link.trim()) ? c.link.trim() : '';
  const tema = c.tema === 'festa' ? 'festa' : 'normale';
  const st = stile(c.stile);
  const id = crypto.randomUUID().slice(0, 8);
  const creato = new Date().toISOString();
  await db
    .prepare('INSERT INTO messaggi (id, titolo, testo, dal, al, prima_di, link, tema, stile, creato) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)')
    .bind(id, JSON.stringify(titolo), JSON.stringify(testo), dal, al, prima, link, tema, st, creato)
    .run();
  return { id, titolo, testo, dal, al, prima_di: prima, link, tema, stile: st, creato };
}

export async function togli(db: D1Database, id: string): Promise<void> {
  await db.prepare('DELETE FROM messaggi WHERE id = ?1').bind(id).run();
  await db.prepare('DELETE FROM messaggi_visti WHERE messaggio = ?1').bind(id).run();
}

/** Istruzioni per il modello: traduzione fedele, gergo radioamatoriale naturale, solo JSON. */
const ISTRUZIONI =
  'You translate short notices written by an Italian amateur radio operator for the users of Aurora, an FT8/FT4 program. ' +
  'Translate faithfully, keeping the same meaning and tone; do not add, remove or change greetings. ' +
  'Keep ham radio jargon natural for operators (DX, QSO, 73, band, "see you on the air"). Keep names like Aurora, IU3WJQ, FT8 unchanged. ' +
  'Reply ONLY with JSON: {"en":{"titolo":"","testo":""},"de":{"titolo":"","testo":""},"fr":{"titolo":"","testo":""},"es":{"titolo":"","testo":""},"pt":{"titolo":"","testo":""}}.';

/** Traduce titolo e testo dall'italiano nelle altre cinque lingue con Workers AI (Llama 3.3, che conosce il gergo radio). Ogni lingua che non arriva resta vuota. */
export async function traduci(
  ai: { run(model: string, input: Record<string, unknown>): Promise<unknown> },
  titolo: string,
  testo: string,
): Promise<{ titolo: Partial<Record<Lingua, string>>; testo: Partial<Record<Lingua, string>> }> {
  const out = { titolo: {} as Partial<Record<Lingua, string>>, testo: {} as Partial<Record<Lingua, string>> };
  if (titolo.trim() === '' && testo.trim() === '') return out;
  let risposta: unknown;
  try {
    const r = (await ai.run('@cf/meta/llama-3.3-70b-instruct-fp8-fast', {
      messages: [
        { role: 'system', content: ISTRUZIONI },
        { role: 'user', content: JSON.stringify({ titolo, testo }) },
      ],
      max_tokens: 1800,
      temperature: 0.2,
    })) as { response?: unknown };
    risposta = r.response;
  } catch {
    return out;
  }
  // il modello a volte risponde con l'oggetto gia' letto, a volte con il testo del JSON (anche tra ``` ```)
  let dati: unknown = risposta;
  if (typeof risposta === 'string') {
    const m = /\{[\s\S]*\}/.exec(risposta);
    try {
      dati = m ? JSON.parse(m[0]) : null;
    } catch {
      dati = null;
    }
  }
  if (typeof dati !== 'object' || dati === null) return out;
  for (const l of LINGUE) {
    if (l === 'it') continue;
    const v = (dati as Record<string, { titolo?: unknown; testo?: unknown } | undefined>)[l];
    if (typeof v?.titolo === 'string' && v.titolo.trim()) out.titolo[l] = v.titolo.trim().slice(0, 120);
    if (typeof v?.testo === 'string' && v.testo.trim()) out.testo[l] = v.testo.trim().slice(0, 1200);
  }
  return out;
}
