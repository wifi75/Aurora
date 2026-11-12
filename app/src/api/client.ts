import { AutomationCountriesSchema, AutomationStateSchema, type AutomationRules, type AutomationState } from './automation';
import { historyParams, type HistoryFilter } from '../lib/history';
import type { Wishlist } from '../lib/wishlist';
import { DigiSignalSchema, EngineSettingsSchema, HealthSchema, SpotResultSchema, UploadTestSchema, WavelogStationsSchema, type DigiSignal, type EngineSettings, type FoxLogEntry, type WavelogStation, type Health, type SpotResult, type WantedAlert } from './schemas';
import { parseEngineTx, useEngineTx } from '../state/engineTx';
import { parseControlValues, useControlValues } from '../state/controlValues';
import { dxccQueryString, type DxccMatrix, type DxccQuery } from '../lib/dxccCard';
import { inOrder } from '../lib/listenWhileOpen';

const BASE = '/api/v1';

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly retryAfterSecs?: number,
    /** Il testo d'errore gia' tradotto che manda il core, se c'e' (per esempio il CAT). */
    readonly detail?: string,
  ) {
    super(`${status} ${code}`);
    this.name = 'ApiError';
  }
}

async function request(path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(BASE + path, { credentials: 'include', ...init });
  if (!response.ok) {
    let code = 'error';
    let detail: string | undefined;
    try {
      const body: unknown = await response.json();
      if (body && typeof body === 'object' && 'error' in body && typeof body.error === 'string') code = body.error;
      if (body && typeof body === 'object' && 'message' in body && typeof body.message === 'string' && body.message !== '') detail = body.message;
    } catch {
      // corpo assente o non JSON: resta il codice generico
    }
    const retry = Number(response.headers.get('retry-after'));
    throw new ApiError(response.status, code, Number.isFinite(retry) && retry > 0 ? retry : undefined, detail);
  }
  return response;
}

function jsonInit(body: unknown): RequestInit {
  return { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) };
}

export async function getHealth(): Promise<Health> {
  return HealthSchema.parse(await (await request('/health')).json());
}

/** Il dispositivo e' associato? Solo un 401 vuol dire "no"; ogni altro errore e' un problema di rete. */
export async function checkSession(): Promise<boolean> {
  const response = await fetch(`${BASE}/devices`, { credentials: 'include' });
  if (response.status === 401) return false;
  if (!response.ok) throw new ApiError(response.status, 'error');
  return true;
}

export async function postHalt(): Promise<void> {
  await request('/commands/halt-tx', { method: 'POST' });
}

/** Le chiamate e il CQ dalla pagina Contest lo dicono al motore: solo quei QSO entrano nel log del contest. */
const fromContest = (init: RequestInit, contest: boolean): RequestInit => (contest ? { ...init, headers: { ...(init.headers as Record<string, string>), 'x-aurora-origine': 'contest' } } : init);

export async function postReply(decodeId: number, contest = false): Promise<void> {
  await request('/commands/reply', fromContest(jsonInit({ decode_id: decodeId }), contest));
}

/** Tasto destro su una stazione: in coda per il prossimo QSO (o subito, se non ce n'e' uno in corso); di nuovo la toglie. */
export async function postQueueReply(decodeId: number): Promise<void> {
  await request('/commands/queue-reply', jsonInit({ decode_id: decodeId }));
}

export async function finishPairing(code: string, deviceName: string): Promise<void> {
  await request('/pair/finish', jsonInit({ code, device_name: deviceName }));
}

export async function getAutomation(): Promise<AutomationState> {
  return AutomationStateSchema.parse(await (await request('/automation')).json());
}

/** I paesi di cty.dat per le esclusioni dell'automazione (vuoto senza cty.dat). */
export async function getAutomationCountries(): Promise<string[]> {
  return AutomationCountriesSchema.parse(await (await request('/automation/countries')).json()).countries;
}

export interface AutomationChange {
  enabled?: boolean;
  mode?: 'dry_run' | 'live';
  rules?: AutomationRules;
  accept_license_notice?: boolean;
}

export async function putAutomation(change: AutomationChange): Promise<AutomationState> {
  const init = { ...jsonInit(change), method: 'PUT' };
  return AutomationStateSchema.parse(await (await request('/automation', init)).json());
}

/**
 * I filtri di Attivita' di banda vanno anche ad Aurora: le Automazioni non rispondono mai alle stazioni che nascondono.
 * Senza server (o con il dispositivo non associato) non succede nulla: ci riprova al prossimo cambio.
 */
export async function putBandFilters(filters: unknown): Promise<void> {
  try {
    await request('/band-filters', { ...jsonInit(filters), method: 'PUT' });
  } catch {
    // il core li rilegge comunque dalle preferenze salvate al prossimo avvio
  }
}

export interface StationInfo {
  engine?: string;
  tx_allowed: boolean;
  rig_control?: boolean;
  /** Niente radio comandabile: le bande cambiano solo la frequenza di Aurora (la radio si sposta a mano). */
  manual_dial?: boolean;
  udp_listen: string;
  udp_forward: string[];
  export_udp: string[];
  export_file: string | null;
  audio_device: string | null;
  cty_loaded: number;
  worked_qsos: number;
  limits: { max_consecutive_cycles: number; max_per_day: number; dead_man_secs: number; run_unattended: boolean };
}

export async function getStationInfo(): Promise<StationInfo> {
  return (await (await request('/station-info')).json()) as StationInfo;
}

export async function postFreeText(text: string, send: boolean): Promise<void> {
  await request('/commands/free-text', jsonInit({ text, send }));
}

/** I campi «DX Call» e «Locatore» scritti a mano (vuoti = nessun DX). */
export async function postDx(call: string, grid: string): Promise<void> {
  await request('/commands/dx', jsonInit({ call, grid }));
}

/** Il rapporto da inviare scelto a mano, in dB (-50..+49). */
export async function postReport(db: number): Promise<void> {
  await request('/commands/report', jsonInit({ db }));
}

/** «Tune»: portante sulla frequenza Tx per un periodo (on) o ferma (off). */
export async function postTune(on: boolean): Promise<void> {
  await request('/commands/tune', jsonInit({ on }));
}

/** Un comando della pagina Fox: select (in coda), unqueue, drop (chiude un QSO), options, reset. */
export async function postFox(body: { action: 'select' | 'unqueue' | 'drop' | 'options' | 'reset' | 'otp'; call?: string; top?: boolean; slots?: number; more_cqs?: boolean; cq?: string; free_text?: string | null; auto_queue?: boolean; ultra?: boolean; seed?: string | null; interval?: number; keep_seed?: boolean }): Promise<void> {
  await request('/fox', jsonInit(body));
}

/** Il Fox log nella coda dell'invio diretto a Club Log (serve Club Log acceso nelle impostazioni); restituisce quanti QSO sono entrati. */
export async function postFoxClublog(qsos: FoxLogEntry[]): Promise<number> {
  const body: unknown = await (await request('/fox/clublog', jsonInit({ qsos }))).json();
  return body && typeof body === 'object' && 'queued' in body && typeof body.queued === 'number' ? body.queued : 0;
}

/** «Log QSO» a mano. */
export async function postLogQso(): Promise<void> {
  await request('/commands/log-qso', { method: 'POST' });
}

export async function postCq(modifier: string, contest = false): Promise<void> {
  await request('/commands/cq', fromContest(jsonInit({ modifier }), contest));
}

/** Sceglie a mano il messaggio Tx (1-6) da cui il motore parte, come il pallino accanto ai messaggi. */
export async function postTxMessage(n: number): Promise<void> {
  await request('/commands/tx-message', jsonInit({ n }));
}

/** Scrive a mano il testo del messaggio Tx `n` (1-6): resta finche' i messaggi non si rigenerano; vuoto torna lo standard. */
export async function postTxText(n: number, text: string): Promise<void> {
  await request('/commands/tx-text', jsonInit({ n, text }));
}

export async function postTxFrequency(hz: number): Promise<void> {
  await request('/commands/tx-frequency', jsonInit({ hz }));
}

/** Risponde con la frequenza impostata (`dial_hz`), cosi' la pagina la mostra subito. */
/** Clic destro sulla barra delle bande: la voce successiva dell'elenco del Band Hopping, subito. */
export async function postBandHopNext(): Promise<void> {
  await request('/commands/band-hop-next', jsonInit({}));
}

/** Le stazioni Wavelog/Cloudlog della chiave salvata, lette dal server (solo dalla rete di casa). */
export async function postWavelogStations(url: string): Promise<WavelogStation[]> {
  return WavelogStationsSchema.parse(await (await request('/qso-upload/wavelog-stations', jsonInit({ url }))).json()).stations;
}

export async function postBand(band: string): Promise<{ dial_hz?: number }> {
  return (await (await request('/commands/band', jsonInit({ band }))).json()) as { dial_hz?: number };
}

/** Porta la radio su una frequenza esatta (Hz), come per uno spot del cluster. */
export async function postFrequency(hz: number): Promise<{ dial_hz?: number }> {
  return (await (await request('/commands/frequency', jsonInit({ hz }))).json()) as { dial_hz?: number };
}

export async function postMode(mode: string): Promise<void> {
  await request('/commands/mode', jsonInit({ mode }));
}

export async function postRxFrequency(hz: number): Promise<void> {
  await request('/commands/rx-frequency', jsonInit({ hz }));
}

/** Regione IARU di una riga della tabella delle frequenze (come in JTDX). */
export type IaruRegion = 'ALL' | 'R1' | 'R2' | 'R3';

export interface FrequencyEntry {
  band: string;
  mode: string;
  hz: number;
  /** Dove vale la riga: tutte le regioni (`ALL`) o una sola. */
  region: IaruRegion;
  /** Riga `predefinita`: quella che la banda sceglie da sola; le altre sono alternative. */
  preferred: boolean;
}

export async function getFrequencies(): Promise<FrequencyEntry[]> {
  return (await (await request('/frequencies')).json()) as FrequencyEntry[];
}

/** La tabella iniziale di JTDX (FT8 e FT4), con le righe di ogni regione. */
export async function getFrequencyDefaults(): Promise<FrequencyEntry[]> {
  return (await (await request('/frequencies/defaults')).json()) as FrequencyEntry[];
}

export async function putFrequencies(entries: FrequencyEntry[]): Promise<void> {
  await request('/frequencies', { ...jsonInit(entries), method: 'PUT' });
}

export interface ScheduleEntry {
  hour: number;
  minute: number;
  band: string;
}

export interface ScheduleState {
  enabled: boolean;
  entries: ScheduleEntry[];
}

export async function getSchedule(): Promise<ScheduleState> {
  return (await (await request('/schedule')).json()) as ScheduleState;
}

export async function putSchedule(state: ScheduleState): Promise<void> {
  await request('/schedule', { ...jsonInit(state), method: 'PUT' });
}

/** `mdns`: l'indirizzo fisso in casa (`http://aurora.local:8787`), se l'annuncio e' attivo. */
export async function getAccess(): Promise<{ open: boolean; mdns?: string | null }> {
  return (await (await request('/access')).json()) as { open: boolean; mdns?: string | null };
}

export async function putAccess(open: boolean): Promise<void> {
  await request('/access', { ...jsonInit({ open }), method: 'PUT' });
}

export interface PairingSettings {
  code_ttl_secs: number;
  min_secs: number;
  max_secs: number;
}

export async function getPairingSettings(): Promise<PairingSettings> {
  return (await (await request('/pairing-settings')).json()) as PairingSettings;
}

export async function putPairingSettings(codeTtlSecs: number): Promise<void> {
  await request('/pairing-settings', { ...jsonInit({ code_ttl_secs: codeTtlSecs }), method: 'PUT' });
}

export interface PairedDevice {
  id: number;
  name: string;
  created_at: number;
}

export async function getDevices(): Promise<PairedDevice[]> {
  return (await (await request('/devices')).json()) as PairedDevice[];
}

export async function revokeDevice(id: number): Promise<void> {
  await request(`/devices/${id}`, { method: 'DELETE' });
}

export interface PairingOffer {
  code: string;
  expires_in: number;
  qr_svg: string;
  /** Il link del QR: `http://<IP>:8787/#/associa?codice=...&nome=aurora.local`. */
  link?: string | null;
  /** Gli indirizzi con cui il telefono raggiunge Aurora (IP della rete di casa e nome mDNS). */
  addresses?: string[];
  /** Ogni indirizzo con il suo link e il suo QR (il primo e' quello del QR grande); `kind`: wifi, cable, mdns, virtual, other. */
  options?: { url: string; link: string; kind: string; qr_svg: string | null }[];
  mdns?: string | null;
}

/** Un codice nuovo con il QR; il link punta all'indirizzo da cui si sta guardando la pagina. */
/** Rete del PC per l'associazione del telefono (solo dal PC). */
export interface NetworkCheck {
  ssid: string | null;
  category: string | null;
  firewall_allowed: boolean | null;
  addresses: [string, string][];
}

export async function getNetworkCheck(): Promise<NetworkCheck> {
  return (await (await request('/lan/check')).json()) as NetworkCheck;
}

/** Aggiunge la regola del firewall di Windows per Aurora (Windows chiede la conferma). */
export async function allowFirewall(): Promise<void> {
  await request('/lan/allow', { method: 'POST' });
}

export async function startPairing(baseUrl: string): Promise<PairingOffer> {
  return (await (await request('/pair/start', jsonInit({ base_url: baseUrl }))).json()) as PairingOffer;
}

export interface OutputLevelState {
  attenuation_db: number;
  min_db: number;
  max_db: number;
}

export async function getOutputLevel(): Promise<OutputLevelState> {
  return (await (await request('/output-level')).json()) as OutputLevelState;
}

export async function putOutputLevel(attenuationDb: number): Promise<void> {
  await request('/output-level', { ...jsonInit({ attenuation_db: attenuationDb }), method: 'PUT' });
}

/** Il volume di registrazione di Windows dell'ingresso audio e la regolazione automatica (AUTO). */
export interface InputLevelState {
  device: string;
  volume_pct: number;
  auto: boolean;
}

export async function getInputLevel(): Promise<InputLevelState> {
  return (await (await request('/input-level')).json()) as InputLevelState;
}

/** Con un volume scelto a mano AUTO si spegne, salvo che lo stesso invio lo riaccenda. */
export async function putInputLevel(change: { volume_pct?: number; auto?: boolean }): Promise<InputLevelState> {
  const body = (await (await request('/input-level', { ...jsonInit(change), method: 'PUT' })).json()) as { status: InputLevelState };
  return body.status;
}

/** Un messaggio dello storico (la riga com'e' nel core). */
export interface HistoryRow {
  id: number;
  ts_ms: number;
  band: string | null;
  mode: string;
  dial_hz: number;
  freq_hz: number;
  snr: number;
  dt: number;
  text: string;
  call: string | null;
  country: string | null;
  continent: string | null;
  tag: string | null;
  /** Ultimo invio a LoTW di chi trasmette (`AAAA-MM-GG`), calcolato a ogni lettura con l'elenco di adesso. */
  lotw_last_upload?: string | null;
}

export interface HistoryPage {
  total: number;
  rows: HistoryRow[];
}

export interface HistoryStats {
  total: number;
  oldest_ms: number | null;
  calls: number;
}

export async function getHistory(filter: HistoryFilter, offset: number, limit: number): Promise<HistoryPage> {
  const params = historyParams(filter, offset, limit, Date.now());
  return (await (await request(`/history?${params.toString()}`)).json()) as HistoryPage;
}

export async function getHistoryStats(): Promise<HistoryStats> {
  return (await (await request('/history/stats')).json()) as HistoryStats;
}

export async function clearHistory(): Promise<void> {
  await request('/history', { method: 'DELETE' });
}

/** Quali servizi online sono accesi (PSK Reporter, propagazione). */
export interface OnlineSettings {
  psk: boolean;
  solar: boolean;
  /** Calendario delle DXpedition da NG3K. */
  dxped?: boolean;
}

export interface PskReport {
  receiver: string;
  locator: string;
  snr: number | null;
  freq_hz: number;
  mode: string;
  /** Secondi Unix. */
  time_s: number;
  country: string | null;
}

export interface PskState {
  enabled: boolean;
  psk: { fetched_ms: number; callsign: string; hours: number; reports: PskReport[]; error: string | null };
}

export interface SolarData {
  updated: string;
  sfi: number | null;
  a_index: number | null;
  k_index: number | null;
  xray: string;
  sunspots: number | null;
  aurora: number | null;
  solar_wind: number | null;
  magnetic_field: number | null;
  geomag: string;
  signal_noise: string;
  muf: string;
  bands: Array<{ name: string; time: string; condition: string }>;
  vhf: Array<{ name: string; location: string; status: string }>;
}

export interface SolarState {
  enabled: boolean;
  solar: { fetched_ms: number; data: SolarData | null; error: string | null };
}

export async function getOnline(): Promise<OnlineSettings> {
  return (await (await request('/online')).json()) as OnlineSettings;
}

export async function putOnline(settings: OnlineSettings): Promise<OnlineSettings> {
  return (await (await request('/online', { ...jsonInit(settings), method: 'PUT' })).json()) as OnlineSettings;
}

export async function getPsk(): Promise<PskState> {
  return (await (await request('/online/psk')).json()) as PskState;
}

export async function getSolar(): Promise<SolarState> {
  return (await (await request('/online/solar')).json()) as SolarState;
}

/** Una DXpedition del calendario NG3K (date `AAAA-MM-GG`). */
export interface Dxpedition {
  start: string;
  end: string;
  country: string;
  call: string;
  qsl: string;
  info: string;
}

export interface DxpedState {
  enabled: boolean;
  dxped: { fetched_ms: number; items: Dxpedition[]; error: string | null };
}

export async function getDxped(): Promise<DxpedState> {
  return (await (await request('/online/dxped')).json()) as DxpedState;
}
/** Chiede subito i dati ai servizi accesi (al massimo una volta al minuto per servizio). */
export async function refreshOnline(): Promise<{ asked: boolean }> {
  return (await (await request('/online/refresh', { method: 'POST' })).json()) as { asked: boolean };
}

export interface AwardStats {
  qsos: number;
  dxcc_worked: number;
  dxcc_total: number;
  cq_zones_worked: number;
  cq_zones_total: number;
  itu_zones_worked: number;
  itu_zones_total: number;
  grids: number;
  prefixes: number;
  calls: number;
  bands: Array<{ band: string; dxcc: number; qsos: number }>;
}

export interface StatsResponse {
  stats: AwardStats;
  missing_dxcc: string[];
}

export interface ClusterServer {
  name: string;
  host: string;
  port: number;
  enabled: boolean;
}

export interface ClusterConfig {
  enabled: boolean;
  callsign: string;
  servers: ClusterServer[];
  login_commands: string[];
  max_spots: number;
  max_age_min: number;
}

export interface ClusterSpot {
  id: number;
  received_ms: number;
  server: string;
  spotter: string;
  freq_khz: number;
  dx_call: string;
  comment: string;
  time_utc: string;
  grid: string | null;
  band: string | null;
  mode: string | null;
  country: string | null;
  continent: string | null;
  /** Continente di chi ha mandato lo spot (assente con un core vecchio). */
  spotter_continent?: string | null;
}

export interface ClusterServerStatus {
  key: string;
  name: string;
  state: 'connecting' | 'connected' | 'error';
  detail: string | null;
  spots: number;
  since_ms: number;
}

export interface ClusterCatalogEntry {
  name: string;
  host: string;
  port: number;
  software: string;
  place: string;
  note: string;
}

/** Impostazioni, stato dei collegamenti e spot recenti del DX Cluster (`limit` spot al massimo). */
/** Un QSO concluso che aspetta la conferma («Richiedimi di loggare il QSO»). */
export interface PendingQso {
  id: number;
  qso: { call: string; grid: string; band: string; mode: string; freq_hz: number; rst_sent: string; rst_rcvd: string; time_on_unix_ms: number; name: string; qth: string; comment: string };
}

export async function getPendingQsos(): Promise<PendingQso[]> {
  return ((await (await request('/log/pending')).json()) as { pending: PendingQso[] }).pending;
}

/** Correzioni della finestra di conferma; un campo assente resta come l'ha scritto il motore. */
export interface PendingEdits {
  call?: string;
  grid?: string;
  rst_sent?: string;
  rst_rcvd?: string;
  name?: string;
  qth?: string;
  comment?: string;
}

export async function answerPendingQso(id: number, action: 'log' | 'discard', edits: PendingEdits = {}): Promise<void> {
  await request(`/log/pending/${id}`, jsonInit({ action, edits }));
}

/** `callsign`: il nominativo con cui si entra davvero (quello dei dati di stazione). */
export async function getClusterSpots(limit = 200): Promise<{ config: ClusterConfig; callsign?: string; servers: ClusterServerStatus[]; spots: ClusterSpot[] }> {
  return (await (await request(`/dx-cluster?limit=${limit}`)).json()) as { config: ClusterConfig; callsign?: string; servers: ClusterServerStatus[]; spots: ClusterSpot[] };
}

export async function putCluster(config: ClusterConfig): Promise<void> {
  await request('/dx-cluster', { ...jsonInit(config), method: 'PUT' });
}

/** Paese e zone di un nominativo, dal file dei prefissi (cty.dat); 404 se non lo conosce. */
export interface DxccInfo {
  country: string;
  continent: string;
  prefix: string;
  cq_zone: number;
  itu_zone: number;
  /** Centro del paese da cty.dat (nord ed est positivi). */
  lat?: number;
  lon?: number;
}

export async function getDxcc(call: string): Promise<DxccInfo> {
  return (await (await request(`/dxcc/${encodeURIComponent(call)}`)).json()) as DxccInfo;
}

/** Chi c'e' dietro un nominativo: paese da cty.dat e, con le ricerche accese, nome e citta' da QRZ.com o HamQTH. */
export interface CallsignInfo {
  call: string;
  dxcc: DxccInfo | null;
  person: { fname: string; name: string; city: string; country: string; state?: string; grid?: string; lotw?: boolean | null; eqsl?: boolean | null; direct?: boolean | null; bureau?: boolean | null; qsl_via?: string; source: string } | null;
  lookup: 'off' | 'found' | 'not_found' | 'error';
  subscriber: boolean | null;
  message: string | null;
  /** Ultimo invio a LoTW dall'elenco pubblico dell'ARRL (`AAAA-MM-GG`); assente con un motore vecchio. */
  lotw_last_upload?: string | null;
}

export async function getCallsign(call: string): Promise<CallsignInfo> {
  return (await (await request(`/callsign/${encodeURIComponent(call)}`)).json()) as CallsignInfo;
}

/** Prova un server: si collega e legge il benvenuto, senza dire chi siamo. */
export async function testClusterServer(host: string, port: number): Promise<{ ok: boolean; ms: number | null; banner: string | null; error: string | null }> {
  return (await (await request('/dx-cluster/test', jsonInit({ host, port }))).json()) as { ok: boolean; ms: number | null; banner: string | null; error: string | null };
}

/** I server DX Cluster pubblici noti. */
export async function getClusterCatalog(): Promise<ClusterCatalogEntry[]> {
  return (await (await request('/dx-cluster/catalog')).json()) as ClusterCatalogEntry[];
}

/** Manda un comando di consultazione a un cluster collegato (mai spot o messaggi). */
export async function sendClusterCommand(server: string, text: string): Promise<void> {
  await request('/dx-cluster/command', jsonInit({ server, text }));
}

export interface ClockConfig {
  enabled: boolean;
  servers: string[];
  interval_secs: number;
  warn_ms: number;
  auto_correct: boolean;
  /** All'avvio di Aurora si misura subito e, se fuori tolleranza, si corregge. */
  correct_at_start: boolean;
}

export interface ClockMeasurement {
  at_ms: number;
  server: string | null;
  offset_ms: number | null;
  delay_ms: number | null;
  servers_ok: number;
  servers_tried: number;
  error: string | null;
  corrected: boolean;
}

/** Client NTP: impostazioni e ultima misura dell'orologio del PC. */
export interface ClockState {
  config: ClockConfig;
  last: ClockMeasurement | null;
  /** L'ultima volta (ms Unix) che l'ora risultava giusta, anche in un avvio precedente. */
  last_good_ms?: number | null;
}

export async function getClock(): Promise<ClockState> {
  return (await (await request('/clock')).json()) as ClockState;
}

export async function putClock(config: ClockConfig): Promise<void> {
  await request('/clock', { ...jsonInit(config), method: 'PUT' });
}

/** Misura subito lo scarto dell'orologio. */
export async function checkClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/check', { method: 'POST' })).json()) as ClockMeasurement;
}

/** Sposta l'ora di Windows dello scarto misurato (serve l'amministratore, solo dal PC di casa). */
export async function correctClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/correct', { method: 'POST' })).json()) as ClockMeasurement;
}

export type CatBackend = 'hamlib' | 'omnirig' | 'tci' | 'ft991link' | 'cat4om' | 'none';
export type CatState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';
export type CatHandshake = 'default' | 'none' | 'xonxoff' | 'hardware';
export type CatPtt = 'cat' | 'dtr' | 'rts' | 'vox';
export type CatSplit = 'none' | 'rig' | 'emulate';
export type CatMode = 'none' | 'usb' | 'data';

/** La configurazione del CAT (`radio.json`): solo scelte tecniche, nessun segreto. */
export interface CatConfig {
  backend: CatBackend;
  model: number;
  rig_name: string;
  port: string;
  baud: number;
  data_bits: number;
  stop_bits: number;
  handshake: CatHandshake;
  force_dtr: boolean | null;
  force_rts: boolean | null;
  ptt: CatPtt;
  ptt_port: string;
  split: CatSplit;
  cat_mode: CatMode;
  civ_address: number;
  poll_secs: number;
  auto_connect: boolean;
  /** Legge a turno anche preamplificatore, AGC, NB/NR, guadagni, RIT/XIT e secondo VFO (spento di base). */
  read_extras?: boolean;
  /** «Accendi» e «Spegni» di JTDX: la radio si accende via CAT al primo collegamento e si spegne alla chiusura di Aurora. */
  power_on?: boolean;
  power_off?: boolean;
  /** Condivide `rigctld` con altri programmi (solo su 127.0.0.1) sulla porta fissa `share_port`. */
  share_enabled: boolean;
  share_port: number;
  /** Indirizzo `host:porta` del server TCI (ExpertSDR, Thetis); vuoto = `127.0.0.1:40001`. */
  tci_address?: string;
  /** Ricevitore della SDR comandato via TCI: 0 = RX1, 1 = RX2 (come «TCI Client RX1/RX2» di WSJT-X). */
  tci_receiver?: number;
  /** La radio di OmniRig: 1 o 2. */
  omnirig_rig?: number;
  /** Indirizzo host:porta del server UDP di FT-991(A) Link (di base 127.0.0.1:50001). */
  ft991_address?: string;
  /** Indirizzo host:porta della porta di gestione di Cat4OM (di base 127.0.0.1:17300): da li' Aurora scopre gruppo e porta Control. */
  cat4om_management?: string;
  /** Il gruppo di Cat4OM (id o nome); vuoto = scoperta automatica (il primo in funzione). */
  cat4om_group?: string;
  /** Porta Control del gruppo scritta a mano (host:porta); vuoto = quella scoperta. Il vecchio 127.0.0.1:17301 vale come vuoto. */
  cat4om_address?: string;
  /** Chiave del gruppo di Cat4OM (vuota se il gruppo non la chiede): resta sul PC, va solo nella prova di autenticazione. */
  cat4om_key?: string;
  /** La radio del gruppo di Cat4OM (radioId o nome); vuoto = la prima collegata. */
  cat4om_radio?: string;
}

export interface RigExtras {
  preamp_db?: number | null;
  att_db?: number | null;
  agc?: number | null;
  nb?: boolean | null;
  nr?: boolean | null;
  notch?: boolean | null;
  tuner?: boolean | null;
  rf_gain?: number | null;
  af_gain?: number | null;
  squelch?: number | null;
  power_set?: number | null;
  mic_gain?: number | null;
  rit_hz?: number | null;
  xit_hz?: number | null;
  vfo?: string | null;
  vfo_b_hz?: number | null;
  vfo_b_mode?: string | null;
}

export interface CatRigStatus {
  state: CatState;
  message: string;
  frequency_hz: number | null;
  mode: string | null;
  /** Larghezza del filtro della radio (Hz), se la radio la dice. */
  passband_hz?: number | null;
  ptt: boolean;
  split: boolean | null;
  failures: number;
  reconnect_attempt: number;
  ptt_blocked: boolean;
  updated_ms: number;
  /** I dati in piu' della radio (con «Leggi anche gli altri dati» acceso); ognuno solo se la radio lo da'. */
  extras?: RigExtras;
}

export interface CatStatus {
  backend: CatBackend;
  state: CatState;
  message: string;
  /** Senza `rigctld` Hamlib non puo' collegare nessuna radio. */
  rigctld_found: boolean;
  /** OmniRig installato su questo PC (solo Windows); assente nelle versioni vecchie del core. */
  omnirig_found?: boolean;
  rig: CatRigStatus | null;
  /** Dove collegare gli altri programmi, se la condivisione e' accesa e la radio e' collegata con Hamlib. */
  share?: { host: string; port: number } | null;
  /** Con Cat4OM: gestione, porta Control (scoperta o scritta), gruppo, radio e ruolo; assente nelle versioni vecchie del core. */
  cat4om?: Cat4omEndpoint | null;
}

export interface Cat4omEndpoint {
  management: string;
  /** La porta Control in uso o in prova; vuota finche' la scoperta non la trova. */
  control: string;
  /** `true` = porta Control trovata dalla scoperta, `false` = scritta a mano. */
  discovered: boolean;
  group: string;
  radio: string;
  /** `master`, `slave` o vuoto se non collegata. */
  role: string;
  /** Perche' l'audio in rete non va (testo del core); vuoto se va o se non e' scelto. Assente nelle versioni vecchie del core. */
  audio_error?: string;
}

export interface CatProfile {
  name: string;
  config: CatConfig;
}

export interface CatSnapshot {
  status: CatStatus;
  config: CatConfig;
  profiles: CatProfile[];
  active_profile: string | null;
}

export interface CatModel {
  id: number;
  manufacturer: string;
  model: string;
  status: string;
}

export interface CatPort {
  port: string;
  name: string;
  virtual: boolean;
  /** `vspe`, `com0com`, `bluetooth` o nulla se non riconoscibile. */
  virtual_kind?: string | null;
}

/** Una radio riconosciuta dal rilevamento passivo (identita' USB e nomi dei dispositivi: nessuna porta aperta). */
export interface CatCandidate {
  device_key: string;
  is_radio: boolean;
  protocol: string;
  rig_label: string;
  rig_token: string;
  rig_model: number | null;
  alternatives: string[];
  cat_port: string;
  other_ports: string[];
  baud: number | null;
  civ_address: number | null;
  audio_input: string | null;
  audio_output: string | null;
  confidence: number;
  evidence: string;
}

export interface CatDetection {
  candidates: CatCandidate[];
  audio_devices: string[];
  ports_seen: number;
}

/** Stato, configurazione e profili del CAT. */
export async function getCat(): Promise<CatSnapshot> {
  return (await (await request('/cat')).json()) as CatSnapshot;
}

/** Salva la configurazione e la applica (solo dal PC). */
export async function putCat(config: CatConfig): Promise<CatSnapshot> {
  return (await (await request('/cat', { ...jsonInit(config), method: 'PUT' })).json()) as CatSnapshot;
}

export async function connectCat(): Promise<CatSnapshot> {
  return (await (await request('/cat/connect', { method: 'POST' })).json()) as CatSnapshot;
}

/** Ripristina Hamlib (Windows, solo dal PC): riscarica il pacchetto ufficiale, ne controlla l'impronta e lo copia nei dati dell'utente. */
export async function restoreHamlib(): Promise<CatSnapshot> {
  return (await (await request('/cat/hamlib/restore', { method: 'POST' })).json()) as CatSnapshot;
}

export async function disconnectCat(): Promise<CatSnapshot> {
  return (await (await request('/cat/disconnect', { method: 'POST' })).json()) as CatSnapshot;
}

/** Profili con nome: salva, carica (scollega e non collega), nuovo, elimina. */
export async function catProfile(action: 'save' | 'load' | 'new' | 'delete', name?: string): Promise<CatSnapshot> {
  return (await (await request('/cat/profiles', jsonInit({ action, name }))).json()) as CatSnapshot;
}

/** Le impostazioni della radio che il display esteso cambia con un clic. */
export type CatExtraItem = 'nb' | 'nr' | 'notch' | 'atu' | 'tune' | 'pwr_up' | 'pwr_down' | 'preamp' | 'att' | 'agc' | 'vfo' | 'rit' | 'xit';

/** Il clic su un indicatore del display esteso: la radio cambia l'impostazione (inverte, passa al valore dopo, azzera) e risponde con i dati riletti. Solo dal PC. */
export async function postCatExtra(item: CatExtraItem): Promise<{ ok: boolean; item: string; extras: RigExtras | null }> {
  return (await (await request('/cat/extra', jsonInit({ item }))).json()) as { ok: boolean; item: string; extras: RigExtras | null };
}

/** Lo Split DX acceso dal cluster: ricezione sullo spot (VFO A), trasmissione sull'altro VFO. */
export type SplitDx = { active: false } | { active: true; rx_hz: number; tx_hz: number };

export async function getSplitDx(): Promise<SplitDx> {
  return (await (await request('/cat/split-dx')).json()) as SplitDx;
}

/** Doppio clic su uno spot con «UP 5», «QSX ...»: la radio riceve su `rxHz` e trasmette su `txHz` con lo split acceso. */
export async function postSplitDx(rxHz: number, txHz: number): Promise<SplitDx> {
  return ((await (await request('/cat/split-dx', jsonInit({ rx_hz: rxHz, tx_hz: txHz }))).json()) as { split: SplitDx }).split;
}

/** «Split DX: spegni»: la radio torna in simplex sulla ricezione. */
export async function deleteSplitDx(): Promise<void> {
  await request('/cat/split-dx', { method: 'DELETE' });
}

export async function getCatModels(): Promise<CatModel[]> {
  return ((await (await request('/cat/models')).json()) as { models: CatModel[] }).models;
}

export async function getCatPorts(): Promise<CatPort[]> {
  return ((await (await request('/cat/ports')).json()) as { ports: CatPort[] }).ports;
}

export async function detectCat(): Promise<CatDetection> {
  return (await (await request('/cat/detect', { method: 'POST' })).json()) as CatDetection;
}

/** Una radio di Cat4OM trovata da «Cerca». */
export interface Cat4omFoundRadio {
  /** `radioId`: va nel campo «Radio». */
  id: string;
  name: string;
  model: string;
  /** `connected`, `disconnected`, `connecting`, `reconnecting`, `no_response`, `stopped`, `unknown` (insieme aperto). */
  status: string;
  /** Il profilo di emulazione di Cat4OM (TCI, Hamlib) che tiene la radio: finche' c'e' Aurora non la comanda. */
  claimed_by: string | null;
  /** Audio in rete configurato in Cat4OM per ricezione e trasmissione (`null` = non detto). */
  audio_rx: boolean | null;
  audio_tx: boolean | null;
}

/** Un gruppo di Cat4OM trovato da «Cerca». */
export interface Cat4omFoundGroup {
  /** L'id del gruppo (va nel campo «Gruppo»); vuoto se la porta scritta a mano non ha risposto. */
  id: string;
  name: string;
  /** La porta Control `host:porta`; vuota se il gruppo e' fermo. */
  control: string;
  running: boolean;
  /** Viene dalla porta Control scritta a mano, non dalla gestione. */
  manual: boolean;
  /** Il Control dichiara l'audio in rete; `null` se non interrogato. */
  audio: boolean | null;
  radios: Cat4omFoundRadio[];
  /** Perche' il Control del gruppo non ha dato le radio (testo del core, vuoto se va bene). */
  error: string;
}

export interface Cat4omSearch {
  management: string;
  /** La gestione non ha risposto e l'elenco viene dalla porta Control scritta a mano (testo del core). */
  management_error: string;
  groups: Cat4omFoundGroup[];
}

/** «Cerca» di Cat4OM: gruppi e radio con i valori scritti nella pagina, senza toccare il collegamento in uso. Solo dal PC. */
export async function searchCat4om(values: { management?: string; address?: string; key?: string }): Promise<Cat4omSearch> {
  return (await (await request('/cat/cat4om/search', jsonInit(values))).json()) as Cat4omSearch;
}

/** Configurazione del rotore d'antenna (`rotore.json`): `launch` = Aurora lancia rotctld, `remote` = rotctld gia' acceso. */
export interface RotatorConfig {
  enabled: boolean;
  connection: 'launch' | 'remote';
  model: number;
  model_name: string;
  port: string;
  baud: number;
  host: string;
  tcp_port: number;
  /** Il campo di rotazione in gradi (di base 0..360; i rotori da 450 gradi arrivano a 450). */
  min_az: number;
  max_az: number;
}

export interface RotatorStatus {
  state: 'off' | 'connecting' | 'connected' | 'error';
  connected: boolean;
  az: number | null;
  el: number | null;
  target: number | null;
  moving: boolean;
  error: string | null;
  rotctld_found: boolean;
}

export interface RotatorSnapshot {
  status: RotatorStatus;
  config: RotatorConfig;
}

export async function getRotator(): Promise<RotatorSnapshot> {
  return (await (await request('/rotator')).json()) as RotatorSnapshot;
}

/** Salva la configurazione del rotore e rifa' il collegamento (solo dal PC). */
export async function putRotator(config: RotatorConfig): Promise<RotatorSnapshot> {
  return (await (await request('/rotator', { ...jsonInit(config), method: 'PUT' })).json()) as RotatorSnapshot;
}

/** Manda l'antenna a un azimut (gradi, 0 = nord). */
export async function rotatorGoto(az: number): Promise<RotatorStatus> {
  return ((await (await request('/rotator/goto', jsonInit({ az }))).json()) as { status: RotatorStatus }).status;
}

export async function rotatorStop(): Promise<RotatorStatus> {
  return ((await (await request('/rotator/stop', { method: 'POST' })).json()) as { status: RotatorStatus }).status;
}

export async function getRotatorModels(): Promise<CatModel[]> {
  return ((await (await request('/rotator/models')).json()) as { models: CatModel[] }).models;
}

/** Le impostazioni del motore di Aurora (nel suo file). */
export async function getEngineSettings(): Promise<EngineSettings> {
  return EngineSettingsSchema.parse(await (await request('/engine-settings')).json());
}

/** Segnala la stazione su DXSummit (solo dalla rete di casa): frequenza e modo li sa il core, qui servono la stazione, il suo locatore e il testo. */
export async function postSpotDxsummit(body: { dx_call: string; dx_grid?: string; text?: string; prop?: string; freq_khz?: number; info?: string }): Promise<SpotResult> {
  return SpotResultSchema.parse(await (await request('/spot/dxsummit', jsonInit(body))).json());
}

/** Il pulsante «Prova» dell'invio diretto dei QSO: controlla le impostazioni salvate senza registrare QSO. Un errore arriva come `ApiError` con il testo in `detail`. */
export async function postUploadTest(service: import('../lib/engineSettingsLayout').UploadService): Promise<string> {
  return UploadTestSchema.parse(await (await request('/qso-upload/test', jsonInit({ service }))).json()).message;
}

/** Salva e applica subito le modifiche; restituisce il documento aggiornato. */
export async function putEngineSettings(changes: Record<string, string>): Promise<EngineSettings> {
  const init = { ...jsonInit({ changes }), method: 'PUT' };
  return EngineSettingsSchema.parse(await (await request('/engine-settings', init)).json());
}

/** Un dispositivo audio del sistema (scheda Audio). */
export interface AudioDevice {
  name: string;
  /** Il numero massimo di canali: 1 mono, 2 stereo. */
  channels: number;
  is_default: boolean;
}

export interface AudioValues {
  input: string;
  output: string;
  input_channel: string;
  output_channel: string;
  save_dir: string;
  /** «0» Nessuno, «1» Decodificati, «2» Tutti. */
  save_mode: string;
}

export interface AudioDevices {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  input_in_use: string | null;
  output_in_use: string | null;
  /** Un'uscita nuova gia' aperta che subentra alla fine della trasmissione in corso. */
  output_pending: string | null;
  default_save_dir: string;
  /** Il valore che vuol dire «predefinito del sistema». */
  system_default: string;
  values: AudioValues;
}

export type AudioPartStatus = 'unchanged' | 'applied' | 'deferred' | 'saved_only' | 'failed';

export interface AudioPart {
  status: AudioPartStatus;
  in_use: string | null;
  /** Il dispositivo nuovo non si e' aperto e quello di prima e' in uso (di nuovo). */
  restored: boolean;
  message: string | null;
}

export interface AudioResult {
  ok: boolean;
  input: AudioPart;
  output: AudioPart;
  save: AudioPart;
  values: AudioValues;
}

export async function getAudioDevices(): Promise<AudioDevices> {
  return (await (await request('/audio/devices')).json()) as AudioDevices;
}

/** Apre i dispositivi nuovi, li applica e salva solo cio' che e' riuscito. */
export async function putAudioSettings(change: Partial<Omit<AudioValues, 'save_mode'>> & { save_mode?: number }): Promise<AudioResult> {
  return (await (await request('/audio/settings', { ...jsonInit(change), method: 'PUT' })).json()) as AudioResult;
}

export interface Log4omQso {
  date: string;
  call: string;
  band: string;
  mode: string;
  rst_sent: string;
  rst_rcvd: string;
  country: string;
  grid: string;
  /** Frequenza in Hz, nome e QTH (assenti con un core vecchio o se il log non li ha). */
  freq_hz?: number;
  name?: string;
  qth?: string;
}

export interface Log4omView {
  total: number;
  qsos: Log4omQso[];
}

export async function getLog4om(limit: number): Promise<Log4omView> {
  return (await (await request(`/log4om?limit=${limit}`)).json()) as Log4omView;
}

/** Una versione pubblicata di Aurora sul repository. */
export interface UpdateRelease {
  tag: string;
  version: string;
  name: string;
  notes: string;
  url: string;
  published_at: string;
  prerelease: boolean;
}

/** Stato degli aggiornamenti: versione in uso, impostazioni (senza il token) e ultimo esito. */
export interface UpdateStatus {
  current: string;
  enabled: boolean;
  repo: string;
  interval_hours: number;
  prereleases: boolean;
  has_token: boolean;
  checked_at_ms: number;
  available: UpdateRelease | null;
  latest: UpdateRelease | null;
  error: string | null;
  /** Il programma Windows sa installare la versione nuova da solo (pacchetto e firma ci sono). */
  can_install?: boolean;
  /** Lo scaricamento del pacchetto durante «Installa e riavvia». */
  download?: { file: string; done: number; total: number } | null;
  /** La preparazione dell'aggiornamento: il core scarica in anticipo la versione nuova e ne controlla la firma (Aurora 0.11.200+). */
  stage?: UpdateStage;
  /** Aggiornamento chiesto dal tray: niente locandina (un core piu' vecchio non lo manda). */
  quiet?: boolean;
}

/** A che punto e' la preparazione: `ready` = pacchetto completo e firmato, «Installa» non riscarica. */
export type UpdatePhase = 'idle' | 'downloading' | 'verifying' | 'ready' | 'installing' | 'error';

export interface UpdateStage {
  phase: UpdatePhase;
  version: string;
  file: string;
  done: number;
  total: number;
  percent: number | null;
  /** Velocita' media dello scaricamento (byte al secondo). */
  speed_bps: number;
  /** Secondi che mancano alla fine dello scaricamento. */
  eta_s: number | null;
  error: string | null;
}

/** Riprova a scaricare in anticipo la versione nuova (dopo uno scaricamento non riuscito): risponde subito, l'avanzamento e' in `stage`. */
export async function downloadUpdate(): Promise<UpdateStatus> {
  return (await (await request('/update/download', { method: 'POST' })).json()) as UpdateStatus;
}

/** Le impostazioni che si mandano: `token` assente = lo si lascia, vuoto = lo si toglie. */
export interface UpdateConfigInput {
  enabled: boolean;
  repo: string;
  interval_hours: number;
  prereleases: boolean;
  token?: string;
}

export async function getUpdate(): Promise<UpdateStatus> {
  return (await (await request('/update')).json()) as UpdateStatus;
}

export async function putUpdate(input: UpdateConfigInput): Promise<UpdateStatus> {
  return (await (await request('/update', { ...jsonInit(input), method: 'PUT' })).json()) as UpdateStatus;
}

/** Controlla subito se c'e' una versione nuova (serve internet). */
/** Installa la versione nuova (solo dal PC): scarica, controlla la firma, installa e riapre Aurora. */
export async function installUpdate(): Promise<{ version: string }> {
  return (await (await request('/update/install', { method: 'POST' })).json()) as { version: string };
}

/** Cosa offre la diagnostica da qui (`crash_folder`, `support_bundle`: solo dal PC stesso). */
export async function getDiagnostics(): Promise<{ crash_folder: boolean; support_bundle?: boolean }> {
  return (await (await request('/diagnostics')).json()) as { crash_folder: boolean; support_bundle?: boolean };
}

/** Il pacchetto per l'assistenza appena creato sul Desktop, con quanto serve al messaggio di posta. */
export interface SupportBundle {
  name: string;
  path: string;
  version: string;
  call: string;
  radio: string;
  ptt: string;
}

/** Crea sul Desktop lo zip per l'assistenza (log, crash, impostazioni senza password) e lo mostra in Esplora file. */
export async function prepareSupportBundle(lang: string): Promise<SupportBundle> {
  const body = JSON.stringify({ lang, local_offset_min: -new Date().getTimezoneOffset() });
  return (await (await request('/diagnostics/support-bundle', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body })).json()) as SupportBundle;
}

/** Riapre Esplora file sull'ultimo pacchetto creato. */
export async function revealSupportBundle(): Promise<void> {
  await request('/diagnostics/support-bundle/reveal', { method: 'POST' });
}

/** Apre in Esplora risorse la cartella dei crash (minidump e rapporti). */
export async function openCrashFolder(): Promise<void> {
  await request('/diagnostics/crash-folder', { method: 'POST' });
}

export async function checkUpdate(): Promise<UpdateStatus> {
  return (await (await request('/update/check', { method: 'POST' })).json()) as UpdateStatus;
}

/** La lista dei desiderati e gli ultimi avvisi. */
export async function getWishlist(): Promise<{ config: Wishlist; recent: WantedAlert[] }> {
  return (await (await request('/wishlist')).json()) as { config: Wishlist; recent: WantedAlert[] };
}

export async function putWishlist(config: Wishlist): Promise<Wishlist> {
  const body = (await (await request('/wishlist', { ...jsonInit(config), method: 'PUT' })).json()) as { config: Wishlist };
  return body.config;
}

/** Manda un avviso di prova; `phone` dice l'esito verso il telefono: `no_url`, `sent` o `failed`. */
export async function testWishlist(): Promise<'no_url' | 'sent' | 'failed'> {
  const body = (await (await request('/wishlist/test', { method: 'POST' })).json()) as { phone: 'no_url' | 'sent' | 'failed' };
  return body.phone;
}

/** Un quadrato locatore (4 caratteri) con QSO nel log: quanti e su quali bande. */
export interface WorkedSquare {
  grid: string;
  count: number;
  bands: string[];
}

/** Un quadrato lavorato su una banda in un modo, con le conferme (per la Mappa DX). */
export interface WorkedGridCell {
  grid: string;
  band: string;
  mode: string;
  qsos: number;
  confirmed: number;
  unconfirmed: number;
}

/** I quadrati locatore gia' lavorati (per la mappa); `cells` per banda, modo e conferma quando il core li ha. */
export async function getWorkedMap(): Promise<{ squares: WorkedSquare[]; cells?: WorkedGridCell[] }> {
  return (await (await request('/map/worked')).json()) as { squares: WorkedSquare[]; cells?: WorkedGridCell[] };
}

/** La scheda DXCC: paesi lavorati per banda e modo e, con `call`, le etichette NEW ONE / NEW BAND / NEW MODE / NEW GRID / GRID BAND / GRID MODE. */
export async function getDxccMatrix(query: DxccQuery = {}): Promise<DxccMatrix> {
  return (await (await request(`/dxcc-matrix${dxccQueryString(query)}`)).json()) as DxccMatrix;
}

/** Cosa ha fatto l'importazione di un file ADIF nel log interno. */
export interface LogImportResult {
  read: number;
  added: number;
  duplicates: number;
  skipped: number;
}

/** Aggiunge al log interno i QSO di un file ADIF scelto dall’utente, senza doppioni. Solo dalla rete di casa. */
export async function importLog(file: Blob): Promise<LogImportResult> {
  return (await (await request('/log/import', { method: 'POST', headers: { 'content-type': 'text/plain' }, body: file })).json()) as LogImportResult;
}

/** Copia delle impostazioni sul Desktop (solo dal PC stesso). */
export async function makeSettingsBackup(): Promise<{ ok: boolean; path: string; name: string; files: number }> {
  return (await (await request('/settings-backup', { method: 'POST' })).json()) as { ok: boolean; path: string; name: string; files: number };
}

/** Ripristino da una copia: i file si applicano alla prossima apertura di Aurora. */
export async function restoreSettingsBackup(file: Blob): Promise<{ ok: boolean; files: number }> {
  return (await (await request('/settings-restore', { method: 'POST', headers: { 'content-type': 'application/zip' }, body: file })).json()) as { ok: boolean; files: number };
}

/** I numeri dei QSO in forma corta, a intervalli: `[1,2,3,7,9,10]` -> `1-3,7,9-10` (l'indirizzo resta corto anche con migliaia di QSO). */
export function compactIds(ids: Iterable<number>): string {
  const sorted = [...new Set(ids)].sort((a, b) => a - b);
  const parts: string[] = [];
  let i = 0;
  while (i < sorted.length) {
    let j = i;
    while (j + 1 < sorted.length && sorted[j + 1] === sorted[j]! + 1) j++;
    parts.push(i === j ? `${sorted[i]}` : `${sorted[i]}-${sorted[j]}`);
    i = j + 1;
  }
  return parts.join(',');
}

/** L'indirizzo da cui si scarica il log interno come file ADIF (`from` e `to` come `AAAA-MM-GG`, facoltativi; `ids` = solo quei QSO). */
export function logExportUrl(from?: string, to?: string, ids?: Iterable<number>): string {
  const p = new URLSearchParams();
  if (from) p.set('from', from);
  if (to) p.set('to', to);
  if (ids) p.set('ids', compactIds(ids));
  const q = p.toString();
  return `${BASE}/log/export.adi${q ? `?${q}` : ''}`;
}

/** Un QSO del log interno, per la pagina di esportazione. `date` e' `AAAA-MM-GG`, `time` `HH:MM:SS` (UTC). */
export interface LogQso {
  id: number;
  date: string;
  time: string;
  call: string;
  band: string;
  mode: string;
  freq_hz: number;
  rst_sent: string;
  rst_rcvd: string;
  grid: string;
  country: string;
  /** Nome e QTH del corrispondente (assenti con un core vecchio). */
  name?: string;
  qth?: string;
}

export interface LogQsos {
  total: number;
  qsos: LogQso[];
  /** Vero se da qui «Salva ADIF…» apre la finestra di Windows (programma Windows, dal PC stesso). */
  save_dialog: boolean;
}

export async function getLogQsos(): Promise<LogQsos> {
  return (await (await request('/log/qsos')).json()) as LogQsos;
}

/** Cancella i QSO scelti, o tutto il log con `all: true`. */
export async function deleteLogQsos(choice: { ids: number[] } | { all: true }): Promise<{ deleted: number }> {
  return (await (await request('/log/qsos', { method: 'DELETE', headers: { 'content-type': 'application/json' }, body: JSON.stringify(choice) })).json()) as { deleted: number };
}

/** Un QSO scritto a mano (data `AAAA-MM-GG` e ora `HH:MM` UTC). */
export interface ManualQso {
  call: string;
  date: string;
  time: string;
  band: string;
  freq_hz: number;
  mode: string;
  rst_sent: string;
  rst_rcvd: string;
  grid: string;
  name: string;
  qth?: string;
  comment: string;
}

export async function addLogQso(q: ManualQso): Promise<void> {
  await request('/log/qsos', jsonInit(q));
}

/** Il database di Log4OM2 per i QSO gia' fatti: percorso, da dove viene (`auto`, `chosen`, `env`, `none`), se e' bloccato da aurora.env. */
export interface Log4omDb {
  path: string | null;
  exists: boolean;
  source: 'auto' | 'chosen' | 'env' | 'none';
  locked: boolean;
  /** Si puo' aprire la finestra «Apri file» (programma Windows, dal PC stesso). */
  dialog?: boolean;
  /** QSO trovati nel file appena scelto. */
  count?: number | null;
}

export async function getLog4omDb(): Promise<Log4omDb> {
  return (await (await request('/log4om-db')).json()) as Log4omDb;
}

/** Cambia il database (`null` = non usarlo piu'): il core lo controlla e ricarica subito i QSO gia' fatti. */
export async function setLog4omDb(path: string | null): Promise<Log4omDb> {
  return (await (await request('/log4om-db', { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ path }) })).json()) as Log4omDb;
}

/** Apre la finestra «Apri file» del programma Windows per scegliere il database. */
export async function chooseLog4omDb(): Promise<Log4omDb | { cancelled: true }> {
  return (await (await request('/log4om-db/choose', { method: 'POST' })).json()) as Log4omDb | { cancelled: true };
}

export type LogSaveResult = { ok: true; cancelled: true } | { ok: true; cancelled: false; path: string; count: number };

/** Salva i QSO scelti (o tutti) con la finestra «Salva con nome» del programma Windows. */
export async function saveLogAdif(choice: { ids: number[] } | { all: true }, format: 'adif' | 'cabrillo' = 'adif'): Promise<LogSaveResult> {
  return (await (await request('/log/export-save', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...choice, format }) })).json()) as LogSaveResult;
}

export async function getStats(): Promise<StatsResponse> {
  return (await (await request('/stats')).json()) as StatsResponse;
}

export type ControlId =
  | 'autoseq' | 'tx_period' | 'single_qso' | 'answer_all' | 'jt_both' | 'ap_list' | 'mtd' | 'wait_call' | 'pounce' | 'answer_b4' | 'bypass' | 'skip_tx1' | 'rrr' | 'split' | 'show_wanted'
  | 'auto_tx' | 'enable_tx' | 'hound' | 'jt' | 'monitor' | 'swl' | 'sync' | 'agc' | 'filter' | 'decode' | 'erase' | 'stop_rx' | 'clear_dx' | 'gen_msgs' | 'rx_to_tx' | 'tx_to_rx' | 'hold_tx' | 'fox' | 'q65_gauss' | 'q65_clear_avg' | 'jt65_clear_avg' | 'q65_eme' | 'q65_drift' | 'q65_contest' | 'msk_sh' | 'msk144_period' | 'wspr_dbm' | 'wspr_pct' | 'wspr_type1';

/** Stato degli interruttori del motore (AutoSeq, 1 QSO, Abilita Tx...). */
export async function getControls(): Promise<Partial<Record<ControlId, boolean>>> {
  const body = (await (await request('/controls')).json()) as { controls?: Partial<Record<ControlId, boolean>>; values?: unknown; tx?: unknown };
  // la stessa lettura porta anche i sei messaggi Tx del motore e quello selezionato
  useEngineTx.getState().set(parseEngineTx(body.tx));
  // e i controlli a valore numerico (la deriva di Q65)
  useControlValues.getState().set(parseControlValues(body.values));
  return body.controls ?? {};
}

/** Porta un interruttore allo stato voluto, o preme un pulsante semplice (senza `on`). */
export async function postControl(id: ControlId, on?: boolean): Promise<void> {
  await request('/commands/control', jsonInit(on === undefined ? { id } : { id, on }));
}

/** Imposta un controllo a valore numerico del motore (la deriva massima di Q65, 0-50 a passi di 5). */
export async function postControlValue(id: ControlId, value: number): Promise<void> {
  await request('/commands/control', jsonInit({ id, value }));
  useControlValues.getState().set({ ...useControlValues.getState().values, [id]: value });
}

/** I rapporti degli errori (facoltativi, spenti di fabbrica): impostazioni, identificativo anonimo, coda e crash da segnalare. */
export interface ReportsState {
  ask_after_crash: boolean;
  auto_errors: boolean;
  /** Una volta al giorno «questa installazione e' aperta» (codice casuale, versione, sistema). */
  count_active?: boolean;
  install_id: string;
  /** L'indirizzo del servizio e' stato scritto nel programma (non e' il segnaposto): senza, non parte niente. */
  configured: boolean;
  endpoint: string;
  crash_pending: boolean;
  /** I rapporti che aspettano la rete, esattamente come partiranno. */
  queue: unknown[];
  waiting: number;
  sent_today: number;
  max_per_day: number;
  /** Un rapporto d'esempio con i dati di questa stazione, gia' ripulito. */
  sample: unknown;
}

/** I file del pacchetto senza dati personali, con il testo che partirebbe. */
export interface ReportsPreview {
  files: { name: string; bytes: number; text: string }[];
  bytes: number;
  configured: boolean;
  endpoint: string;
}

export async function getReports(): Promise<ReportsState> {
  return (await (await request('/reports')).json()) as ReportsState;
}

export async function putReports(input: { ask_after_crash?: boolean; auto_errors?: boolean; count_active?: boolean }): Promise<ReportsState> {
  return (await (await request('/reports', { ...jsonInit(input), method: 'PUT' })).json()) as ReportsState;
}

/** Un identificativo dell'installazione nuovo. */
export async function newReportsId(): Promise<ReportsState> {
  return (await (await request('/reports/install-id', { method: 'POST' })).json()) as ReportsState;
}

export async function previewCrashReport(): Promise<ReportsPreview> {
  return (await (await request('/reports/crash/preview', { method: 'POST' })).json()) as ReportsPreview;
}

export async function sendCrashReport(): Promise<{ ok: boolean; code: string }> {
  return (await (await request('/reports/crash/send', { method: 'POST' })).json()) as { ok: boolean; code: string };
}

export async function dismissCrashReport(): Promise<void> {
  await request('/reports/crash/dismiss', { method: 'POST' });
}

/** Manda a chi sviluppa Aurora il pacchetto senza dati personali; il codice breve si dice a chi aiuta. */
export async function sendSupportBundle(): Promise<{ ok: boolean; code: string }> {
  return (await (await request('/reports/bundle/send', { method: 'POST' })).json()) as { ok: boolean; code: string };
}

/** QMAP: da dove arriva l'I/Q e cosa cercare. */
export interface QmapConfig {
  source: 'off' | 'tci' | 'file';
  tci_address: string;
  sample_rate: number;
  file_path: string;
  center_hz: number;
  period_s: number;
  submodes: number[];
  span_hz: number;
  fading: boolean;
  /** Con 60 s cerca anche i Q65-30 nelle due meta' del minuto. */
  also_30: boolean;
  /** Media dei periodi per ogni segnale. */
  average: boolean;
}

/** Un messaggio Q65 trovato nella fetta. */
export interface QmapDecode {
  offset_hz: number;
  rf_hz: number | null;
  /** Frequenza «sked»: meno meta' del Doppler della propria eco. */
  sked_hz: number | null;
  period_s: number;
  submode: number;
  mode: string;
  /** Meta' del minuto per i Q65-30 (0 prima, 1 seconda). */
  iseq: number;
  /** Periodi sommati (1 = uno solo). */
  averaged: number;
  snr_db: number;
  dt_s: number;
  text: string;
  sync: number;
}

export interface QmapSnapshot {
  config: QmapConfig;
  state: 'off' | 'connecting' | 'receiving' | 'decoding' | 'error';
  message: string;
  sample_rate: number;
  center_hz: number | null;
  decodes: QmapDecode[];
  candidates: number;
  spectrum_db: number[];
  spectrum_low_hz: number;
  spectrum_high_hz: number;
  period_start_ms: number;
  updated_ms: number;
  /** Secondi di dati dell'ultimo passaggio. */
  pass_s: number;
  /** Segnali con una media in corso. */
  averaging: number;
  /** La Luna e il Doppler della propria eco (serve il locatore nelle impostazioni). */
  moon: QmapMoon | null;
}

export interface QmapMoon {
  az_deg: number;
  el_deg: number;
  self_doppler_hz: number;
  freq_mhz: number;
  echo_delay_s: number;
  dec_deg: number;
}

export async function getQmap(): Promise<QmapSnapshot> {
  return (await (await request('/qmap')).json()) as QmapSnapshot;
}

export async function putQmap(config: QmapConfig): Promise<QmapSnapshot> {
  return (await (await request('/qmap', { ...jsonInit(config), method: 'PUT' })).json()) as QmapSnapshot;
}

export async function postQmapDecode(): Promise<void> {
  await request('/qmap/decode', { method: 'POST' });
}

/** «Cancella media». */
export async function postQmapClearAverage(): Promise<void> {
  await request('/qmap/clear-average', { method: 'POST' });
}

/** Clic su un messaggio: Q65 con il suo periodo e sottomodo, radio con il tono 0 a 1000 Hz. */
export async function postQmapTune(rf_hz: number, period_s: number, submode: number): Promise<{ ok: boolean; dial_hz: number; mode: string }> {
  return (await (await request('/qmap/tune', jsonInit({ rf_hz, period_s, submode }))).json()) as { ok: boolean; dial_hz: number; mode: string };
}

// --- SSTV (crate mode-sstv) ---

export interface SstvModeInfo {
  id: string;
  name: string;
  width: number;
  height: number;
  vis: number;
  seconds: number;
  color: 'rgb' | 'yuv';
}

export interface SstvSettings {
  default_mode: string;
  auto_save: boolean;
  save_tx: boolean;
  auto_receive: boolean;
  lock_mode: string | null;
  folder: string;
  /** Cartella da cui si prendono le foto da trasmettere (vuota: nessuna). */
  tx_folder: string;
  rx_offset_hz: number;
  tx_offset_hz: number;
  slant_ppm: number;
  /** Il nominativo della stazione in FSK dopo ogni immagine (formato di MMSSTV). Assente con un motore vecchio. */
  fsk_id?: boolean;
  /** Modo riconosciuto anche senza VIS, dal periodo delle righe. Assente con un motore vecchio. */
  auto_sync?: boolean;
}

export interface SstvRxStatus {
  listening: boolean;
  receiving: boolean;
  mode: string | null;
  width: number;
  height: number;
  rows: number;
  sync: number;
  offset_hz: number;
  level_db: number;
  last_image: string | null;
  /** Come e' partita l'immagine: `vis`, `sync` (dal periodo delle righe, senza VIS) o `forced`. */
  source?: string | null;
  /** L'ultimo nominativo letto dall'FSK ID dopo un'immagine. */
  last_call?: string | null;
}

export interface SstvTxStatus {
  state: string;
  mode: string | null;
  progress: number;
  elapsed_s: number;
  total_s: number;
  message: string | null;
}

export interface SstvSnapshot {
  rx: SstvRxStatus;
  tx: SstvTxStatus;
  settings: SstvSettings;
  folder: string;
  can_transmit: boolean;
  modes: SstvModeInfo[];
}

export interface SstvImageEntry {
  id: string;
  file: string;
  direction: string;
  mode: string;
  width: number;
  height: number;
  time_ms: number;
  sync_quality?: number;
  offset_hz?: number;
  /** Scarto dell'orologio misurato su questa immagine (ppm). */
  clock_ppm?: number;
  complete: boolean;
  call?: string;
}

export async function getSstv(): Promise<SstvSnapshot> {
  return (await (await request('/sstv')).json()) as SstvSnapshot;
}

export async function putSstv(settings: SstvSettings): Promise<SstvSnapshot> {
  return (await (await request('/sstv', { ...jsonInit(settings), method: 'PUT' })).json()) as SstvSnapshot;
}

// i comandi dell'ascolto SSTV arrivano al core nell'ordine in cui sono partiti (uscire e rientrare subito nella pagina)
const sstvRxQueue = inOrder();

export async function sstvRx(action: 'start' | 'stop' | 'cancel'): Promise<void> {
  await sstvRxQueue(() => request(`/sstv/rx/${action}`, { method: 'POST' }));
}

export async function sstvForce(mode: string): Promise<void> {
  await request('/sstv/rx/force', jsonInit({ mode }));
}

export async function getSstvImages(): Promise<SstvImageEntry[]> {
  return ((await (await request('/sstv/images')).json()) as { images: SstvImageEntry[] }).images;
}

/** Indirizzo del PNG di un'immagine dell'archivio (per <img>). */
/** Le foto della cartella scelta per la trasmissione (solo dal PC di Aurora). */
export async function getSstvTxFolder(): Promise<{ folder: string; files: { name: string; size: number }[] }> {
  return (await (await request('/sstv/tx-folder')).json()) as { folder: string; files: { name: string; size: number }[] };
}

export async function getSstvTxPhoto(name: string): Promise<Blob> {
  return await (await request('/sstv/tx-folder/file?name=' + encodeURIComponent(name))).blob();
}

/** Cancella piu' immagini dell'archivio in una volta. */
export async function deleteSstvImages(ids: string[]): Promise<number> {
  const res = await request('/sstv/images/delete', jsonInit({ ids }));
  return ((await res.json()) as { deleted: number }).deleted;
}

/** Cancella delle foto dalla cartella da cui si trasmette (solo dal PC di Aurora; vanno via dal disco). */
export async function deleteSstvTxPhotos(names: string[]): Promise<number> {
  const res = await request('/sstv/tx-folder/delete', jsonInit({ names }));
  return ((await res.json()) as { deleted: number }).deleted;
}

export function sstvTxPhotoUrl(name: string): string {
  return `${BASE}/sstv/tx-folder/file?name=${encodeURIComponent(name)}`;
}

export function sstvImageUrl(id: string): string {
  return `${BASE}/sstv/images/${encodeURIComponent(id)}`;
}

/** L'immagine pulita dal rumore (ordine 1..6, levigatezza 0,05..2); l'originale non cambia. */
export function sstvImageCleanUrl(id: string, order: number, smooth: number): string {
  return `${BASE}/sstv/images/${encodeURIComponent(id)}/clean?order=${order}&smooth=${smooth}`;
}

export async function getSstvImageCleanBlob(id: string, order: number, smooth: number): Promise<Blob> {
  return (await request(`/sstv/images/${encodeURIComponent(id)}/clean?order=${order}&smooth=${smooth}`)).blob();
}

export async function getSstvImageBlob(id: string): Promise<Blob> {
  return (await request(`/sstv/images/${encodeURIComponent(id)}`)).blob();
}

/** L'immagine in arrivo (o l'ultima finita) pulita dal rumore; `v` cambia l'indirizzo quando l'immagine cambia. */
export function sstvCurrentCleanUrl(order: number, smooth: number, v: number): string {
  return `${BASE}/sstv/rx/current/clean?order=${order}&smooth=${smooth}&v=${v}`;
}

export async function getSstvCurrentCleanBlob(order: number, smooth: number): Promise<Blob> {
  return (await request(`/sstv/rx/current/clean?order=${order}&smooth=${smooth}`)).blob();
}

export async function getSstvCurrentBlob(): Promise<Blob> {
  return (await request('/sstv/rx/current')).blob();
}

export async function deleteSstvImage(id: string): Promise<void> {
  await request(`/sstv/images/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/** Trasmette l'immagine composta (PNG alla risoluzione del modo). */
export async function sstvTransmit(mode: string, png: Blob, call?: string | null): Promise<SstvTxStatus> {
  const q = new URLSearchParams({ mode });
  if (call) q.set('call', call);
  const res = await request(`/sstv/tx?${q.toString()}`, { method: 'POST', headers: { 'content-type': 'image/png' }, body: png });
  return ((await res.json()) as { tx: SstvTxStatus }).tx;
}

export async function sstvAbort(): Promise<void> {
  await request('/sstv/tx/abort', { method: 'POST' });
}

// ---- Modi testuali (PSK31/PSK63, RTTY): vedi docs/api.md ----

/** Una macro: `name` e' il tasto («F5», «Ctrl-F5»...) o un evento, `color` lo sfondo (0xRRGGBB). */
export interface DigiMacro {
  name: string;
  label: string;
  text: string;
  color: number | null;
}

export type RttyCharset = 'ita2' | 'lowercase' | 'cyrillic' | 'swedish';

/** Un profilo RTTY (Set 1-4): spostamento, velocita', Rev, ricevitore, set di caratteri, AFC, Snap, Squelch, UOS e soglia. */
export interface RttyProfile {
  shift_hz: number;
  baud: number;
  reverse: boolean;
  receiver: 'auto' | 'filters' | 'discriminator';
  charset: RttyCharset;
  afc: boolean;
  snap: boolean;
  squelch: boolean;
  uos: boolean;
  squelch_snr_db: number;
}

export interface DigiSettings {
  mode: 'psk31' | 'psk63' | 'rtty' | 'cw';
  /** Velocita' della CW in parole al minuto. */
  cw_wpm: number;
  /** CW: banda del filtro di ricezione (50, 100, 150, 250 o 500 Hz). */
  cw_bandwidth_hz: number;
  /** CW: soglia dello squelch sul rapporto segnale/rumore (dB in 3 kHz, -30..+20). */
  cw_squelch_snr_db: number;
  /** CW: decodifica SOM di fldigi (per il tasto a mano). */
  cw_som: boolean;
  /** Anti-click della cascata mentre la pagina ascolta (acceso di base). */
  wf_anticlick: boolean;
  /** PSK: soglia dello squelch sulla qualita' della fase (0-100, come fldigi). */
  psk_squelch: number;
  rx_freq_hz: number;
  tx_follow_rx: boolean;
  tx_freq_hz: number;
  rtty_shift_hz: number;
  rtty_baud: number;
  /** RTTY: mark e space scambiati a mano («Rev»); con la radio in LSB il motore li inverte gia' da solo. */
  rtty_reverse: boolean;
  /** RTTY: UOS, dopo uno spazio si torna alle lettere (acceso di base, come fldigi). */
  rtty_uos: boolean;
  /** RTTY: soglia dello squelch sul rapporto segnale/rumore (dB, 0-40). */
  rtty_squelch_snr_db: number;
  /** RTTY: ricevitore a due filtri, a discriminatore di frequenza o scelto dallo spostamento (`auto`: discriminatore sotto i 40 Hz). */
  rtty_receiver: 'auto' | 'filters' | 'discriminator';
  /** RTTY: set di caratteri (ITA2 maiuscolo di base, minuscole, cirillico MTK-2, svedese). */
  rtty_charset: RttyCharset;
  /** RTTY: i quattro profili della finestra «Impostazioni RTTY» e quello scelto (0-3), gia' copiato nei campi qui sopra. */
  rtty_profile: number;
  rtty_profiles: RttyProfile[];
  afc_on: boolean;
  /** Squelch acceso: la soglia e' quella del modo (`psk_squelch`, `rtty_squelch_snr_db`, `cw_squelch_snr_db`), sempre relativa. */
  squelch_on: boolean;
  snap_on: boolean;
  auto_receive: boolean;
  macros: DigiMacro[];
  /** Le macro di un modo (`psk31`, `psk63`, `rtty`): sostituiscono quelle di base con lo stesso nome. */
  mode_macros: Record<string, DigiMacro[]>;
  /** Per modo: `false` = le macro del modo sostituiscono quelle di base (opzione «Merge»); senza voce si uniscono. */
  merge_macros: Record<string, boolean>;
  /** CW: si decodificano tutti i segnali della banda audio (pannello «Segnali»). */
  band_cw?: boolean;
  /** PSK31 e PSK63: tutti i segnali della banda audio («browser», pannello «Segnali»). */
  band_psk?: boolean;
  /** Tutta la banda: i nominativi verificati come spot locali nel pannello del DX Cluster di Aurora (mai in rete). */
  band_spots?: boolean;
  /** Tutta la banda: spot locali solo di chi chiama CQ. */
  band_spots_cq_only?: boolean;
  /** Tutta la banda: convalida dei nominativi (`minimal`, `normal`, `strict`, `maximum`). */
  band_validation?: string;
}

/** I segnali di tutta la banda con lo storico di 60 s di ciascuno. */
export async function getDigiBand(): Promise<DigiSignal[]> {
  const body = (await (await request('/digi/band')).json()) as { signals?: unknown };
  const parsed = DigiSignalSchema.array().safeParse(body.signals ?? []);
  return parsed.success ? parsed.data : [];
}

/** Svuota tutta la banda: nel core i canali, il loro storico e i nominativi ripartono da zero. */
export async function clearDigiBand(): Promise<void> {
  await request('/digi/band/clear', { method: 'POST' });
}

export interface DigiSnapshot {
  /** `snr_db`: rapporto segnale/rumore del modo (RTTY quello dello squelch, PSK come fldigi, CW in 3 kHz); `imd_db` e `quality` (0-100) solo
   * in PSK; `wpm` la velocita' capita in CW; `radio_lsb`: la radio e' in LSB (RTTY invertito da solo). */
  rx: { listening: boolean; level_db: number; freq_hz: number; squelch_open: boolean; snr_db?: number; imd_db?: number; quality?: number; wpm?: number; radio_lsb?: boolean };
  /** `open`: trasmissione aperta a levetta (resta in trasmissione, il testo scritto parte mentre si scrive). */
  tx: { state: string; progress: number; elapsed_s: number; total_s: number; message: string | null; open?: boolean };
  settings: DigiSettings;
  can_transmit: boolean;
  modes: { id: string; name: string }[];
}

export async function getDigi(): Promise<DigiSnapshot> {
  return (await (await request('/digi')).json()) as DigiSnapshot;
}

export async function putDigi(settings: DigiSettings): Promise<DigiSnapshot> {
  return (await (await request('/digi', { ...jsonInit(settings), method: 'PUT' })).json()) as DigiSnapshot;
}

// i comandi dell'ascolto dei modi testuali arrivano al core nell'ordine in cui sono partiti (uscire e rientrare subito nella pagina)
const digiRxQueue = inOrder();

export async function digiRx(action: 'start' | 'stop'): Promise<void> {
  await digiRxQueue(() => request(`/digi/rx/${action}`, { method: 'POST' }));
}

export async function digiTransmit(text: string): Promise<void> {
  await request('/digi/tx', jsonInit({ text }));
}

export interface DigiImported {
  folder: string;
  default_count: number;
  default_file: string | null;
  modes: Record<string, number>;
  skipped: number;
}

/** Importa le macro dai file di una cartella dell'utente, oppure (`defaults`) rimette quelle di Aurora; solo dal PC di Aurora. */
export async function importMacros(how: 'replace' | 'add' | 'defaults' = 'replace', folder?: string): Promise<{ imported: DigiImported; snapshot: DigiSnapshot }> {
  return (await (await request('/digi/import-macros', jsonInit({ how, ...(folder ? { folder } : {}) }))).json()) as { imported: DigiImported; snapshot: DigiSnapshot };
}

/** Una stringa CAT mandata alla radio cosi' com'e' (macro `<CATCMD:...>`); solo dal PC di Aurora e con Hamlib. */
export async function digiCat(cmd: string): Promise<void> {
  await request('/digi/cat', jsonInit({ cmd }));
}

/** Il testo di un file della cartella dei testi delle macro (macro `<TEXTFILE:nome>`); solo dal PC di Aurora. */
export async function digiTextFile(name: string): Promise<string> {
  return ((await (await request('/digi/textfile?name=' + encodeURIComponent(name))).json()) as { text: string }).text;
}

export async function digiAbort(): Promise<void> {
  await request('/digi/tx/abort', { method: 'POST' });
}

/** Apre la trasmissione (levetta RX/TX): resta in trasmissione finche' non si chiude; `text` e' il testo gia' scritto. */
export async function digiTxOpen(text: string): Promise<void> {
  await request('/digi/tx/open', jsonInit({ text }));
}

/** Testo scritto durante la trasmissione: si accoda a quello che sta partendo. */
export async function digiTxType(text: string): Promise<void> {
  await request('/digi/tx/type', jsonInit({ text }));
}

/** Toglie fino a `count` caratteri non ancora partiti dalla fine del testo in coda; restituisce quanti ne ha tolti. */
export async function digiTxUnsend(count: number): Promise<number> {
  return ((await (await request('/digi/tx/unsend', jsonInit({ count }))).json()) as { removed: number }).removed;
}

/** Sostituisce il testo in coda dalla posizione `from` con `text` senza fermare la trasmissione; falso se `from` e' gia' modulato. */
export async function digiTxReplace(from: number, text: string): Promise<boolean> {
  return ((await (await request('/digi/tx/replace', jsonInit({ from, text }))).json()) as { replaced: boolean }).replaced;
}

/** Chiude la trasmissione aperta: finito il testo gia' scritto si torna in ricezione. */
export async function digiTxClose(): Promise<void> {
  await request('/digi/tx/close', { method: 'POST' });
}

// ---- Satelliti (orbite, passaggi, Doppler sulla radio e rotore): vedi docs/api.md ----

export interface SatTransponder {
  name: string;
  mode: string;
  up_low_hz: number;
  up_high_hz: number;
  down_low_hz: number;
  down_high_hz: number;
  inverting: boolean;
  tone_hz: number | null;
}

export interface SatInfo {
  norad: number;
  name: string;
  transponders: SatTransponder[];
  notes: string;
}

export interface SatSettings {
  selected: number | null;
  tracked: number[];
  transponder: number;
  down_hz: number | null;
  tx_offset_hz: number;
  doppler_on: boolean;
  rotor_on: boolean;
  rotor_preposition: boolean;
  min_el_deg: number;
  map: string;
  /** I server da cui si scaricano i dati delle orbite. */
  tle_sources: string[];
  /** Ogni quante ore si aggiornano da soli; 0 = solo a mano. */
  tle_update_hours: number;
}

export interface SatLook {
  az_deg: number;
  el_deg: number;
  range_km: number;
  range_rate_kms: number;
  lat_deg: number;
  lon_deg: number;
  alt_km: number;
  speed_kms: number;
  rising: boolean;
}

export interface SatPass {
  aos_unix: number;
  los_unix: number;
  max_el_deg: number;
  max_el_unix: number;
  aos_az_deg: number;
  los_az_deg: number;
  max_az_deg: number;
  duration_s: number;
}

export interface SatPlan {
  down_nominal_hz: number;
  up_nominal_hz: number;
  rx_hz: number;
  tx_hz: number;
  rx_shift_hz: number;
  tx_shift_hz: number;
}

export interface SatNow {
  time_unix: number;
  norad: number | null;
  name: string;
  look: SatLook | null;
  visible: boolean;
  plan: SatPlan | null;
  radio: { ready: boolean; error: string | null };
  radio_active: boolean;
  rotor: { ready: boolean; active: boolean; az_deg: number; el_deg: number; error: string | null };
  next_pass: SatPass | null;
  next_path: [number, number][];
}

export interface SatSnapshot {
  observer: { lat_deg: number; lon_deg: number } | null;
  tle: { count: number; age_s: number | null; presets?: { name: string; url: string }[] };
  listing: { norad: number; name: string; has_tle: boolean }[];
  catalog: SatInfo[];
  settings: SatSettings;
  now: SatNow;
  sky: { sun_lat_deg: number; sun_lon_deg: number; moon_lat_deg: number; moon_lon_deg: number; time_unix: number };
  details: { norad: number; name: string; inclination_deg: number; eccentricity: number; revs_per_day: number; period_min: number; epoch_unix: number } | null;
  maps: string[];
}

export interface SatPosition {
  norad: number;
  name: string;
  lat_deg: number;
  lon_deg: number;
  alt_km: number;
  az_deg: number;
  el_deg: number;
  footprint_km: number;
  visible: boolean;
}

export async function getSat(): Promise<SatSnapshot> {
  return (await (await request('/sat')).json()) as SatSnapshot;
}

export async function putSat(settings: SatSettings): Promise<SatSnapshot> {
  return (await (await request('/sat', { ...jsonInit(settings), method: 'PUT' })).json()) as SatSnapshot;
}

export async function satRefreshTle(): Promise<number> {
  return ((await (await request('/sat/tle/refresh', { method: 'POST' })).json()) as { count: number }).count;
}

export async function getSatPasses(norad: number, hours: number, minEl: number): Promise<SatPass[]> {
  return ((await (await request(`/sat/passes?norad=${norad}&hours=${hours}&min_el=${minEl}`)).json()) as { passes: SatPass[] }).passes;
}

export async function getSatTrack(norad: number, minutes: number): Promise<[number, number][]> {
  return ((await (await request(`/sat/track?norad=${norad}&minutes=${minutes}`)).json()) as { points: [number, number][] }).points;
}

export async function getSatPositions(): Promise<SatPosition[]> {
  return ((await (await request('/sat/positions')).json()) as { positions: SatPosition[] }).positions;
}

/** L'indirizzo di una mappa del mondo (per <img>). */
export function satMapUrl(name: string): string {
  return `${BASE}/sat/maps/${encodeURIComponent(name)}`;
}

export async function satUploadMap(name: string, file: Blob): Promise<string[]> {
  return ((await (await request(`/sat/maps?name=${encodeURIComponent(name)}`, { method: 'POST', body: file })).json()) as { maps: string[] }).maps;
}
