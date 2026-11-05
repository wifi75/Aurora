/**
 * aurora-rapporti: raccoglie i rapporti degli errori che Aurora manda solo se l'operatore li ha accesi.
 *
 * - POST /v1/report  rapporto breve JSON (livello 2), al massimo 32 KB
 * - POST /v1/bundle  pacchetto zip senza dati personali (livello 1 e scheda Assistenza), al massimo 10 MB
 * - POST /v1/attivo  «sono attiva oggi»: codice casuale dell'installazione, versione e sistema (una riga al giorno)
 * - GET  /v1/attivi  installazioni attive per giorno, settimana e versione e il totale di sempre (protetto da ADMIN_TOKEN)
 * - GET  /v1/summary errori piu' frequenti per versione (protetto da ADMIN_TOKEN, un segreto di Wrangler)
 * - GET  /v1/bundle/<codice>  scarica un pacchetto dal suo codice breve (protetto da ADMIN_TOKEN)
 *
 * L'indirizzo IP del mittente si usa solo come chiave del limite di richieste: non si registra e non si salva da nessuna parte.
 * Ogni notte un Cron Trigger cancella rapporti, pacchetti e righe dell'indice piu' vecchi di GIORNI_CONSERVAZIONE (90).
 */

interface Env {
  RAPPORTI: R2Bucket;
  INDICE: D1Database;
  LIMITE: RateLimit;
  CHIAVE_INGRESSO: string;
  GIORNI_CONSERVAZIONE: string;
  ADMIN_TOKEN?: string;
}

const MAX_REPORT = 32 * 1024;
const MAX_BUNDLE = 10 * 1024 * 1024;
/** I campi di un rapporto che si tengono; tutto il resto si scarta. */
const CAMPI = ['schema', 'id', 'installazione', 'giorno', 'versione', 'sistema', 'motore', 'radio', 'ptt', 'cat', 'audio_in', 'audio_out', 'area', 'livello', 'origine', 'messaggio', 'conteggio'] as const;

function risposta(status: number, corpo: unknown): Response {
  return new Response(JSON.stringify(corpo), { status, headers: { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store' } });
}

function oggi(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Testo corto e pulito da caratteri di controllo. */
function corto(v: unknown, max: number): string {
  return typeof v === 'string' ? v.replace(/[\u0000-\u001f\u007f]/g, ' ').slice(0, max) : '';
}

/** Confronto a tempo costante del segreto dell'amministratore. */
function stessoSegreto(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

function amministratore(req: Request, env: Env): boolean {
  const atteso = env.ADMIN_TOKEN ?? '';
  const dato = (req.headers.get('Authorization') ?? '').replace(/^Bearer\s+/i, '');
  return atteso.length >= 16 && stessoSegreto(dato, atteso);
}

/** Legge il corpo senza superare `max` byte (anche se Content-Length manca o mente). */
async function leggiCorpo(req: Request, max: number): Promise<ArrayBuffer | null> {
  const dichiarato = Number(req.headers.get('Content-Length') ?? '0');
  if (dichiarato > max || !req.body) return null;
  const pezzi: Uint8Array[] = [];
  let totale = 0;
  const lettore = req.body.getReader();
  for (;;) {
    const { done, value } = await lettore.read();
    if (done) break;
    totale += value.byteLength;
    if (totale > max) {
      await lettore.cancel();
      return null;
    }
    pezzi.push(value);
  }
  const out = new Uint8Array(totale);
  let at = 0;
  for (const p of pezzi) {
    out.set(p, at);
    at += p.byteLength;
  }
  return out.buffer;
}

async function ingresso(req: Request, env: Env): Promise<Response | null> {
  if (req.headers.get('X-Aurora-Key') !== env.CHIAVE_INGRESSO) return risposta(403, { error: 'chiave' });
  // l'IP solo come chiave del contatore, mai salvato
  const ip = req.headers.get('CF-Connecting-IP') ?? 'sconosciuto';
  const { success } = await env.LIMITE.limit({ key: ip });
  return success ? null : risposta(429, { error: 'troppe richieste' });
}

async function report(req: Request, env: Env): Promise<Response> {
  const corpo = await leggiCorpo(req, MAX_REPORT);
  if (!corpo) return risposta(413, { error: 'troppo grande' });
  let dati: Record<string, unknown>;
  try {
    dati = JSON.parse(new TextDecoder().decode(corpo)) as Record<string, unknown>;
  } catch {
    return risposta(400, { error: 'json' });
  }
  const pulito: Record<string, unknown> = {};
  for (const c of CAMPI) if (c in dati) pulito[c] = typeof dati[c] === 'number' ? dati[c] : corto(dati[c], c === 'messaggio' ? 1000 : 200);
  const giorno = oggi();
  const id = crypto.randomUUID();
  await env.RAPPORTI.put(`report/${giorno}/${id}.json`, JSON.stringify(pulito), { httpMetadata: { contentType: 'application/json' } });
  const versione = corto(dati.versione, 40) || '?';
  const area = corto(dati.area, 40) || 'altro';
  // il messaggio raggruppato: le cifre non contano
  const messaggio = corto(dati.messaggio, 300).replace(/\d+/g, '#');
  const conteggio = Math.max(1, Math.min(1_000_000, Math.floor(Number(dati.conteggio) || 1)));
  const installazione = corto(dati.installazione, 36);
  await env.INDICE.prepare(
    `INSERT INTO errori (giorno, versione, area, messaggio, conteggio, rapporti, installazioni) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
     ON CONFLICT (giorno, versione, area, messaggio) DO UPDATE SET conteggio = conteggio + ?5, rapporti = rapporti + 1,
       installazioni = CASE WHEN instr(installazioni, ?6) > 0 OR length(installazioni) > 3700 THEN installazioni ELSE installazioni || ',' || ?6 END`,
  )
    .bind(giorno, versione, area, messaggio, conteggio, installazione)
    .run();
  return risposta(200, { ok: true });
}

async function bundle(req: Request, env: Env): Promise<Response> {
  const corpo = await leggiCorpo(req, MAX_BUNDLE);
  if (!corpo) return risposta(413, { error: 'troppo grande' });
  // uno zip comincia con «PK»
  const testa = new Uint8Array(corpo, 0, Math.min(2, corpo.byteLength));
  if (testa[0] !== 0x50 || testa[1] !== 0x4b) return risposta(400, { error: 'zip' });
  const giorno = oggi();
  const codice = (req.headers.get('X-Aurora-Codice') ?? '').toUpperCase().replace(/[^0-9A-F]/g, '').slice(0, 8) || crypto.randomUUID().slice(0, 8).toUpperCase();
  const chiave = `bundle/${giorno}/${crypto.randomUUID()}.zip`;
  const versione = corto(req.headers.get('X-Aurora-Versione'), 40) || '?';
  const motivo = corto(req.headers.get('X-Aurora-Motivo'), 20) || 'crash';
  await env.RAPPORTI.put(chiave, corpo, {
    httpMetadata: { contentType: 'application/zip' },
    customMetadata: { codice, versione, motivo, installazione: corto(req.headers.get('X-Aurora-Installazione'), 36) },
  });
  await env.INDICE.prepare('INSERT OR REPLACE INTO pacchetti (codice, chiave, giorno, versione, motivo, byte) VALUES (?1, ?2, ?3, ?4, ?5, ?6)')
    .bind(codice, chiave, giorno, versione, motivo, corpo.byteLength)
    .run();
  return risposta(200, { ok: true, code: codice });
}

async function summary(url: URL, env: Env): Promise<Response> {
  const giorni = Math.max(1, Math.min(90, Number(url.searchParams.get('giorni') ?? '7') || 7));
  const dal = new Date(Date.now() - giorni * 86_400_000).toISOString().slice(0, 10);
  const versione = url.searchParams.get('versione');
  const sql = `SELECT versione, area, messaggio, SUM(conteggio) AS conteggio, SUM(rapporti) AS rapporti, MIN(giorno) AS primo, MAX(giorno) AS ultimo
    FROM errori WHERE giorno >= ?1 ${versione ? 'AND versione = ?2' : ''}
    GROUP BY versione, area, messaggio ORDER BY versione DESC, conteggio DESC LIMIT 200`;
  const q = env.INDICE.prepare(sql);
  const { results } = await (versione ? q.bind(dal, versione) : q.bind(dal)).all();
  const pacchetti = await env.INDICE.prepare('SELECT codice, giorno, versione, motivo, byte FROM pacchetti WHERE giorno >= ?1 ORDER BY giorno DESC LIMIT 100').bind(dal).all();
  return risposta(200, { dal, errori: results, pacchetti: pacchetti.results });
}

async function scaricaPacchetto(codice: string, env: Env): Promise<Response> {
  const riga = await env.INDICE.prepare('SELECT chiave FROM pacchetti WHERE codice = ?1').bind(codice.toUpperCase()).first<{ chiave: string }>();
  if (!riga) return risposta(404, { error: 'codice sconosciuto' });
  const oggetto = await env.RAPPORTI.get(riga.chiave);
  if (!oggetto) return risposta(404, { error: 'pacchetto cancellato' });
  return new Response(oggetto.body, { headers: { 'Content-Type': 'application/zip', 'Content-Disposition': `attachment; filename="aurora-${codice}.zip"` } });
}

/** Cancella cio' che e' piu' vecchio dei giorni di conservazione. */
async function pulizia(env: Env): Promise<void> {
  const giorni = Number(env.GIORNI_CONSERVAZIONE) || 90;
  const limite = new Date(Date.now() - giorni * 86_400_000).toISOString().slice(0, 10);
  for (const prefisso of ['report/', 'bundle/']) {
    let cursor: string | undefined;
    do {
      const elenco = await env.RAPPORTI.list({ prefix: prefisso, cursor, limit: 1000 });
      const vecchi = elenco.objects.map((o) => o.key).filter((k) => (k.split('/')[1] ?? '') < limite);
      if (vecchi.length) await env.RAPPORTI.delete(vecchi);
      cursor = elenco.truncated ? elenco.cursor : undefined;
    } while (cursor);
  }
  await env.INDICE.batch([env.INDICE.prepare('DELETE FROM errori WHERE giorno < ?1').bind(limite), env.INDICE.prepare('DELETE FROM pacchetti WHERE giorno < ?1').bind(limite), env.INDICE.prepare('DELETE FROM attivi WHERE giorno < ?1').bind(limite)]);
  // le installazioni ferme da piu' di un anno escono dall'elenco ma restano nel totale (contatore `archiviate`)
  const anno = new Date(Date.now() - 365 * 86_400_000).toISOString().slice(0, 10);
  await env.INDICE.batch([
    env.INDICE.prepare("INSERT INTO contatori (nome, valore) SELECT 'archiviate', COUNT(*) FROM installazioni WHERE ultima < ?1 ON CONFLICT (nome) DO UPDATE SET valore = valore + excluded.valore").bind(anno),
    env.INDICE.prepare('DELETE FROM installazioni WHERE ultima < ?1').bind(anno),
  ]);
}

/** Una installazione dice che oggi e' aperta: una sola riga al giorno, la versione e il sistema si aggiornano. */
async function attivo(req: Request, env: Env): Promise<Response> {
  const corpo = await leggiCorpo(req, 2048);
  if (!corpo) return risposta(413, { error: 'troppo grande' });
  let dati: Record<string, unknown>;
  try {
    dati = JSON.parse(new TextDecoder().decode(corpo)) as Record<string, unknown>;
  } catch {
    return risposta(400, { error: 'json' });
  }
  const id = corto(dati.installazione, 36);
  if (!/^[0-9a-f-]{36}$/.test(id)) return risposta(400, { error: 'installazione' });
  await env.INDICE.prepare('INSERT INTO attivi (giorno, installazione, versione, sistema) VALUES (?1, ?2, ?3, ?4) ON CONFLICT (giorno, installazione) DO UPDATE SET versione = excluded.versione, sistema = excluded.sistema')
    .bind(oggi(), id, corto(dati.versione, 20), corto(dati.sistema, 60))
    .run();
  await env.INDICE.prepare('INSERT INTO installazioni (installazione, prima, ultima) VALUES (?1, ?2, ?2) ON CONFLICT (installazione) DO UPDATE SET ultima = excluded.ultima')
    .bind(id, oggi())
    .run();
  return risposta(200, { ok: true });
}

/** Quante installazioni sono attive: oggi, ultimi 7 e 30 giorni, per versione e per giorno. */
async function attivi(env: Env): Promise<Response> {
  const giorni = (n: number) => new Date(Date.now() - n * 86_400_000).toISOString().slice(0, 10);
  const conta = async (dal: string) => (await env.INDICE.prepare('SELECT COUNT(DISTINCT installazione) AS n FROM attivi WHERE giorno >= ?1').bind(dal).first<{ n: number }>())?.n ?? 0;
  const perGiorno = await env.INDICE.prepare('SELECT giorno, COUNT(*) AS n FROM attivi WHERE giorno >= ?1 GROUP BY giorno ORDER BY giorno DESC').bind(giorni(30)).all();
  const perVersione = await env.INDICE.prepare('SELECT versione, COUNT(DISTINCT installazione) AS n FROM attivi WHERE giorno >= ?1 GROUP BY versione ORDER BY n DESC').bind(giorni(7)).all();
  // il totale di sempre: le installazioni nell'elenco piu' quelle archiviate dopo un anno di silenzio
  const elenco = (await env.INDICE.prepare('SELECT COUNT(*) AS n FROM installazioni').first<{ n: number }>())?.n ?? 0;
  const archiviate = (await env.INDICE.prepare("SELECT valore AS n FROM contatori WHERE nome = 'archiviate'").first<{ n: number }>())?.n ?? 0;
  const nuove = await env.INDICE.prepare('SELECT prima AS giorno, COUNT(*) AS n FROM installazioni WHERE prima >= ?1 GROUP BY prima ORDER BY prima DESC').bind(giorni(30)).all();
  return risposta(200, { totale: elenco + archiviate, oggi: await conta(oggi()), settimana: await conta(giorni(6)), mese: await conta(giorni(29)), nuove_per_giorno: nuove.results, per_giorno: perGiorno.results, per_versione: perVersione.results });
}

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const url = new URL(req.url);
    try {
      if (req.method === 'POST' && url.pathname === '/v1/attivo') {
        const no = await ingresso(req, env);
        if (no) return no;
        return await attivo(req, env);
      }
      if (req.method === 'GET' && url.pathname === '/v1/attivi') {
        return amministratore(req, env) ? await attivi(env) : risposta(401, { error: 'segreto' });
      }
      if (req.method === 'POST' && (url.pathname === '/v1/report' || url.pathname === '/v1/bundle')) {
        const no = await ingresso(req, env);
        if (no) return no;
        return url.pathname === '/v1/report' ? await report(req, env) : await bundle(req, env);
      }
      if (req.method === 'GET' && url.pathname === '/v1/summary') {
        return amministratore(req, env) ? await summary(url, env) : risposta(401, { error: 'segreto' });
      }
      const pacchetto = /^\/v1\/bundle\/([0-9A-Fa-f]{8})$/.exec(url.pathname);
      if (req.method === 'GET' && pacchetto) {
        return amministratore(req, env) ? await scaricaPacchetto(pacchetto[1], env) : risposta(401, { error: 'segreto' });
      }
      return risposta(404, { error: 'non trovato' });
    } catch {
      // niente dettagli (e niente IP) nei log
      return risposta(500, { error: 'interno' });
    }
  },

  async scheduled(_event: ScheduledController, env: Env, ctx: ExecutionContext): Promise<void> {
    ctx.waitUntil(pulizia(env));
  },
} satisfies ExportedHandler<Env>;
